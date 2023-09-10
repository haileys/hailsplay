use derive_more::Display;
use diesel::{AsExpression, FromSqlRow, sql_types};
use diesel::prelude::*;
use rusqlite::{Connection, Row};

use crate::api::archive::MediaStreamId;
use crate::ytdlp::Metadata;
use crate::db::asset::AssetId;
use crate::db::schema::archived_media;
use crate::db::types;

#[derive(Debug, Display, Clone, Copy, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Integer)]
pub struct ArchiveRecordId(i64);

#[derive(Debug, Insertable)]
#[diesel(table_name = archived_media)]
pub struct ArchiveRecord {
    #[diesel(column_name = path)]
    pub filename: String,
    pub canonical_url: types::Url,
    pub archived_at: types::TimestampUtc,
    pub stream_uuid: MediaStreamId,
    pub thumbnail_id: Option<AssetId>,
    pub metadata: types::Json<Metadata>,
}

impl ArchiveRecord {
    pub fn parse_metadata(&self) -> Result<Metadata, serde_json::Error> {
        self.metadata.parse()
    }
}

fn archive_record_from_row(row: &Row) -> Result<(ArchiveRecordId, ArchiveRecord), rusqlite::Error> {
    let id = ArchiveRecordId(row.get(0)?);
    let record = ArchiveRecord {
        filename: row.get(1)?,
        canonical_url: types::Url(row.get(2)?),
        archived_at: types::TimestampUtc(row.get(3)?),
        stream_uuid: MediaStreamId(row.get(4)?),
        thumbnail_id: Option::map(row.get(5)?, AssetId),
        metadata: types::Json::from_string(row.get(6)?),
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
        record.canonical_url.0.to_string(),
        record.archived_at.0,
        record.stream_uuid.0,
        record.thumbnail_id.map(|AssetId(id)| id),
        record.metadata.into_string(),
    );

    conn.prepare(r"
        INSERT INTO archived_media (path, canonical_url, archived_at, stream_uuid, thumbnail_id, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        RETURNING id
    ")?.query_row(params, |row| row.get(0).map(ArchiveRecordId))
}
