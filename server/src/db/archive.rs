use chrono::{DateTime, Utc};
use derive_more::Display;
use rusqlite::{Connection, Row};
use url::Url;

use crate::api::archive::MediaStreamId;
use crate::ytdlp::Metadata;
use crate::db::asset::AssetId;

#[derive(Debug, Display, Clone, Copy)]
pub struct ArchiveRecordId(i64);

#[derive(Debug)]
pub struct ArchiveRecord {
    pub filename: String,
    pub canonical_url: Url,
    pub archived_at: DateTime<Utc>,
    pub stream_uuid: MediaStreamId,
    pub thumbnail_id: Option<AssetId>,
    pub metadata: serde_json::Value,
}

impl ArchiveRecord {
    pub fn parse_metadata(&self) -> Result<Metadata, serde_json::Error> {
        serde_json::value::from_value(self.metadata.clone())
    }
}

fn archive_record_from_row(row: &Row) -> Result<(ArchiveRecordId, ArchiveRecord), rusqlite::Error> {
    let id = ArchiveRecordId(row.get(0)?);
    let record = ArchiveRecord {
        filename: row.get(1)?,
        canonical_url: row.get(2)?,
        archived_at: row.get(3)?,
        stream_uuid: MediaStreamId(row.get(4)?),
        thumbnail_id: Option::map(row.get(5)?, AssetId),
        metadata: row.get(6)?,
    };
    Ok((id, record))
}

pub fn load_by_stream_uuid(conn: &mut Connection, id: &MediaStreamId)
    -> Result<(ArchiveRecordId, ArchiveRecord), rusqlite::Error>
{
    conn.prepare(r"
        SELECT id, path, canonical_url, archived_at, stream_uuid, thumbnail_id, metadata
        FROM archived_media
        WHERE stream_uuid = ?1
    ")?.query_row([&id.0], archive_record_from_row)
}

pub fn insert_media_record(conn: &mut Connection, record: ArchiveRecord)
    -> Result<ArchiveRecordId, rusqlite::Error>
{
    let params = (
        record.filename,
        record.canonical_url.to_string(),
        record.archived_at,
        record.stream_uuid.0,
        record.thumbnail_id.map(|AssetId(id)| id),
        record.metadata,
    );

    conn.prepare(r"
        INSERT INTO archived_media (path, canonical_url, archived_at, stream_uuid, thumbnail_id, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        RETURNING id
    ")?.query_row(params, |row| row.get(0).map(ArchiveRecordId))
}
