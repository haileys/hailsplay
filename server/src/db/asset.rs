use bytes::Bytes;
use mime::Mime;
use rusqlite::{Connection, Row};

#[derive(Clone, Copy, Debug)]
pub struct AssetId(pub i64);

#[derive(Clone, Debug)]
pub struct AssetDigest(pub String);

pub struct Asset {
    pub filename: String,
    pub content_type: Mime,
    pub digest: AssetDigest,
}

pub fn create(conn: &mut Connection, filename: String, content_type: Mime, data: &[u8])
    -> Result<AssetId, rusqlite::Error>
{
    let txn = conn.transaction()?;

    let filename = filenamify::filenamify(filename);
    let content_type = content_type.to_string();
    let digest = create_blob(&txn, data)?;

    let id = txn.query_row(
        "INSERT INTO assets (filename, content_type, digest_sha256) VALUES (?1, ?2, ?3) RETURNING id",
        (filename, content_type, &digest.0),
        |row| Ok(AssetId(row.get(0)?)))?;

    txn.commit()?;

    Ok(id)
}

fn get_mime(row: &Row, idx: usize) -> Result<Mime, rusqlite::Error> {
    let mime: String = row.get(idx)?;
    mime.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub fn load_asset(conn: &Connection, id: AssetId)
    -> Result<Asset, rusqlite::Error>
{
    conn.query_row(
        "SELECT filename, content_type, digest_sha256 FROM assets WHERE id = ?1",
        [id.0],
        |row| Ok(Asset {
            filename: row.get(0)?,
            content_type: get_mime(row, 1)?,
            digest: AssetDigest(row.get(2)?),
        }),
    )
}

pub fn load_blob(conn: &Connection, digest: &AssetDigest)
    -> Result<Bytes, rusqlite::Error>
{
    conn.query_row(
        "SELECT blob FROM asset_blobs WHERE digest_sha256 = ?1",
        [&digest.0],
        |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(Bytes::from(data))
        })
}

fn create_blob(conn: &Connection, data: &[u8])
    -> Result<AssetDigest, rusqlite::Error>
{
    let digest = sha256::digest(data);

    conn.execute(
        "INSERT OR IGNORE INTO asset_blobs (digest_sha256, blob) VALUES (?1, ?2)",
        (&digest, data))?;

    Ok(AssetDigest(digest))
}
