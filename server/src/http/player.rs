use axum::extract::State;
use axum::Json;

use crate::{error::AppResult, App};

pub async fn play(app: State<App>) -> AppResult<Json<()>> {
    let mut session = app.session().await?;
    session.mpd().play().await?;
    Ok(Json(()))
}

pub async fn pause(app: State<App>) -> AppResult<Json<()>> {
    let mut session = app.session().await?;
    session.mpd().pause().await?;
    Ok(Json(()))
}

pub async fn stop(app: State<App>) -> AppResult<Json<()>> {
    let mut session = app.session().await?;
    session.mpd().stop().await?;
    Ok(Json(()))
}
