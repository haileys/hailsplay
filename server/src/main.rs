mod config;
mod error;
mod frontend;
mod fs;
mod http;
mod mpd;
mod ytdlp;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, MutexGuard};

use config::Config;
use derive_more::{Display, FromStr};
use fs::WorkingDirectory;
use log::LevelFilter;
use mpd::Mpd;
use serde::Deserialize;
use url::Url;
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

    let app = App::new(config, working);
    let router = http::routes(app);
    let router = frontend::serve(router);

    let fut = axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

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
