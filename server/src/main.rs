mod error;
mod ytdlp;
mod config;
mod fs;
mod http;
mod mpd;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::{Query, State};
use axum::routing::post;
use axum::{
    response::IntoResponse,
    routing::get,
    Router,
    extract::TypedHeader,
};

use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

use config::Config;
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
        .route("/queue/add", post(http::queue::add))
        .route("/queue", get(http::queue::index))
        .route("/media/:id/stream", get(http::media::stream))
        .route("/metadata", get(metadata))
        .route("/ws", get(ws_handler))
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
}

pub struct AppCtx {
    pub config: Config,
    pub working: WorkingDirectory,
    pub state: Mutex<AppState>,
}

#[derive(Default)]
pub struct AppState {
    pub media_by_url: HashMap<Url, Uuid>,
    pub media: HashMap<Uuid, Arc<MediaRecord>>, 
}

pub struct MediaRecord {
    pub url: Url,
    pub download: ytdlp::DownloadHandle,
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

async fn ws_handler(
    app: State<App>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        async move {
            tokio::spawn(async move {
                match handle_socket(app, socket, addr).await {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("handle_socket returned error: {e:?}");
                    }
                }
            });
        }
    })
}

async fn handle_socket(app: State<App>, _socket: WebSocket, _: SocketAddr) -> anyhow::Result<()> {
    let mut mpd = app.mpd().await?;

    loop {
        let playlist = mpd.playlistinfo().await?;

        // do something with the playlist
        log::debug!("playlist -> {playlist:?}");

        let changed = mpd.idle().await?;

        log::debug!("changed -> {changed:?}");
    }
}

/*
async fn handle_message(msg: ClientMessage, session: &mut Session) -> anyhow::Result<()> {
    match msg {
        ClientMessage::MetadataRequest(request) => {
            let (mut tx, rx) = oneshot::channel();
            session.inflight_metadata_request = Some(rx);

            tokio::spawn(async move {
                let request_id = request.request_id;

                let metadata_fut = request_metadata(&request.url)
                    .map(|result| {
                        MetadataResponse {
                            request_id: request_id,
                            result: result.map_err(|e| format!("{e:?}")),
                        }
                    });

                futures::pin_mut!(metadata_fut);

                let fut = future::poll_fn(|cx| {
                    if let Poll::Ready(()) = tx.poll_closed(cx) {
                        return Poll::Ready(None);
                    }

                    if let Poll::Ready(response) = metadata_fut.as_mut().poll(cx) {
                        return Poll::Ready(Some(response));
                    }

                    Poll::Pending
                });

                if let Some(response) = fut.await {
                    let _ = tx.send(response);
                }
            });
        }
    }

    Ok(())
}
*/

async fn request_metadata(url: &Url) -> anyhow::Result<Metadata> {
    let metadata = ytdlp::fetch_metadata(url).await?;

    Ok(Metadata {
        title: metadata.title.unwrap_or_else(|| url.to_string()),
        artist: metadata.uploader,
        thumbnail: metadata.thumbnail,
    })
}
