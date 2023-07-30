use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Serialize, Deserialize};
use url::Url;

use crate::error::AppResult;
use crate::{App, mpd, MediaRecord, ytdlp};

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


#[derive(Deserialize)]
pub struct Add {
    url: Url,
}

#[derive(Serialize)]
pub struct AddResponse {
    mpd_id: mpd::Id,
}

#[axum::debug_handler]
pub async fn add(app: State<App>, data: Json<Add>) -> AppResult<Json<AddResponse>> {
    let id = uuid::Uuid::new_v4();

    let dir = app.working_dir().create_dir(Path::new(&id.to_string())).await?;
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

    Ok(Json(AddResponse { mpd_id }))
}
