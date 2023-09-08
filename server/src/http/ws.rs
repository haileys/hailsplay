use std::net::SocketAddr;

use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;

use crate::App;
use crate::mpd::MpdEvent;
use crate::api::{self, Session};
use hailsplay_protocol::{ClientMessage, ServerMessage, TrackId};

pub async fn handler(
    app: axum::extract::State<App>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        async move {
            tokio::spawn(async move {
                match handle_socket(app, socket, addr).await {
                    Ok(()) => {}
                    Err(e) => {
                        if let Some(e) = e.downcast_ref::<std::io::Error>() {
                            log::warn!("handle_socket io error: {e:?}");
                            return;
                        }

                        log::warn!("handle_socket returned error: {e:?}");
                    }
                }
            });
        }
    })
}

struct Socket {
    ws: WebSocket,
}

impl Socket {
    #[allow(unused)]
    pub async fn recv(&mut self) -> anyhow::Result<Option<ClientMessage>> {
        let msg = self.ws.recv().await.transpose()?;

        let Some(msg) = msg else {
            return Ok(None);
        };

        let Message::Text(json) = msg else {
            anyhow::bail!("unexpected message type");
        };

        Ok(Some(serde_json::from_str(&json)?))
    }

    pub async fn send(&mut self, msg: ServerMessage) -> anyhow::Result<()> {
        let json = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(json)).await?;
        Ok(())
    }
}

struct State {
    socket: Socket,
    session: Session,
    current_track: Option<TrackId>,
}

async fn handle_socket(app: axum::extract::State<App>, ws: WebSocket, _: SocketAddr) -> anyhow::Result<()> {
    let session = app.session().await?;

    let mut state = State {
        socket: Socket { ws },
        session,
        current_track: None,
    };

    // send initial state to client
    send_player_status(&mut state).await?;
    send_playlist(&mut state).await?;

    // watch events
    loop {
        let changed = state.session.mpd().idle().await?;

        for event in changed.events() {
            match event {
                MpdEvent::Playlist => send_playlist(&mut state).await?,
                MpdEvent::Player => send_player_status(&mut state).await?,
            }
        }
    }
}

async fn send_playlist(state: &mut State) -> anyhow::Result<()> {
    let queue = api::queue(&mut state.session).await?;
    state.socket.send(ServerMessage::Queue { queue }).await?;
    Ok(())
}

async fn send_player_status(state: &mut State) -> anyhow::Result<()> {
    let player = api::status(&mut state.session).await?;

    // if current track has changed since the client last knew about
    // it, send an update
    if player.track != state.current_track {
        state.current_track = player.track.clone();

        let track = match &player.track {
            Some(id) => api::track(&mut state.session, id).await?,
            None => None,
        };

        let track_info = match track {
            Some(track) => Some(track.load_info(&mut state.session).await?),
            None => None,
        };

        state.socket.send(ServerMessage::TrackChange { track: track_info }).await?;
    }

    state.socket.send(ServerMessage::Player { player }).await?;

    Ok(())
}
