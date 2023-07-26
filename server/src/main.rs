mod error;
mod ytdlp;
mod config;
mod fs;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::task::Poll;

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
use axum::extract::ws::{self, Message, WebSocket, WebSocketUpgrade};

use config::Config;
use error::AppResult;
use fs::WorkingDirectory;
use futures::{FutureExt, future, Future, StreamExt};
use log::LevelFilter;
use serde::Deserialize;
use tokio::sync::oneshot;
use url::Url;
use hailsplay_protocol::{ClientMessage, Metadata, MetadataResponse, ServerMessage};
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
            log::error!("fatal error: {e:?}");
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> anyhow::Result<()> {
    let working = WorkingDirectory::open_or_create(&config.storage.working).await?;
    let media_state = App::new(config, working);

    let app = Router::new()
        .route("/queue/add", post(add))
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
struct App(pub Arc<AppCtx>);

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

#[derive(Deserialize)]
struct Add {
    url: Url,
}

#[axum::debug_handler]
async fn add(app: State<App>, data: Json<Add>) -> AppResult<Json<String>> {
    let id = uuid::Uuid::new_v4();

    let dir = app.0.0.working.create_dir(Path::new(&id.to_string())).await?;
    let dir = dir.into_shared();

    let download = ytdlp::start_download(dir, &data.url).await?;

    let path = download.file.path().display().to_string();

    let record = Arc::new(MediaRecord { 
        url: data.url.clone(),
        download,
    });
    
    {
        let mut state = app.0.0.state.lock().unwrap();
        state.media_by_url.insert(data.url.clone(), id);
        state.media.insert(id, record);
    }

    Ok(Json(path))
}

async fn ws_handler(
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
                match handle_socket(socket, addr).await {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("handle_socket returned error: {e:?}");
                    }
                }
            });
        }
    })
}

#[derive(Default)]
struct Session { 
    inflight_metadata_request: Option<oneshot::Receiver<MetadataResponse>>,
}

async fn handle_socket(mut socket: WebSocket, _: SocketAddr) -> anyhow::Result<()> {
    let mut session = Session::default();

    loop {
        enum Event {
            MetadataReady(MetadataResponse),
            Socket(Option<Result<ws::Message, axum::Error>>),
        }

        let fut = future::poll_fn(|cx| {
            if let Some(rx) = session.inflight_metadata_request.as_mut() {
                if let Poll::Ready(result) = rx.poll_unpin(cx) {
                    session.inflight_metadata_request = None;
                    if let Ok(response) = result {
                        return Poll::Ready(Event::MetadataReady(response));
                    }
                }
            }

            if let Poll::Ready(result) = socket.poll_next_unpin(cx) {
                return Poll::Ready(Event::Socket(result));
            }

            Poll::Pending
        });

        match fut.await {
            Event::MetadataReady(metadata) => {
                let msg = ServerMessage::MetadataResponse(metadata);
                let msg = ws::Message::Text(serde_json::to_string(&msg)?);
                socket.send(msg).await?;
            }
            Event::Socket(None) => { break; }
            Event::Socket(Some(result)) => {
                let text = match result? {
                    Message::Text(text) => text,
                    Message::Close(_) => { break; }
                    msg => {
                        eprintln!("WARN: websocket connection received unknown message kind: {msg:?}");
                        continue;
                    }
                };

                let msg = serde_json::from_str::<ClientMessage>(&text)?;

                println!("--> {msg:?}");

                handle_message(msg, &mut session).await?;
            }
        }
    }

    return Ok(())
}

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

async fn request_metadata(url: &Url) -> anyhow::Result<Metadata> {
    let metadata = ytdlp::fetch_metadata(url).await?;

    Ok(Metadata {
        title: metadata.title.unwrap_or_else(|| url.to_string()),
        artist: metadata.uploader,
        thumbnail: metadata.thumbnail,
    })
}
