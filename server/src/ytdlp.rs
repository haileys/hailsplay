use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};
use url::Url;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metadata {
    pub title: String,
    pub uploader: String,
    #[serde(default)]
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Deserialize)]
pub struct Thumbnail {
    pub url: Url,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

pub async fn metadata(url: Url) -> anyhow::Result<Metadata> {
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
