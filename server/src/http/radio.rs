use axum::{Json, extract::State};

use hailsplay_protocol::{TuneParams, RadioStation};

use crate::db::radio;
use crate::error::AppResult;
use crate::http;
use crate::App;

pub async fn tune(app: State<App>, params: Json<TuneParams>) -> AppResult<Json<()>> {
    let mut mpd = app.mpd().await?;

    // add streaming url and then immediately play it by id
    let id = mpd.addid(&params.url).await?;
    mpd.stop().await?;
    mpd.playid(id).await?;

    Ok(Json(()))
}

pub async fn stations(app: State<App>) -> AppResult<Json<Vec<RadioStation>>> {
    let stations = app.use_database(|conn| {
        let stations = radio::all_stations(conn)?;

        stations.into_iter()
            .map(|station| Ok(RadioStation {
                name: station.name,
                icon_url: http::assets::url(conn, app.config(), station.icon)?,
                stream_url: station.stream_url,
            }))
            .collect::<Result<Vec<_>, rusqlite::Error>>()
    }).await?;

    Ok(Json(stations))
}
