use rusqlite::Connection;
use url::Url;

use crate::db::asset::AssetId;

pub struct StationId(i64);

pub struct Station {
    pub name: String,
    pub icon: AssetId,
    pub stream_url: Url,
}

pub fn insert_station(conn: &mut Connection, station: Station) -> Result<StationId, rusqlite::Error> {
    conn.query_row(
        "INSERT INTO radio_stations (name, icon_id, stream_url) VALUES (?1, ?2, ?3) RETURNING id",
        (&station.name, station.icon.0, &station.stream_url),
        |row| row.get(0).map(StationId))
}

pub fn all_stations(conn: &mut Connection) -> Result<Vec<Station>, rusqlite::Error> {
    conn.prepare("SELECT name, icon_id, stream_url FROM radio_stations ORDER BY id ASC")?
        .query_map([], |row| Ok(Station {
            name: row.get(0)?,
            icon: AssetId(row.get(1)?),
            stream_url: row.get(2)?,
        }))?
        .collect()
}
