use std::{net::SocketAddr, mem::take};

use axum::{extract::{State, ConnectInfo, WebSocketUpgrade, ws::{WebSocket, Message}}, response::IntoResponse};
use hailsplay_protocol::{self as proto};
use proto::ClientMessage;
use serde::{Serialize, Deserialize};

use crate::{App, player::{PlayerStatus, metadata::TrackInfo, Queue}};
use crate::mpd::Changed;
use crate::player;

pub async fn handler(
    app: State<App>,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t", rename_all = "kebab-case")]
pub enum ServerMessage {
    Queue { queue: Queue },
    TrackChange { track: Option<TrackInfo> },
    Player { player: PlayerStatus },
}

async fn handle_socket(app: State<App>, ws: WebSocket, _: SocketAddr) -> anyhow::Result<()> {
    let mut session = app.session().await?;

    let mut socket = Socket { ws };

    let mut reload = Reload::new();
    let mut current_track = None;

    loop {
        if take(&mut reload.playlist) {
            let queue = player::queue(&mut session).await?;
            socket.send(ServerMessage::Queue { queue }).await?;
        }

        if take(&mut reload.player) {
            let player = player::status(&mut session).await?;

            // if current track has changed since the client last knew about
            // it, send an update
            if player.track != current_track {
                current_track = player.track.clone();

                let track = match &player.track {
                    Some(id) => Some(player::metadata::load(&mut session, id).await?),
                    None => None,
                };

                socket.send(ServerMessage::TrackChange { track }).await?;
            }

            socket.send(ServerMessage::Player { player }).await?;
        }

        let changed = session.mpd().idle().await?;
        reload.set(changed);
    }
}

struct Reload {
    playlist: bool,
    player: bool,
}

impl Reload {
    pub fn new() -> Self {
        Reload {
            playlist: true,
            player: true,
        }
    }

    pub fn set(&mut self, changed: Changed) {
        for sys in changed.subsystems {
            match sys.as_str() {
                "playlist" => { self.playlist = true; }
                "player" => { self.player = true; }
                _ => {
                    log::warn!("unknown subsystem: {sys}");
                }
            }
        }
    }
}
