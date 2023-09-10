use chrono::{DateTime, Utc};
use derive_more::Display;
use rusqlite::{Connection, Row};
use url::Url;

use crate::{api::archive::MediaStreamId, ytdlp::Metadata};

#[derive(Debug, Display, Clone, Copy)]
pub struct ArchiveRecordId(i64);

#[derive(Debug)]
pub struct ArchiveRecord {
    pub id: ArchiveRecordId,
    pub filename: String,
    pub canonical_url: Url,
    pub archived_at: DateTime<Utc>,
    pub stream_uuid: MediaStreamId,
    pub thumbnail_id: i64,
    pub metadata: serde_json::Value,
}

#[derive(thiserror::Error, Debug)]
#[error("failed to parse ytdlp metadata json for archived_media id={id}: {error}")]
pub struct MetadataParseError {
    pub id: ArchiveRecordId,
    #[source]
    pub error: serde_json::Error,
}

impl ArchiveRecord {
    pub fn parse_metadata(&self) -> Result<Metadata, MetadataParseError> {
        serde_json::value::from_value(self.metadata.clone())
            .map_err(|e| MetadataParseError { id: self.id, error: e })
    }
}

fn archive_record_from_row(row: &Row) -> Result<ArchiveRecord, rusqlite::Error> {
    Ok(ArchiveRecord {
        id: ArchiveRecordId(row.get(0)?),
        filename: row.get(1)?,
        canonical_url: row.get(2)?,
        archived_at: row.get(3)?,
        stream_uuid: MediaStreamId(row.get(4)?),
        thumbnail_id: row.get(5)?,
        metadata: row.get(6)?,
    })
}

pub fn load_by_stream_uuid(conn: &mut Connection, id: &MediaStreamId)
    -> Result<ArchiveRecord, rusqlite::Error>
{
    conn.prepare(r"
        SELECT id, path, canonical_url, archived_at, stream_uuid, thumbnail_id, metadata
        FROM archived_media
        WHERE stream_uuid = ?1
    ")?.query_row([&id.0], archive_record_from_row)
}
