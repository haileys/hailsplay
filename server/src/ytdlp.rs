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
use thiserror::Error;

use crate::fs::{SharedDir, SharedFile};

#[derive(Deserialize, Clone, Debug)]
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
    pub ext: String,
    pub audio_ext: String,
    pub video_ext: String,
}

#[derive(Debug, Error)]
pub enum FetchMetadataError {
    #[error("spawning yt-dlp command: {0}")]
    Spawn(std::io::Error),
    #[error("command failed: stderr output: {0}")]
    CommandError(String),
    #[error("parsing metadata: {0}")]
    ParseMetadata(serde_json::Error),
}

pub async fn fetch_metadata(url: &Url) -> Result<Metadata, FetchMetadataError> {
    const MAX_READ_SIZE: u64 = 512 * 1024; // 512 KiB

    let mut process = tokio::process::Command::new("yt-dlp")
        .arg("--dump-json")
        .arg(url.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(FetchMetadataError::Spawn)?;

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
        Ok(status) if status.success() => {}
        _ => {
            return Err(FetchMetadataError::CommandError(stderr));
        }
    }

    let metadata = serde_json::from_str::<Metadata>(&stdout)
        .map_err(FetchMetadataError::ParseMetadata)?;

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
    pub progress: watch::Receiver<Progress>,
    pub complete: oneshot::Receiver<Result<(), DownloadError>>,
}

#[derive(Clone)]
pub struct Progress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

impl Progress {
    pub fn complete(&self) -> bool {
        self.downloaded_bytes == self.total_bytes
    }
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("spawning yt-dlp command: {0}")]
    Spawn(std::io::Error),
    #[error("reading from yt-dlp: {0}")]
    Read(std::io::Error),
    #[error("reading from yt-dlp: {0}")]
    YtDlp(&'static str),
    #[error("command failed")]
    CommandError,
    #[error("reading metadata: {0}")]
    ReadMetadata(std::io::Error),
    #[error("parsing metadata: {0}")]
    ParseMetadata(serde_json::Error),
}

impl DownloadError {
    pub fn unexpected_eof() -> Self {
        DownloadError::Read(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "yt-dlp terminated abruptly",
        ))
    }
}

pub async fn start_download(dir: SharedDir, url: &Url) -> Result<DownloadHandle, DownloadError> {
    let mut process = Command::new("yt-dlp")
        .arg("--extract-audio")
        .arg("--audio-quality=0") // best
        .arg("--no-overwrites")
        .arg("--no-part")
        .arg("--write-info-json")
        .arg("--write-thumbnail")
        .arg("--newline") // output progress updates as newlines
        .arg("--progress-template=download:hailsplay-progress:D=%(progress.downloaded_bytes)s:T=%(progress.total_bytes)s")
        .arg(url.to_string())
        .stdout(Stdio::piped())
        .current_dir(dir.path())
        .kill_on_drop(true)
        .spawn()
        .map_err(DownloadError::Spawn)?;

    let stdout = process.stdout.take().unwrap();
    let mut ytdlp = YtdlpReader {
        process,
        reader: BufReader::new(stdout),
    };

    let mut file = None;
    let mut thumbnail = None;
    let mut metadata = None;
    let mut progress = None;

    loop {
        let line = match ytdlp.read_line().await? {
            Some(line) => line,
            None => { return Err(DownloadError::unexpected_eof()); }
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
            Line::Progress(p) => {
                log::debug!("yt-dlp reported total bytes: {}", p.total_bytes);
                progress = Some(p);
                break;
            }
            Line::Complete { .. } => {
                break;
            }
            Line::Other(_) => {}
        }
    }

    let Some(file) = file else {
        return Err(DownloadError::YtDlp("missing download filename in yt-dlp output"));
    };

    let Some(metadata_file) = metadata else {
        return Err(DownloadError::YtDlp("missing metadata filename in yt-dlp output"));
    };

    let Some(progress) = progress else {
        return Err(DownloadError::YtDlp("yt-dlp never started reporting progress"));
    };

    let metadata_json = tokio::fs::read_to_string(metadata_file.path()).await
        .map_err(DownloadError::ReadMetadata)?;

    let metadata = serde_json::from_str::<Metadata>(&metadata_json)
        .map_err(DownloadError::ParseMetadata)?;

    let (progress_tx, progress_rx) = watch::channel(progress.clone());
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

        let download_fut = run_download(ytdlp, progress_tx, progress.total_bytes);
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
    progress_tx: watch::Sender<Progress>,
    total_bytes: u64,
) -> Result<(), DownloadError> {
    loop {
        let Some(line) = ytdlp.read_line().await? else {
            return Err(DownloadError::unexpected_eof());
        };

        match line {
            Line::Progress(progress) => {
                let percent = (progress.downloaded_bytes as f64 / progress.total_bytes as f64) * 100.0;
                log::debug!("yt-dlp download progress {:.1}%", percent);
                let _ = progress_tx.send(progress);
            }
            Line::Complete => {
                log::debug!("yt-dlp download complete");
                let _ = progress_tx.send(Progress {
                    downloaded_bytes: total_bytes,
                    total_bytes,
                });
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

    let result = ytdlp.process.wait().await;

    match result {
        Ok(status) if status.success() => Ok(()),
        _ => Err(DownloadError::CommandError),
    }
}

struct YtdlpReader {
    process: Child,
    reader: BufReader<ChildStdout>,
}

impl YtdlpReader {
    pub async fn read_line(&mut self) -> Result<Option<Line>, DownloadError> {
        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) => Ok(None),
            Ok(_) => {
                let line = line.trim();
                Ok(Some(parse_line(line)))
            }
            Err(e) => {
                return Err(DownloadError::Read(e));
            }
        }
    }
}

enum Line {
    Thumbnail { filename: String },
    Metadata { filename: String },
    Download { filename: String },
    Progress(Progress),
    Complete,
    Other(String)
}

fn parse_line(line: &str) -> Line {
    lazy_static!{
        static ref THUMBNAIL: Regex = Regex::new(
            r"^\[info\] Writing video thumbnail original to: (.*)$").unwrap();

        static ref METADATA: Regex = Regex::new(
            r"^\[info\] Writing video metadata as JSON to: (.*)$").unwrap();

        static ref DOWNLOAD: Regex = Regex::new(
            r"^\[download\] Destination: (.*)$").unwrap();

        static ref PROGRESS: Regex = Regex::new(
            r"^hailsplay-progress:D=(\d+):T=(\d+)$").unwrap();

        static ref COMPLETE: Regex = Regex::new(
            r"^\[download\] 100%").unwrap();
    }

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
        let downloaded_bytes: u64 = m.get(1).unwrap().as_str().parse().unwrap();
        let total_bytes: u64 = m.get(2).unwrap().as_str().parse().unwrap();
        return Line::Progress(Progress {
            downloaded_bytes,
            total_bytes,
        });
    }

    if let Some(_) = COMPLETE.captures(line) {
        return Line::Complete;
    }

    return Line::Other(line.to_owned());
}
