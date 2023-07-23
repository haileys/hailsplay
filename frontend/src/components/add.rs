use url::Url;
use uuid::Uuid;
use web_sys::HtmlInputElement;
use ws_proto::{MetadataResponse, Metadata, ClientMessage, MetadataRequest};
use yew::{prelude::*, html::Scope};
use crate::{session, subscribe::SubscribeHandle};
use crate::log;

pub struct Add {
    link: Scope<Self>,
    state: State,
    input: NodeRef,
    _handle: SubscribeHandle,
}

enum State {
    Default,
    WaitingForMetadata(Uuid),
    Metadata(Metadata),
}

pub enum AddMsg {
    Change,
    Submit(web_sys::SubmitEvent),
    MetadataResponse(MetadataResponse),
}

impl Add {
    fn input_url(&self) -> Option<Url> {
        if let Some(el) = self.input.cast::<HtmlInputElement>() {
            if let Ok(url) = Url::parse(&el.value()) {
                return Some(url);
            }
        }

        None
    }

    fn fetch_metadata(&mut self) {
        let request_id = Uuid::new_v4();

        if let Some(url) = self.input_url() {
            let req = MetadataRequest {
                request_id,
                url,
            };
            
            session::get().send(ClientMessage::MetadataRequest(req));

            self.state = State::WaitingForMetadata(request_id);
        }
    }
}

impl Component for Add {
    type Properties = ();
    type Message = AddMsg;

    fn create(ctx: &Context<Self>) -> Self {
        let callback = ctx.link().callback(AddMsg::MetadataResponse);
        let handle = session::get().metadata.subscribe(callback);

        Add {
            link: ctx.link().clone(),
            state: State::Default,
            input: NodeRef::default(),
            _handle: handle,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: AddMsg) -> bool {
        match msg {
            AddMsg::Change => {
                self.fetch_metadata();
                true
            }
            AddMsg::Submit(_) => {
                false
            }
            AddMsg::MetadataResponse(response) => {
                let State::WaitingForMetadata(id) = self.state else {
                    return false;
                };

                if id != response.request_id {
                    return false;
                }

                match response.result {
                    Ok(metadata) => {
                        self.state = State::Metadata(metadata);
                        true
                    }
                    Err(e) => {
                        log!("metadata error! {e:?}");
                        false
                    }
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="add">
                {match &self.state {
                    State::Default => html!{},
                    State::WaitingForMetadata(_) => html!{
                        <div class="loading-spinner">
                            <div class="lds-spinner"><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
                        </div>
                    },
                    State::Metadata(meta) => html!{
                        <div class="add-preview">
                            <div class="preview-cover-art">
                                {match &meta.thumbnail {
                                    Some(url) => html!{<img src={url.to_string()} />},
                                    None => html!{},
                                }}
                            </div>
                            <div class="preview-details">
                                <div class="title">{&meta.title}</div>
                                <div class="artist">{&meta.artist}</div>
                            </div>
                        </div>
                    }
                }}

                <input
                    class="url-input"
                    type="text"
                    placeholder="Add URL..."
                    inputmode="url"
                    enterkeyhint="go"
                    ref={self.input.clone()}
                    oninput={self.link.callback(|_| AddMsg::Change)}
                    onpaste={self.link.callback(|_| AddMsg::Change)}
                    onkeyup={self.link.callback(|_| AddMsg::Change)}
                    onsubmit={self.link.callback(AddMsg::Submit)}
                />
            </div>
        }
    }
}
