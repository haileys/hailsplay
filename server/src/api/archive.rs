use std::{collections::HashMap, sync::{Arc, Mutex}, path::{PathBuf, Path}};

use chrono::Utc;
use derive_more::{Display, FromStr};
use mime::Mime;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;
use thiserror::Error;

use crate::api::asset;
use crate::config::Config;
use crate::db::Pool;
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

    pub async fn load(&self, id: MediaStreamId) -> Result<Option<RecordKind>, rusqlite::Error> {
        let record = self.shared.database.with(|conn| {
            archive::load_by_stream_uuid(conn, &id)
                .optional()
        }).await?;

        // database records always take precedence over in-process state
        if let Some((id, record)) = record {
            return Ok(Some(RecordKind::Archive(id, record)));
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

        tokio::task::spawn(
            wait_for_download_complete(
                self.shared.clone(),
                record.clone(),
            )
        );

        let mut locked = self.shared.locked.lock().unwrap();
        locked.media_by_url.insert(url.clone(), id);
        locked.media.insert(id, record.clone());

        Ok(RecordKind::Memory(record))
    }
}

async fn wait_for_download_complete(shared: Arc<Shared>, record: Arc<MemoryRecord>) {
    let complete = record.download.complete.clone();

    match complete.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            log::error!("media download failed: {e}");
            return;
        }
        Err(_) => {
            log::error!("media download task abruptly terminated");
            return;
        }
    }

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

    let metadata_value = match serde_json::to_value(metadata) {
        Ok(value) => value,
        Err(e) => {
            log::error!("failed to serialize metadata for database insert: {e:?}");
            return;
        }
    };

    let result = shared.database.with(|conn| {
        let thumbnail_id = thumbnail
            .map(|thumbnail| thumbnail.insert(conn))
            .transpose()?;

        let record = ArchiveRecord {
            filename: record.download.filename(),
            canonical_url,
            archived_at: Utc::now(),
            stream_uuid: record.id,
            thumbnail_id,
            metadata: metadata_value,
        };

        archive::insert_media_record(conn, record)
    }).await;

    if let Err(e) = result {
        log::error!("failed to insert archived media record: {e:?}");
        return;
    }

    let mut locked = shared.locked.lock().unwrap();
    locked.media.remove(&record.id);
    locked.media_by_url.remove(&record.url);

    log::info!("successfully downloaded and archived url: {}", record.url);
}

pub enum RecordKind {
    Memory(Arc<MemoryRecord>),
    Archive(ArchiveRecordId, ArchiveRecord),
}

impl RecordKind {
    pub fn content_type(&self) -> Mime {
        let path = match self {
            RecordKind::Archive(_, record) => Path::new(&record.filename),
            RecordKind::Memory(record) => record.download.file.path(),
        };

        crate::mime::from_path(path)
    }

    pub fn disk_path(&self, config: &Config) -> PathBuf {
        match self {
            RecordKind::Archive(_, record) => {
                config.storage.archive.join(&record.filename)
            }
            RecordKind::Memory(record) => {
                record.download.file.path().to_owned()
            }
        }
    }

    pub fn filename(&self) -> String {
        match self {
            RecordKind::Archive(_, record) => record.filename.clone(),
            RecordKind::Memory(record) => record.download.filename(),
        }
    }

    pub fn stream_id(&self) -> MediaStreamId {
        match self {
            RecordKind::Archive(_, record) => record.stream_uuid,
            RecordKind::Memory(record) => record.id,
        }
    }

    pub fn internal_stream_url(&self, config: &Config) -> Url {
        let path = format!("media/{id}/stream", id = self.stream_id());
        config.http.internal_url.join(&path).unwrap()
    }

    pub fn parse_metadata(&self) -> Result<Metadata, MetadataParseError> {
        match self {
            RecordKind::Archive(id, record) => {
                record.parse_metadata()
                    .map_err(|error| MetadataParseError { id: *id, error })
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

#[derive(Debug, Display, Deserialize, FromStr, Clone, Copy, Hash, PartialEq, Eq)]
#[display(fmt = "{}", "self.0")]
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
