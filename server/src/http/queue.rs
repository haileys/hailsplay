use anyhow::Result;
use axum::{Json, debug_handler};
use axum::extract::{Path, State};

use hailsplay_protocol::{TrackId, TrackInfo, Queue, AddResponse, AddParams};
use reqwest::StatusCode;

use crate::error::AppResult;
use crate::mpd::{self, Mpd};
use crate::api;
use crate::App;

#[debug_handler]
pub async fn index(app: State<App>) -> AppResult<Json<Queue>> {
    let mut session = app.session().await?;
    Ok(Json(api::queue(&mut session).await?))
}

pub async fn show(app: State<App>, Path(track_id): Path<TrackId>)
    -> AppResult<Result<Json<TrackInfo>, StatusCode>>
{
    let mut session = app.session().await?;
    let track = api::track(&mut session, &track_id).await?;

    match track {
        None => Ok(Err(StatusCode::NOT_FOUND)),
        Some(track) => Ok(Ok(Json(track.load_info(&mut session).await?))),
    }
}

#[axum::debug_handler]
pub async fn add(app: State<App>, data: Json<AddParams>) -> AppResult<Json<AddResponse>> {
    let record = app.archive().add_url(&data.url).await?;

    let metadata = record.parse_metadata()?;
    let stream_url = record.internal_stream_url(app.config());

    log::info!("Adding {}", metadata.title
        .unwrap_or_else(|| data.url.to_string()));

    let mut mpd = app.mpd().await?;
    let mpd_id = mpd.addid(&stream_url).await?;

    if should_autoplay(&mut mpd, &mpd_id).await? {
        mpd.play().await?;
    }

    Ok(Json(AddResponse { mpd_id: mpd_id.into() }))
}

async fn should_autoplay(mpd: &mut Mpd, added_id: &mpd::Id) -> Result<bool> {
    let playlist = mpd.playlistinfo().await?;

    if playlist.items.len() != 1 {
        return Ok(false);
    }

    Ok(playlist.items[0].id == *added_id)
}
