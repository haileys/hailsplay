use axum::Router;
use axum::routing::{get, post};

use crate::App;

pub mod assets;
pub mod media;
pub mod metadata;
pub mod player;
pub mod queue;
pub mod radio;
pub mod ws;

pub fn routes(app: App) -> Router {
    Router::new()
        .route("/api/queue", post(queue::add))
        .route("/api/queue", get(queue::index))
        .route("/api/queue/:id", get(queue::show))
        .route("/api/radio/tune", post(radio::tune))
        .route("/api/radio/stations", get(radio::stations))
        .route("/api/metadata", get(metadata::metadata))
        .route("/assets/:id/:digest/:filename", get(assets::file))
        .route("/media/:id/stream", get(media::stream))
        .route("/ws", get(ws::handler))
        .with_state(app)
}
