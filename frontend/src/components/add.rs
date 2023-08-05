use url::Url;
use web_sys::HtmlInputElement;
use hailsplay_protocol::Metadata;
use yew::{prelude::*, html::Scope};
use crate::util::{self, TaskHandle};

pub struct Add {
    link: Scope<Self>,
    url: Option<Url>,
    metadata: MetadataState,
    input: NodeRef,
}

#[derive(Properties, PartialEq)]
pub struct AddProps {
    pub onsubmit: Callback<Url>,
}

type MetadataResult = Result<Metadata, gloo::net::Error>;

#[derive(Debug)]
enum MetadataState {
    Value(Option<MetadataResult>),
    Loading(TaskHandle),
}

pub enum AddMsg {
    Change,
    Submit,
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

    fn set_url(&mut self, url: Option<Url>) -> bool {
        if self.url == url {
            return false;
        }

        self.url = url.clone();
        self.metadata = match url {
            None => MetadataState::Value(None),
            Some(url) => MetadataState::Loading(
                util::link(&self.link)
                    .map(AddMsg::Metadata)
                    .spawn_cancellable({
                        let url = url.clone();
                        |abort| async move {
                            let metadata = gloo::net::http::Request::get("/metadata")
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
                    })
            ),
        };

        true
    }

    fn take_url(&mut self) -> Option<Url> {
        let url = self.url.take();
        if url.is_some() {
            self.metadata = MetadataState::Value(None);
        }
        url
    }
}

impl Component for Add {
    type Properties = AddProps;
    type Message = AddMsg;

    fn create(ctx: &Context<Self>) -> Self {
        Add {
            link: ctx.link().clone(),
            url: None,
            metadata: MetadataState::Value(None),
            input: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: AddMsg) -> bool {
        match msg {
            AddMsg::Change => {
                self.set_url(self.input_url());
                true
            }
            AddMsg::Submit => {
                if let Some(url) = self.take_url() {
                    ctx.props().onsubmit.emit(url);
                }
                true
            }
            AddMsg::Metadata(result) => {
                self.metadata = MetadataState::Value(Some(result));
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="add">
                {match &self.metadata {
                    MetadataState::Value(None) => html!{},
                    MetadataState::Loading(..) => html!{
                        <div class="loading-spinner">
                            <div class="lds-spinner"><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
                        </div>
                    },
                    MetadataState::Value(Some(Ok(meta))) => html!{
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
                    MetadataState::Value(Some(Err(e))) => html! {
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
                    oninput={ctx.link().callback(|_| AddMsg::Change)}
                    onpaste={ctx.link().callback(|_| AddMsg::Change)}
                    onkeyup={ctx.link().callback(on_key_up)}
                    onsubmit={ctx.link().callback(|_| AddMsg::Submit)}
                />
            </div>
        }
    }
}

fn on_key_up(ev: KeyboardEvent) -> AddMsg {
    if ev.key_code() == 13 {
        ev.prevent_default();
        AddMsg::Submit
    } else {
        AddMsg::Change
    }
}
