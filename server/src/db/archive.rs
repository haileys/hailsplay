use derive_more::Display;
use diesel::sqlite::Sqlite;
use diesel::{AsExpression, FromSqlRow, sql_types};
use diesel::prelude::*;
use url::Url;

use crate::api::archive::MediaStreamId;
use crate::ytdlp::Metadata;
use crate::db::asset::AssetId;
use crate::db::schema::archived_media;
use crate::db::types;
use crate::db::{Connection, Error};

#[derive(Debug, Display, Clone, Copy, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Integer)]
pub struct ArchiveRecordId(#[diesel(deserialize_as = i32)] i32);

#[derive(Debug, QueryableByName, Selectable)]
#[diesel(table_name = archived_media, check_for_backend(Sqlite))]
pub struct ArchiveRecord {
    // #[diesel(deserialize_as = i32)]
    pub id: ArchiveRecordId,
    #[diesel(column_name = path)]
    pub filename: String,
    #[diesel(deserialize_as = String)]
    pub canonical_url: Url,
    pub archived_at: types::TimestampUtc,
    pub stream_uuid: MediaStreamId,
    pub thumbnail_id: Option<AssetId>,
    pub metadata: types::Json<Metadata>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = archived_media, check_for_backend(Sqlite))]
pub struct NewArchiveRecord {
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

pub fn load_by_stream_uuid(conn: &mut Connection, id: &MediaStreamId)
    -> Result<ArchiveRecord, Error>
{
    let row = archived_media::table
        .filter(archived_media::stream_uuid.eq(id))
        .select(ArchiveRecord::as_select())
        .get_results(conn)?;

    Ok((row.id, row.attrs))
}

pub fn insert_media_record(conn: &mut Connection, record: NewArchiveRecord)
    -> Result<ArchiveRecordId, Error>
{
    Ok(diesel::insert_into(archived_media::table)
        .values(&record)
        .returning(archived_media::id)
        .get_result(conn)?)
}
