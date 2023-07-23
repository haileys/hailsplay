mod error;
mod ytdlp;

use std::net::SocketAddr;
use std::task::Poll;

use axum::{
    response::IntoResponse,
    routing::get,
    Router,
    extract::TypedHeader,
};

//allows to extract the IP of connecting user
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{self, Message, WebSocket, WebSocketUpgrade};

use futures::{FutureExt, future, Future, StreamExt};
use tokio::sync::oneshot;
use ws_proto::{ClientMessage, MetadataRequest, Metadata, MetadataResponse, ServerMessage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let app = Router::new()
        .route("/ws", get(ws_handler));

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;


    Ok(())
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

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) -> anyhow::Result<()> {
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

                let metadata_fut = request_metadata(request)
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

async fn request_metadata(request: MetadataRequest) -> anyhow::Result<Metadata> {
    let metadata = ytdlp::metadata(request.url).await?;

    let thumbnail = metadata.thumbnails.into_iter()
        .max_by_key(|th| th.width + th.height);

    Ok(Metadata {
        title: metadata.title,
        artist: metadata.uploader,
        thumbnail: thumbnail.map(|th| th.url),
    })
}
