use std::{collections::HashMap, sync::{Arc, Mutex}, path::{PathBuf, Path}};

use chrono::Utc;
use derive_more::{Display, FromStr};
use diesel::{FromSqlRow, AsExpression, sql_types, OptionalExtension};
use mime::Mime;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;
use thiserror::Error;

use crate::{api::asset, ytdlp::DownloadError, db::{archive::NewArchiveRecord, OptionalExt}};
use crate::config::Config;
use crate::db::{self, Pool};
use crate::db::archive::{self, ArchiveRecord, ArchiveRecordId};
use crate::fs::WorkingDirectory;
use crate::ytdlp::{self, Metadata};

#[derive(Clone)]
pub struct Archive {
    shared: Arc<Shared>,
}

#[derive(Error, Debug)]
pub enum AddUrlError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yt-dlp error: {0}")]
    YtDlp(ytdlp::DownloadError),
}

impl Archive {
    pub fn new(database: Pool, working: WorkingDirectory, http: reqwest::Client) -> Archive {
        let shared = Shared {
            database,
            working,
            http,
            locked: Mutex::default(),
        };

        Archive { shared: Arc::new(shared) }
    }

    pub async fn load(&self, id: MediaStreamId) -> Result<Option<RecordKind>, db::Error> {
        let record = self.shared.database.diesel(|conn| {
            archive::load_by_stream_uuid(conn, &id)
                .optional()
        }).await?;

        // database records always take precedence over in-process state
        if let Some(record) = record {
            return Ok(Some(RecordKind::Archive(record)));
        }

        // check locked state next
        let locked = self.shared.locked.lock().unwrap();
        if let Some(record) = locked.media.get(&id) {
            return Ok(Some(RecordKind::Memory(record.clone())));
        }

        // otherwise not found
        Ok(None)
    }

    pub async fn add_url(&self, url: &Url) -> Result<RecordKind, AddUrlError> {
        let id = MediaStreamId(uuid::Uuid::new_v4());
        let dir = self.shared.working.create_dir(&id.to_string()).await?;
        let dir = dir.into_shared();

        let download = ytdlp::start_download(dir, url).await
            .map_err(AddUrlError::YtDlp)?;

        let record = Arc::new(MemoryRecord {
            id,
            url: url.clone(),
            download: Arc::new(download),
        });

        tokio::task::spawn({
            let shared = self.shared.clone();
            let record = record.clone();
            async move {
                let result = archive_once_download_complete(shared, record).await;
                if let Err(e) = result {
                    log::error!("error archiving media, not saving: {e:?}");
                }
            }
        });

        let mut locked = self.shared.locked.lock().unwrap();
        locked.media_by_url.insert(url.clone(), id);
        locked.media.insert(id, record.clone());

        Ok(RecordKind::Memory(record))
    }
}

#[derive(Error, Debug)]
enum ArchiveError {
    #[error("media download failed: {0}")]
    DownloadFailed(DownloadError),
    #[error("media download task abruptly terminated")]
    DownloadTaskFailed,
    #[error("database error: {0}")]
    Database(#[from] db::Error),
}

async fn archive_once_download_complete(
    shared: Arc<Shared>,
    record: Arc<MemoryRecord>,
) -> Result<(), ArchiveError> {
    let complete = record.download.complete.clone();

    complete.await
        .map_err(|_| ArchiveError::DownloadTaskFailed)?
        .map_err(ArchiveError::DownloadFailed)?;

    let metadata = record.metadata();

    log::info!("finished downloading, now processing: {}", record.url);

    let canonical_url = metadata.webpage_url.as_ref()
        .unwrap_or(&record.url)
        .clone();

    let thumbnail = match &metadata.thumbnail {
        Some(thumbnail_url) => {
            log::info!("downloading thumbnail: {thumbnail_url}");
            match asset::download(shared.http.clone(), &thumbnail_url).await {
                Ok(asset) => Some(asset),
                Err(e) => {
                    log::warn!("failed to download thumbnail: {thumbnail_url}: {e:?}");
                    None
                }
            }
        }
        None => None,
    };

    shared.database.diesel(|conn| -> Result<_, ArchiveError> {
        let thumbnail_id = thumbnail
            .map(|thumbnail| thumbnail.insert(conn))
            .transpose()?;

        let record = NewArchiveRecord {
            filename: record.download.filename(),
            canonical_url: canonical_url.into(),
            archived_at: Utc::now().into(),
            stream_uuid: record.id,
            thumbnail_id,
            metadata: metadata.into(),
        };

        Ok(archive::insert_media_record(conn, record)?)
    }).await?;

    let mut locked = shared.locked.lock().unwrap();
    locked.media.remove(&record.id);
    locked.media_by_url.remove(&record.url);

    log::info!("successfully downloaded and archived url: {}", record.url);

    Ok(())
}

pub enum RecordKind {
    Memory(Arc<MemoryRecord>),
    Archive(ArchiveRecord),
}

impl RecordKind {
    pub fn content_type(&self) -> Mime {
        let path = match self {
            RecordKind::Archive(record) => Path::new(&record.filename),
            RecordKind::Memory(record) => record.download.file.path(),
        };

        crate::mime::from_path(path)
    }

    pub fn disk_path(&self, config: &Config) -> PathBuf {
        match self {
            RecordKind::Archive(record) => {
                config.storage.archive.join(&record.filename)
            }
            RecordKind::Memory(record) => {
                record.download.file.path().to_owned()
            }
        }
    }

    pub fn filename(&self) -> String {
        match self {
            RecordKind::Archive(record) => record.filename.clone(),
            RecordKind::Memory(record) => record.download.filename(),
        }
    }

    pub fn stream_id(&self) -> MediaStreamId {
        match self {
            RecordKind::Archive(record) => record.stream_uuid,
            RecordKind::Memory(record) => record.id,
        }
    }

    pub fn internal_stream_url(&self, config: &Config) -> Url {
        let path = format!("media/{id}/stream", id = self.stream_id());
        config.http.internal_url.join(&path).unwrap()
    }

    pub fn parse_metadata(&self) -> Result<Metadata, MetadataParseError> {
        match self {
            RecordKind::Archive(record) => {
                record.parse_metadata()
                    .map_err(|error| MetadataParseError { id: record.id, error })
            }
            RecordKind::Memory(record) => {
                Ok(record.metadata().clone())
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("failed to parse ytdlp metadata json for archived_media id={id}: {error}")]
pub struct MetadataParseError {
    pub id: ArchiveRecordId,
    #[source]
    pub error: serde_json::Error,
}

#[derive(Debug, Display, Deserialize, FromStr, Clone, Copy, Hash, PartialEq, Eq, FromSqlRow, AsExpression)]
#[display(fmt = "{}", "self.0")]
#[diesel(sql_type = sql_types::Text)]
pub struct MediaStreamId(pub Uuid);

struct Shared {
    database: Pool,
    working: WorkingDirectory,
    http: reqwest::Client,
    locked: Mutex<Locked>,
}

#[derive(Default)]
struct Locked {
    media_by_url: HashMap<Url, MediaStreamId>,
    media: HashMap<MediaStreamId, Arc<MemoryRecord>>,
}

#[derive(Clone)]
pub struct MemoryRecord {
    pub id: MediaStreamId,
    pub url: Url,
    pub download: Arc<ytdlp::DownloadHandle>,
}

impl MemoryRecord {
    pub fn metadata(&self) -> &ytdlp::Metadata {
        &self.download.metadata
    }
}
