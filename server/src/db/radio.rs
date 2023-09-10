use diesel::prelude::*;
use diesel::{FromSqlRow, AsExpression, sql_types};

use crate::db::{Connection, Error};
use crate::db::asset::AssetId;
use crate::db::types;
use crate::db::schema::radio_stations;

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Integer)]
pub struct StationId(i64);

#[derive(Queryable, Selectable)]
#[diesel(table_name = radio_stations)]
pub struct Station {
    pub id: StationId,
    pub name: String,
    pub icon_id: AssetId,
    pub stream_url: types::Url,
}

#[derive(Insertable)]
#[diesel(table_name = radio_stations)]
pub struct NewStation {
    pub name: String,
    pub icon_id: AssetId,
    pub stream_url: types::Url,
}

pub fn insert_station(conn: &mut Connection, station: NewStation) -> Result<StationId, Error> {
    Ok(diesel::insert_into(radio_stations::table)
        .values(&station)
        .returning(radio_stations::id)
        .get_result(conn)?)
}

pub fn all_stations(conn: &mut Connection) -> Result<Vec<Station>, Error> {
    Ok(radio_stations::table
        .order(radio_stations::id)
        .get_results(conn)?)
}

pub fn find_by_url(conn: &mut Connection, url: &str) -> Result<Option<Station>, Error> {
    Ok(radio_stations::table
        .filter(radio_stations::stream_url.eq(url))
        .get_result(conn)
        .optional()?)
}
