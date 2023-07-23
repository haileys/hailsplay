use std::cell::RefCell;
use url::Url;
use yew::Callback;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew_websocket::format::Text;
use ws_proto::{ServerMessage, ClientMessage, MetadataRequest};
use uuid::Uuid;

use crate::subscribe::Subscriber;
use crate::util;
use crate::log;

thread_local! {
    static SESSION: Session = Session::default();
}

pub fn get() -> &'static Session {
    SESSION.with(|sess| unsafe {
        // Safety: safe on wasm because we only have one thread:
        std::mem::transmute::<&Session, &'static Session>(sess)
     })
}

pub type MetadataCallback = Callback<Option<ws_proto::Metadata>>;

#[derive(Default)]
pub struct Session {
    websocket: RefCell<Option<WebSocketTask>>,
    pub metadata: Subscriber<ws_proto::MetadataResponse>,
}

impl Session {
    pub fn send(&self, msg: ClientMessage) {
        match serde_json::to_string(&msg) {
            Ok(text) => {
                let mut websocket = self.websocket.borrow_mut();
                if let Some(ws) = websocket.as_mut() {
                    ws.send(text);
                } else {
                    log!("TODO! no websocket! queue the request");
                }
            }
            Err(e) => { log!("serde_json::to_string error: {e:?}, msg: {msg:?}"); }
        }
    }
}

fn on_server_message(msg: ServerMessage) {
    match msg {
        ServerMessage::MetadataResponse(msg) => {
            get().metadata.broadcast(msg)
        }
    }
}

pub fn start() {
    let websocket_url = util::websocket_origin() + "/ws";

    log!("before connect_text");

    let websocket = WebSocketService::connect_text(&websocket_url,
        Callback::from({
            move |msg: Text| {
                match msg {
                    Ok(buff) => {
                        match serde_json::from_str::<ServerMessage>(&buff) {
                            Ok(msg) => { on_server_message(msg) }
                            Err(e) => {
                                log!("deserialize error in websocket receive: {e:?}");
                            }
                        }
                    }
                    Err(e) => {
                        log!("websocket recv error: {:?}", e);
                    }
                }
            }
        }),
        Callback::from(|status: WebSocketStatus| {
            log!("websocket status: {:?}", status);
        }));

    match websocket {
        Ok(websocket) => {
            *get().websocket.borrow_mut() = Some(websocket);
        }
        Err(e) => {
            log!("websocket connect error: {e:?}");
        }
    }
}
