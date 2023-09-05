mod error;
mod ytdlp;
mod config;
mod fs;
mod http;
mod mpd;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::Json;
use axum::extract::Query;
use axum::routing::post;
use axum::{
    routing::get,
    Router,
};

use config::Config;
use derive_more::{Display, FromStr};
use error::AppResult;
use fs::WorkingDirectory;
use log::LevelFilter;
use mpd::Mpd;
use serde::Deserialize;
use url::Url;
use hailsplay_protocol::Metadata;
use uuid::Uuid;

#[tokio::main]
async fn main() -> ExitCode {
    pretty_env_logger::formatted_builder()
        .filter(Some("hailsplay"), LevelFilter::Debug)
        .filter(None, LevelFilter::Info)
        .init();

    let config = config::load();

    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("fatal error: {e:?}\n{}", e.backtrace());
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> anyhow::Result<()> {
    let working = WorkingDirectory::open_or_create(&config.storage.working).await?;
    let media_state = App::new(config, working);

    let app = Router::new()
        .route("/api/queue/add", post(http::queue::add))
        .route("/api/queue", get(http::queue::index))
        .route("/api/metadata", get(metadata))
        .route("/media/:id/stream", get(http::media::stream))
        .route("/ws", get(http::ws::handler))
        .with_state(media_state);

    let fut = axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>());

    log::info!("Listening on 0.0.0.0:3000");

    fut.await?;

    Ok(())
}

#[derive(Clone)]
pub struct App(pub Arc<AppCtx>);

impl App {
    pub async fn mpd(&self) -> anyhow::Result<Mpd> {
        Ok(Mpd::connect(&self.0.config.mpd).await?)
    }

    pub fn working_dir(&self) -> &WorkingDirectory {
        &self.0.working
    }

    pub fn lock_state(&self) -> MutexGuard<AppState> {
        self.0.state.lock().unwrap()
    }
}

pub struct AppCtx {
    pub config: Config,
    pub working: WorkingDirectory,
    pub state: Mutex<AppState>,
}

#[derive(Default)]
pub struct AppState {
    pub media_by_url: HashMap<Url, MediaId>,
    pub media: HashMap<MediaId, Arc<MediaRecord>>,
}

#[derive(Debug, Display, Deserialize, FromStr, Clone, Copy, Hash, PartialEq, Eq)]
#[display(fmt = "{}", "self.0")]
pub struct MediaId(pub Uuid);

pub struct MediaRecord {
    pub url: Url,
    pub download: ytdlp::DownloadHandle,
}

impl MediaRecord {
    pub fn metadata(&self) -> &ytdlp::Metadata {
        &self.download.metadata
    }
}

impl App {
    pub fn new(config: Config, working: WorkingDirectory) -> Self {
        App(Arc::new(AppCtx {
            config,
            working,
            state: Default::default(),
        }))
    }
}

#[derive(Deserialize)]
struct MetadataParams {
    url: Url,
}

async fn metadata(params: Query<MetadataParams>) -> AppResult<Json<Metadata>> {
    log::info!("Fetching metadata for {}", params.url);
    let metadata = request_metadata(&params.url).await?;
    Ok(Json(metadata))
}

async fn request_metadata(url: &Url) -> anyhow::Result<Metadata> {
    let metadata = ytdlp::fetch_metadata(url).await?;

    Ok(Metadata {
        title: metadata.title.unwrap_or_else(|| url.to_string()),
        artist: metadata.uploader,
        thumbnail: metadata.thumbnail,
    })
}
