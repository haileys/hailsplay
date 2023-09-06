use bytes::Bytes;
use rusqlite::Connection;

#[derive(Clone, Copy, Debug)]
pub struct AssetId(pub i64);

#[derive(Clone, Debug)]
pub struct AssetDigest(pub String);

pub struct Asset {
    pub filename: String,
    pub content_type: String,
    pub digest: AssetDigest,
}

pub fn create(conn: &mut Connection, filename: String, content_type: String, data: &[u8])
    -> Result<AssetId, rusqlite::Error>
{
    let filename = filenamify::filenamify(filename);
    let digest = create_blob(conn, data)?;

    conn.query_row(
        "INSERT INTO assets (filename, content_type, digest_sha256) VALUES (?1, ?2, ?3) RETURNING id",
        (filename, content_type, &digest.0),
        |row| Ok(AssetId(row.get(0)?)))
}

pub fn load_asset(conn: &mut Connection, id: AssetId)
    -> Result<Asset, rusqlite::Error>
{
    conn.query_row(
        "SELECT filename, content_type, digest_sha256 FROM assets WHERE id = ?1",
        [id.0],
        |row| Ok(Asset {
            filename: row.get(0)?,
            content_type: row.get(1)?,
            digest: AssetDigest(row.get(2)?),
        }),
    )
}

pub fn load_blob(conn: &mut Connection, digest: &AssetDigest)
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

fn create_blob(conn: &mut Connection, data: &[u8])
    -> Result<AssetDigest, rusqlite::Error>
{
    let digest = sha256::digest(data);

    conn.execute(
        "INSERT OR IGNORE INTO asset_blobs (digest_sha256, blob) VALUES (?1, ?2)",
        (&digest, data))?;

    Ok(AssetDigest(digest))
}
