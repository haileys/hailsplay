use std::sync::Arc;

use anyhow::Result;
use axum::Json;
use axum::extract::{Path, State};

use hailsplay_protocol as proto;

use crate::error::AppResult;
use crate::mpd::{self, Mpd};
use crate::player::{self, TrackId, QueueItem};
use crate::player::metadata::{self, TrackInfo};
use crate::{App, MediaRecord, ytdlp, MediaId};

pub async fn index(app: State<App>) -> AppResult<Json<Vec<QueueItem>>> {
    let mut session = app.session().await?;
    Ok(Json(player::queue(&mut session).await?))
}

pub async fn show(app: State<App>, Path(track_id): Path<TrackId>) -> AppResult<Json<TrackInfo>> {
    let mut session = app.session().await?;
    Ok(Json(metadata::load(&mut session, &track_id).await?))
}

#[axum::debug_handler]
pub async fn add(app: State<App>, data: Json<proto::AddParams>) -> AppResult<Json<proto::AddResponse>> {
    let id = MediaId(uuid::Uuid::new_v4());

    let dir = app.working_dir().create_dir(&id.to_string()).await?;
    let dir = dir.into_shared();

    let download = ytdlp::start_download(dir, &data.url).await?;
    let metadata = download.metadata.clone();

    {
        let mut state = app.0.0.state.lock().unwrap();
        state.media_by_url.insert(data.url.clone(), id);
        state.media.insert(id, Arc::new(MediaRecord {
            url: data.url.clone(),
            download,
        }));
    }

    let stream_url = app.0.0.config.http.external_url
        .join(&format!("media/{id}/stream"))?;

    log::info!("Adding {}", metadata.title
        .unwrap_or_else(|| data.url.to_string()));

    let mut mpd = app.mpd().await?;
    let mpd_id = mpd.addid(&stream_url).await?;

    if should_autoplay(&mut mpd, &mpd_id).await? {
        mpd.play().await?;
    }

    Ok(Json(proto::AddResponse { mpd_id: mpd_id.into() }))
}

async fn should_autoplay(mpd: &mut Mpd, added_id: &mpd::Id) -> Result<bool> {
    let playlist = mpd.playlistinfo().await?;

    if playlist.items.len() != 1 {
        return Ok(false);
    }

    Ok(playlist.items[0].id == *added_id)
}
