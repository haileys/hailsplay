use axum::Router;
use axum::routing::{get, post};

use crate::App;

pub mod media;
pub mod metadata;
pub mod queue;
pub mod radio;
pub mod ws;

pub fn routes(app: App) -> Router {
    Router::new()
        .route("/api/queue", post(queue::add))
        .route("/api/queue", get(queue::index))
        .route("/api/metadata", get(metadata::metadata))
        .route("/media/:id/stream", get(media::stream))
        .route("/ws", get(ws::handler))
        .with_state(app)
}
