use derive_more::From;
use url::Url;
use yew::prelude::*;

use hailsplay_protocol as proto;

use crate::components::{Add, Playlist};
use crate::util::{cancellable, spawn_cancellable};

pub struct App;

pub enum AppEvent {
    AddUrl(Url),
}

#[derive(Debug, From)]
enum AddUrlError {
    Net(gloo::net::Error),
}

impl Component for App {
    type Message = AppEvent;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        App
    }

    fn update(&mut self, _: &Context<Self>, msg: AppEvent) -> bool {
        match msg {
            AppEvent::AddUrl(url) => { add_url(url); false }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <header>
                    <div class="app-name">{"hailsPlay"}</div>
                </header>
                <main>
                    <div class="main-border main-border-top"></div>
                    <div class="main-content">
                        <Playlist />
                    </div>
                    <div class="main-border main-border-bottom"></div>
                </main>
                <Add onsubmit={ctx.link().callback(AppEvent::AddUrl)} />
            </>
        }
    }
}

pub fn add_url(url: Url) {
    let fut = cancellable(|abort| {
        async move {
            gloo::net::http::Request::post("/queue/add")
                .abort_signal(Some(&abort))
                .json(&proto::AddParams { url })?
                .send()
                .await?
                .json::<proto::AddResponse>()
                .await?;

            Ok::<(), AddUrlError>(())
        }
    });

    spawn_cancellable(async move {
        match fut.await? {
            Ok(()) => {}
            Err(e) => {
                crate::log!("error! {e:?}");
            }
        }

        Ok(())
    });
}
