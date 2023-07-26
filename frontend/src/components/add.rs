use url::Url;
use web_sys::HtmlInputElement;
use hailsplay_protocol::Metadata;
use yew::{prelude::*, html::Scope};
use crate::util;

pub struct Add {
    link: Scope<Self>,
    state: State,
    input: NodeRef,
    // _handle: SubscribeHandle,
}

type MetadataResult = Result<Metadata, gloo_net::Error>;

enum State {
    Default,
    WaitingForMetadata(Url, util::TaskHandle),
    Metadata(MetadataResult),
}

pub enum AddMsg {
    Change,
    Submit(web_sys::SubmitEvent),
    Metadata(MetadataResult),
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

    fn fetch_metadata(&mut self) -> bool {
        let Some(url) = self.input_url() else { return false; };

        // don't send another request if inflight is the same url
        match &self.state {
            State::WaitingForMetadata(inflight_url, _) if inflight_url == &url => {
                return false;
            }
            _ => {}
        }

        let handle = util::link(&self.link)
            .map(AddMsg::Metadata)
            .spawn_cancellable({
                let url = url.clone();
                |abort| async move {
                    let metadata = gloo_net::http::Request::get("/metadata")
                        .query([("url", &url)])
                        .abort_signal(Some(&abort))
                        .build()
                        .expect("gloo_net::http::RequestBuilder::build")
                        .send()
                        .await?
                        .json::<Metadata>()
                        .await?;

                    Ok(metadata)
                }
            });

        self.state = State::WaitingForMetadata(url, handle);
        true
    }
}

impl Component for Add {
    type Properties = ();
    type Message = AddMsg;

    fn create(ctx: &Context<Self>) -> Self {
        Add {
            link: ctx.link().clone(),
            state: State::Default,
            input: NodeRef::default(),
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: AddMsg) -> bool {
        match msg {
            AddMsg::Change => {
                self.fetch_metadata();
                true
            }
            AddMsg::Submit(_) => {
                false
            }
            AddMsg::Metadata(result) => {
                match self.state {
                    State::WaitingForMetadata(..) => {}
                    _ => { return false; }
                };

                self.state = State::Metadata(result);
                true
            }
        }
    }

    fn view(&self, _: &Context<Self>) -> Html {
        html! {
            <div class="add">
                {match &self.state {
                    State::Default => html!{},
                    State::WaitingForMetadata(..) => html!{
                        <div class="loading-spinner">
                            <div class="lds-spinner"><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
                        </div>
                    },
                    State::Metadata(Ok(meta)) => html!{
                        <div class="add-preview">
                            <div class="preview-cover-art">
                                {match &meta.thumbnail {
                                    Some(url) => html!{<img src={url.to_string()} />},
                                    None => html!{},
                                }}
                            </div>
                            <div class="preview-details">
                                <div class="title">{&meta.title}</div>
                                {match &meta.artist {
                                    Some(artist) => html!{ <div class="artist">{artist}</div> },
                                    None => html!{}
                                }}
                            </div>
                        </div>
                    },
                    State::Metadata(Err(e)) => html! {
                        <div class="add-preview">
                            <div class="title">{format!("{e:?}")}</div>
                        </div>
                    },
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
