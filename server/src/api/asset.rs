use std::io;
use std::path::Path;

use headers::{HeaderMapExt, ContentType};
use mime::Mime;
use thiserror::Error;
use url::Url;

use crate::db::{self, Connection};
use crate::db::asset::{self, AssetId};

pub struct UploadableAsset {
    filename: String,
    content_type: Mime,
    data: Vec<u8>,
}

pub async fn upload(path: &Path) -> io::Result<UploadableAsset> {
    let filename = path.file_name()
        .ok_or(io::Error::new(
            // TODO - when io_error_more feature stabilises,
            // change this to InvalidFilename:
            io::ErrorKind::InvalidInput,
            "path has no filename",
        ))?
        .to_string_lossy()
        .to_string();

    let content_type = crate::mime::from_path(path);

    let data = tokio::fs::read(path).await?;

    Ok(UploadableAsset { filename, content_type, data })
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("http download error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("file too large to download, max {max} bytes")]
    FileTooLarge { max: usize },
}

impl DownloadError {
    pub const TOO_LARGE: Self = DownloadError::FileTooLarge { max: MAX_ASSET_SIZE };
}

const MAX_ASSET_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

pub async fn download(http: reqwest::Client, url: &Url) -> Result<UploadableAsset, DownloadError> {
    let mut response = http.get(url.to_string())
        .send()
        .await?;

    let mut data = Vec::new();

    if let Some(size) = response.content_length() {
        let size = usize::try_from(size)
            .map_err(|_| DownloadError::TOO_LARGE)?;

        if size > MAX_ASSET_SIZE {
            return Err(DownloadError::TOO_LARGE);
        }

        data.reserve_exact(size);
    }

    while let Some(chunk) = response.chunk().await? {
        let remaining = MAX_ASSET_SIZE - data.len();

        if chunk.len() > remaining {
            return Err(DownloadError::TOO_LARGE);
        }

        data.extend(&chunk);
    }

    let filename = url.path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or_default()
        .to_owned();

    let content_type = response.headers()
        .typed_get::<ContentType>()
        .map(|content_type| Mime::from(content_type))
        .unwrap_or_else(|| crate::mime::from_path(Path::new(&filename)).into());

    Ok(UploadableAsset { filename, content_type, data })
}

impl UploadableAsset {
    pub fn insert(self, conn: &mut Connection) -> Result<AssetId, db::Error> {
        asset::create(conn, self.filename, self.content_type, self.data)
    }
}
