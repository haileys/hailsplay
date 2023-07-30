use axum::Json;
use axum::extract::State;
use serde::Serialize;
use url::Url;

use crate::error::AppResult;
use crate::App;

#[derive(Serialize)]
pub struct Playlist {
    pub items: Vec<Url>,
}

pub async fn index(app: State<App>) -> AppResult<Json<Playlist>> {
    let mut mpd = app.mpd().await?;
    let playlist = mpd.playlistinfo().await?;
    Ok(Json(Playlist {
        items: playlist.items.into_iter().map(|item| item.file).collect(),
    }))
}
