use derive_more::From;
use url::Url;
use yew::prelude::*;

use crate::components::Add;
use crate::util::{cancellable, Cancelled, spawn_cancellable};

pub struct App;

pub enum AppEvent {
    AddUrl(Url),
}

#[derive(Debug, From)]
enum AddUrlError {
    Net(gloo_net::Error),
}

impl Component for App {
    type Message = AppEvent;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        App
    }

    fn update(&mut self, _: &Context<Self>, msg: AppEvent) -> bool {
        match msg {
            AppEvent::AddUrl(url) => {
                let fut = cancellable(|abort| {
                    async move {
                        gloo_net::http::Request::post("/queue/add")
                            .query([("url", &url)])
                            .abort_signal(Some(&abort))
                            .build()?
                            .send()
                            .await?
                            .json::<()>()
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

                false
            }
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
                    <div class="main-content"></div>
                    <div class="main-border main-border-bottom"></div>
                </main>
                <Add onsubmit={ctx.link().callback(AppEvent::AddUrl)} />
            </>
        }
    }
}
