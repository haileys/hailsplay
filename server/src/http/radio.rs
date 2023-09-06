use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::AppResult;
use crate::App;

#[derive(Deserialize)]
pub struct TuneParams {
    pub url: Url,
}

#[derive(Serialize)]
pub struct TuneResult;

pub async fn tune(app: State<App>, params: Json<TuneParams>) -> AppResult<Json<TuneResult>> {
    let mut mpd = app.mpd().await?;

    // add streaming url and then immediately play it by id
    let id = mpd.addid(&params.url).await?;
    mpd.stop().await?;
    mpd.playid(id).await?;

    Ok(Json(TuneResult))
}
