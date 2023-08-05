use std::{cell::RefCell, time::Duration};

use derive_more::From;
use gloo::{net::websocket::{futures::WebSocket, WebSocketError, Message}, utils::errors::JsError};
use futures::StreamExt;
use hailsplay_protocol::{Playlist, ServerMessage};
use yew::Callback;

use crate::{subscribe::{Subscriber, SubscribeHandle}, util};

pub struct Session {
    playlist: Subscriber<Playlist>,
}

pub fn get() -> &'static Session {
    thread_local! {
        static SESSION: RefCell<Option<&'static Session>> = RefCell::new(None);
    }

    SESSION.with(|session| {
        &**session.borrow_mut()
            .get_or_insert_with(|| Box::leak(Box::new(start_session())))
    })
}

impl Session {
    pub fn watch_playlist(&self, callback: Callback<Playlist>) -> SubscribeHandle {
        self.playlist.subscribe(callback)
    }
}

fn handle_message(message: ServerMessage) {
    match message {
        ServerMessage::Playlist(playlist) => {
            get().playlist.broadcast(playlist);
        }
    }
}

fn start_session() -> Session {
    wasm_bindgen_futures::spawn_local(session_task());

    Session {
        playlist: Subscriber::default(),
    }
}

async fn session_task() {
    loop {
        match connection_task().await {
            Ok(()) => {
                crate::log!("websocket connection eof, reconnecting");
            }
            Err(e) => {
                crate::log!("websocket connection failed, reconnecting: {e:?}");
            }
        }
        gloo::timers::future::sleep(Duration::from_secs(1)).await;
    }
}

#[derive(Debug, From)]
enum ConnectionError {
    Connect(JsError),
    Socket(WebSocketError),
}

async fn connection_task() -> Result<(), ConnectionError> {
    let url = util::websocket_origin() + "/ws";
    let mut websocket = WebSocket::open(&url)?;

    loop {
        let msg = match websocket.next().await {
            None => { break; }
            Some(result) => result?,
        };

        let json = match msg {
            Message::Text(json) => json,
            Message::Bytes(_) => { continue; }
        };

        crate::log!("WS: {json:?}");

        let msg = match serde_json::from_str::<ServerMessage>(&json) {
            Ok(msg) => msg,
            Err(e) => {
                crate::log!("websocket message: json parse error: {e:?}");
                continue;
            }
        };

        handle_message(msg);
    }

    Ok(())
}
