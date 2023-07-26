use std::path::Path;
use std::process::Stdio;
use std::task::Poll;
use futures::{FutureExt, future};
use regex::Regex;
use lazy_static::lazy_static;
use tokio::process::{Command, Child, ChildStdout};
use tokio::io::{AsyncRead, AsyncReadExt, BufReader, AsyncBufReadExt};
use tokio::sync::{oneshot, watch};
use url::Url;
use serde::Deserialize;

use crate::fs::{SharedDir, SharedFile};

#[derive(Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    #[serde(rename = "fulltitle")]
    pub full_title: Option<String>,
    pub uploader: Option<String>,
    pub uploader_url: Option<Url>,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub webpage_url: Option<Url>,
    pub genre: Option<String>,
    pub thumbnail: Option<Url>,
}

pub async fn fetch_metadata(url: &Url) -> anyhow::Result<Metadata> {
    const MAX_READ_SIZE: u64 = 512 * 1024; // 512 KiB

    let mut process = tokio::process::Command::new("yt-dlp")
        .arg("--dump-json")
        .arg(url.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = process.stdout.take().unwrap();
    let stderr = process.stderr.take().unwrap();

    let stdout = stdout.take(MAX_READ_SIZE);
    let stderr = stderr.take(MAX_READ_SIZE);

    let result = futures::future::join3(
        process.wait(),
        read_to_string_logging_errors("stdout", stdout),
        read_to_string_logging_errors("stderr", stderr),
    );

    let (status, stdout, stderr) = result.await;

    match status {
        Ok(status) => {
            if !status.success() {
                if let Some(code) = status.code() {
                    anyhow::bail!("yt-dlp returned non-zero exit status: {code:?}\n\nstderr: {stderr}")
                } else {
                    anyhow::bail!("yt-dlp exited with failure status\n\nstderr: {stderr}")
                }
            }
        }
        Err(e) => {
            anyhow::bail!("error invoking yt-dlp: {e:?}\n\nstderr: {stderr}")
        }
    }

    let metadata = serde_json::from_str::<Metadata>(&stdout)?;

    Ok(metadata)
}

async fn read_to_string_logging_errors(stream: &str, mut read: impl AsyncRead + Unpin) -> String {
    let mut out = String::new();
    if let Err(e) = read.read_to_string(&mut out).await {
        eprintln!("error reading {stream} of subprocess: {e:?}");
    }
    out
}

pub struct DownloadHandle {
    pub file: SharedFile,
    pub thumbnail: Option<SharedFile>,
    pub metadata: Metadata,
    pub metadata_file: SharedFile,
    pub progress: watch::Receiver<f64>,
    pub complete: oneshot::Receiver<Result<(), anyhow::Error>>,
}

pub async fn start_download(dir: SharedDir, url: &Url) -> anyhow::Result<DownloadHandle> {
    let mut process = Command::new("yt-dlp")
        .arg("--no-overwrites")
        .arg("--no-part")
        .arg("--write-info-json")
        .arg("--write-thumbnail")
        .arg("--newline") // output progress updates as newlines
        .arg(url.to_string())
        .stdout(Stdio::piped())
        .current_dir(dir.path())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = process.stdout.take().unwrap();
    let mut ytdlp = YtdlpReader {
        process,
        reader: BufReader::new(stdout),
    };

    let mut file = None;
    let mut thumbnail = None;
    let mut metadata = None;

    loop {
        let line = match ytdlp.read_line().await? {
            Some(line) => line,
            None => { anyhow::bail!("unexpected yt-dlp EOF"); }
        };

        match line {
            Line::Thumbnail { filename: f } => {
                log::debug!("yt-dlp reported thumbnail filename: {f}");
                thumbnail = Some(dir.claim_external_file(Path::new(&f)));
            }
            Line::Metadata { filename: f } => {
                log::debug!("yt-dlp reported metadata filename: {f}");
                metadata = Some(dir.claim_external_file(Path::new(&f)));
            }
            Line::Download { filename: f } => {
                log::debug!("yt-dlp reported download filename: {f}");
                file = Some(dir.claim_external_file(Path::new(&f)));
            }
            | Line::Progress { .. }
            | Line::Complete { .. } => {
                break;
            }
            Line::Other(_) => {}
        }
    }

    let Some(file) = file else {
        anyhow::bail!("missing download filename in yt-dlp output");
    };

    let Some(metadata_file) = metadata else {
        anyhow::bail!("missing metadata filename in yt-dlp output");
    };

    let metadata_json = tokio::fs::read_to_string(metadata_file.path()).await?;
    let metadata = serde_json::from_str::<Metadata>(&metadata_json)?;

    let (progress_tx, progress_rx) = watch::channel(0.0);
    let (complete_tx, complete_rx) = oneshot::channel();

    let handle = DownloadHandle { 
        file: file.into_shared(),
        thumbnail: thumbnail.map(|th| th.into_shared()),
        metadata: metadata,
        metadata_file: metadata_file.into_shared(),
        progress: progress_rx,
        complete: complete_rx,
    };

    tokio::task::spawn(async move {
        let mut complete_tx = Some(complete_tx);

        let download_fut = run_download(ytdlp, progress_tx);
        futures::pin_mut!(download_fut);

        future::poll_fn(|cx| {
            if let Poll::Ready(()) = complete_tx.as_mut().unwrap().poll_closed(cx) {
                // complete_rx has hung up on us, end our task
                return Poll::Ready(());
            }

            if let Poll::Ready(result) = download_fut.poll_unpin(cx) {
                let complete_tx = complete_tx.take().unwrap();
                let _ = complete_tx.send(result);
                return Poll::Ready(());
            }

            Poll::Pending
        }).await
    });

    Ok(handle)
}

async fn run_download(
    mut ytdlp: YtdlpReader,
    progress_tx: watch::Sender<f64>
) -> anyhow::Result<()> {
    // watch progress
    loop {
        let Some(line) = ytdlp.read_line().await? else {
            anyhow::bail!("unexpected yt-dlp EOF");
        };

        match line {
            Line::Progress { progress } => {
                log::debug!("yt-dlp download progress {:.1}%", progress * 100.0);
                let _ = progress_tx.send(progress);
            }
            Line::Complete => {
                log::debug!("yt-dlp download finished");
                let _ = progress_tx.send(1.0);
                break;
            }
            | Line::Download { .. }
            | Line::Thumbnail { .. }
            | Line::Metadata { .. }
            | Line::Other { .. } => {}
        }
    }

    // read any remaining lines
    while let Some(_) = ytdlp.read_line().await? {
        // pass
    }

    let status = ytdlp.process.wait().await?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("yt-dlp returned failure status");
    }
}

struct YtdlpReader { 
    process: Child,
    reader: BufReader<ChildStdout>,
}

impl YtdlpReader {
    pub async fn read_line(&mut self) -> Result<Option<Line>, tokio::io::Error> {
        let mut line = String::new();
        match self.reader.read_line(&mut line).await? {
            0 => Ok(None),
            _ => Ok(Some(parse_line(line))),
        }
    }
}

enum Line {
    Thumbnail { filename: String },
    Metadata { filename: String },
    Download { filename: String },
    Progress { progress: f64 },
    Complete,
    Other(String)
}

fn parse_line(line: String) -> Line {
    lazy_static!{
        static ref THUMBNAIL: Regex = Regex::new(
            r"^\[info\] Writing video thumbnail original to: (.*)$").unwrap();

        static ref METADATA: Regex = Regex::new(
            r"^\[info\] Writing video metadata as JSON to: (.*)$").unwrap();

        static ref DOWNLOAD: Regex = Regex::new(
            r"^\[download\] Destination: (.*)$").unwrap();

        static ref PROGRESS: Regex = Regex::new(
            r"^\[download\]\s+([0-9]+\.[0-9]+)%").unwrap();
        
        static ref COMPLETE: Regex = Regex::new(
            r"^\[download\] 100%").unwrap();
    }

    let line = line.trim();

    if let Some(m) = THUMBNAIL.captures(line) {
        return Line::Thumbnail { filename: m.get(1).unwrap().as_str().to_owned() };
    }

    if let Some(m) = METADATA.captures(line) {
        return Line::Metadata { filename: m.get(1).unwrap().as_str().to_owned() };
    }

    if let Some(m) = DOWNLOAD.captures(line) {
        return Line::Download { filename: m.get(1).unwrap().as_str().to_owned() };
    }

    if let Some(m) = PROGRESS.captures(line) {
        let percent: f64 = m.get(1).unwrap().as_str().parse().unwrap();
        return Line::Progress { progress: percent / 100.0 };
    }

    if let Some(_) = COMPLETE.captures(line) {
        return Line::Complete;
    }

    return Line::Other(line.to_owned());
}
