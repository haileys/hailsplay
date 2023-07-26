use std::f64::consts::E;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use regex::Regex;
use lazy_static::lazy_static;
use tokio::process::Command;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader, AsyncBufReadExt};
use tokio::sync::{oneshot, watch};
use url::Url;
use serde::Deserialize;
use url::form_urlencoded::parse;

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
    pub filename: PathBuf,
    pub thumbnail: Option<PathBuf>,
    pub metadata: Metadata,
    pub progress: watch::Receiver<f64>,
    pub complete: oneshot::Receiver<Result<(), anyhow::Error>>,
}

pub async fn start_download(dir: &Path, url: &Url) -> anyhow::Result<DownloadHandle> {
    let (tx, rx) = oneshot::channel();

    tokio::task::spawn({
        let dir = dir.to_owned();
        let url = url.to_owned();
        run_download(dir, url, tx)
    });

    rx.await;
    todo!();
}

async fn run_download(
    dir: PathBuf,
    url: Url,
    handle_tx: oneshot::Sender<anyhow::Result<DownloadHandle>>,
) {
    let result = Command::new("ytdlp")
        .arg("--no-overwrites")
        .arg("--no-part")
        .arg("--write-info-json")
        .arg("--write-thumbnail")
        .arg("--newline") // output progress updates as newlines
        .arg(url.to_string())
        .stdout(Stdio::piped())
        .current_dir(dir)
        .kill_on_drop(true)
        .spawn();

    let process = match result {
        Ok(process) => process,
        Err(e) => {
            let _ = handle_tx.send(Err(e.into()));
            return;
        }
    };

    let stdout = process.stdout.take().unwrap();
    let stdout = BufReader::new(stdout);

    let mut thumbnail = None;
    let mut metadata = None;
    let mut filename = None;
    let (progress_tx, progress_rx) = watch::channel(0.0);
    let (complete_tx, complete_rx) = oneshot::channel();

    let mut line = String::new();
    loop {
        // read line from ytdlp
        line.truncate(0);
        match stdout.read_line(&mut line).await {
            Ok(_) => {},
            Err(e) => {
                log::error!("read_line error: {e:?}");
                return;
            }
        }

        let line = parse_line(&line);

        match line {
            Status::Thumbnail { filename: f } => {
                thumbnail = Some(f.to_owned());
            }
            Status::Metadata { filename: f } => {
                metadata = Some(f.to_owned());
            }
            Status::Download { filename: f } => {
                filename = Some(f.to_owned());
            }
            Status::Progress { progress } => {
                match progress_tx.send(progress) {
                    Ok(()) => {}
                    Err(_) => {
                        // receiver disconnected, cleanup
                    }
                }
            }
        }
    }
}

enum Status<'a> {
    Thumbnail { filename: &'a str },
    Metadata { filename: &'a str },
    Download { filename: &'a str },
    Progress { progress: f64 },
    Complete,
    Other(&'a str)
}

fn parse_line<'a>(line: &'a str) -> Status<'a> {
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

    if let Some(m) = THUMBNAIL.captures(line) {
        return Status::Thumbnail { filename: m.get(1).unwrap().as_str() };
    }

    if let Some(m) = METADATA.captures(line) {
        return Status::Metadata { filename: m.get(1).unwrap().as_str() };
    }

    if let Some(m) = DOWNLOAD.captures(line) {
        return Status::Download { filename: m.get(1).unwrap().as_str() };
    }

    if let Some(m) = PROGRESS.captures(line) {
        let percent: f64 = m.get(1).unwrap().as_str().parse().unwrap();
        return Status::Progress { progress: percent / 100.0 };
    }

    if let Some(m) = COMPLETE.captures(line) {
        return Status::Complete;
    }

    return Status::Other(s);
}

// struct StatusLine<'a> {
//     pub tag: &'a str,
//     pub line: &'a str,
// }

// impl FromStr for StatusLine<'_> {
//     type Err = anyhow::Error;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
        
//         let captures = RE.captures(s)
//             .ok_or_else(|| {
//                 anyhow::bail!("yt-dlp line did not match expected format: {s}")
//             })?;

//         let tag = captures.get(1).unwrap().as_str();
//         let line = captures.get(2).unwrap().as_str();

//         Ok(StatusLine { tag, line })
//     }
// }
