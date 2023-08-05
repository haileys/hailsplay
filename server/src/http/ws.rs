use std::net::SocketAddr;

use axum::{extract::{State, ConnectInfo, WebSocketUpgrade, ws::{WebSocket, Message}}, response::IntoResponse};
use hailsplay_protocol::{self as proto, ServerMessage};
use proto::ClientMessage;

use crate::App;
use crate::mpd::playlist;

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

async fn handle_socket(app: State<App>, ws: WebSocket, _: SocketAddr) -> anyhow::Result<()> {
    let mut mpd = app.mpd().await?;

    let mut socket = Socket { ws };

    loop {
        let playlist = playlist::fetch(&mut mpd, &app).await?;
        socket.send(ServerMessage::Playlist(playlist)).await?;

        let changed = mpd.idle().await?;
        log::debug!("changed -> {changed:?}");
    }
}
