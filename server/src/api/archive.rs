use std::{collections::HashMap, sync::{Arc, Mutex}, path::{PathBuf, Path}};

use derive_more::{Display, FromStr};
use mime::Mime;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;
use thiserror::Error;

use crate::{db::{self, archive::{ArchiveRecord, MetadataParseError}}, ytdlp::Metadata, config::Config};
use crate::fs::WorkingDirectory;
use crate::ytdlp;

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
    pub fn new(database: db::Pool, working: WorkingDirectory) -> Archive {
        let shared = Shared {
            database,
            working,
            locked: Mutex::default(),
        };

        Archive { shared: Arc::new(shared) }
    }

    pub async fn load(&self, id: MediaStreamId) -> Result<Option<RecordKind>, rusqlite::Error> {
        let record = self.shared.database.with(|conn| {
            db::archive::load_by_stream_uuid(conn, &id)
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

        let mut locked = self.shared.locked.lock().unwrap();
        locked.media_by_url.insert(url.clone(), id);
        locked.media.insert(id, record.clone());

        Ok(RecordKind::Memory(record))
    }
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
            RecordKind::Archive(record) => {
                record.filename.clone()
            }
            RecordKind::Memory(record) => {
                let filename = record.download.file.path().file_name()
                    .expect("download always has a file name");

                filename.to_string_lossy().to_string()
            }
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
            }
            RecordKind::Memory(record) => {
                Ok(record.metadata().clone())
            }
        }
    }
}

#[derive(Debug, Display, Deserialize, FromStr, Clone, Copy, Hash, PartialEq, Eq)]
#[display(fmt = "{}", "self.0")]
pub struct MediaStreamId(pub Uuid);

struct Shared {
    database: db::Pool,
    working: WorkingDirectory,
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
