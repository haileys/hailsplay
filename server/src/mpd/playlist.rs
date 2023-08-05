use std::str::FromStr;

use hailsplay_protocol as proto;
use regex::Regex;
use url::Url;

use crate::mpd::{self, Mpd};
use crate::{App, MediaId};

pub async fn fetch(mpd: &mut Mpd, app: &App) -> anyhow::Result<proto::Playlist> {
    let playlist = mpd.playlistinfo().await?;
    Ok(map_playlist(app, &playlist))
}

pub fn map_playlist(app: &App, playlist: &mpd::Playlist) -> proto::Playlist {
    let items = playlist.items.iter()
        .map(|item| map_playlist_item(app, item))
        .collect();

    proto::Playlist { items }
}

fn map_playlist_item(app: &App, item: &mpd::PlaylistItem) -> proto::PlaylistItem {
    proto::PlaylistItem {
        id: item.id.clone().into(),
        meta: playlist_item_metadata(app, item)
            .unwrap_or_else(|| {
                proto::Metadata {
                    title: item.file.to_string(),
                    artist: None,
                    thumbnail: None,
                }
            }),
    }
}

fn playlist_item_metadata(app: &App, item: &mpd::PlaylistItem) -> Option<proto::Metadata> {
    let media_id = id_from_stream_url(&item.file)?;
    let state = app.lock_state();
    let media = state.media.get(&media_id)?;
    let metadata = media.metadata();

    Some(proto::Metadata {
        title: metadata.title.clone()
            .or_else(|| Some(media.download.file.path().file_name()?.to_string_lossy().to_string()))?,
        artist: metadata.uploader.clone(),
        thumbnail: metadata.thumbnail.clone(),
    })
}

fn id_from_stream_url(url: &Url) -> Option<MediaId> {
    lazy_static::lazy_static! {
        static ref URL_RE: Regex =
            Regex::new("^/media/(.*?)/stream$").unwrap();
    }

    if let Some(captures) = URL_RE.captures(url.path()) {
        let media_id = captures.get(1).unwrap().as_str();
        MediaId::from_str(media_id).ok()
    } else {
        None
    }
}
