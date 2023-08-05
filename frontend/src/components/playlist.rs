use hailsplay_protocol as proto;
use yew::{html, Component, Context};

use crate::{subscribe::SubscribeHandle, session};

pub struct Playlist {
    playlist: Option<proto::Playlist>,
    _handle: SubscribeHandle,
}

pub enum PlaylistMsg {
    Update(proto::Playlist),
}

impl Component for Playlist {
    type Message = PlaylistMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let update_playlist = ctx.link().callback(PlaylistMsg::Update);
        let handle = session::get().watch_playlist(update_playlist);

        Playlist {
            playlist: None,
            _handle: handle,
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: PlaylistMsg) -> bool {
        match msg {
            PlaylistMsg::Update(playlist) => {
                self.playlist = Some(playlist);
                true
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> yew::Html {
        let Some(playlist) = &self.playlist else {
            return html!{};
        };

        html!{
            <div class="playlist">
                {for playlist.items.iter().map(|item| html!{
                    <div class="playlist-item">
                        <div class="playlist-item-cover-art">
                            {item.meta.thumbnail.as_ref().map(|thumbnail| html!{
                                <img src={thumbnail.to_string()} />
                            })}
                        </div>
                        <div class="playlist-item-details">
                            <div class="title">{&item.meta.title}</div>
                            {item.meta.artist.as_ref().map(|artist| html!{
                                <div class="artist">{artist}</div>
                            })}
                        </div>
                    </div>
                })}
            </div>
        }
    }
}
