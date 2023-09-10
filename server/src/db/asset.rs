use bytes::Bytes;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::{AsExpression, FromSqlRow};
use mime::Mime;

use crate::db::{self, Error};
use crate::db::types;
use crate::db::schema::{assets, asset_blobs};

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::BigInt)]
pub struct AssetId(pub i64);

#[derive(Clone, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct AssetDigest(pub String);

#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = assets)]
pub struct Asset {
    pub filename: String,
    pub content_type: types::Mime,
    #[diesel(column_name = digest_sha256)]
    pub digest: AssetDigest,
}

impl Asset {
    pub fn content_type(&self) -> Mime {
        self.content_type.0
    }
}

pub fn create(conn: &mut db::Connection, filename: String, content_type: Mime, data: Vec<u8>)
    -> Result<AssetId, Error>
{
    conn.transaction(|conn| {
        let filename = filenamify::filenamify(filename);
        let content_type = content_type.into();
        let digest = create_blob(conn, data)?;

        let asset = Asset { filename, content_type, digest };

        Ok(diesel::insert_into(assets::table)
            .values(&asset)
            .returning(assets::id)
            .get_result(conn)?)
    })
}

pub fn load_asset(conn: &mut db::Connection, id: AssetId)
    -> Result<Asset, db::Error>
{
    Ok(assets::table
        .filter(assets::id.eq(id))
        .get_result(conn)?)
}

pub fn load_blob(conn: &mut db::Connection, digest: &AssetDigest)
    -> Result<Bytes, db::Error>
{
    Ok(asset_blobs::table
        .filter(asset_blobs::digest_sha256.eq(digest))
        .select(asset_blobs::blob)
        .get_result(conn)?)
}

#[derive(Insertable)]
#[diesel(table_name = asset_blobs)]
struct NewBlob {
    #[diesel(column_name = digest_sha256)]
    digest: AssetDigest,
    blob: Vec<u8>,
}

fn create_blob(conn: &mut db::Connection, blob: Vec<u8>)
    -> Result<AssetDigest, rusqlite::Error>
{
    let digest = AssetDigest(sha256::digest(&blob));

    let row = NewBlob {
        digest: digest.clone(),
        blob,
    };

    diesel::insert_or_ignore_into(asset_blobs::table)
        .values(&row)
        .execute(conn)?;

    Ok(digest)
}
