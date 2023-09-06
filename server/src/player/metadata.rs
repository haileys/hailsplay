use std::str::FromStr;

use regex::Regex;
use serde::{Serialize, Deserialize};
use url::Url;

use crate::MediaId;
use crate::db::radio;
use crate::http::assets;
use crate::mpd::PlaylistItem;
use crate::player::{Session, TrackId};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    image_url: Option<Url>,
    primary_label: String,
    secondary_label: Option<String>,
}

pub async fn load(session: &mut Session, track_id: &TrackId) -> anyhow::Result<TrackInfo> {
    let item = session.mpd().playlistid(&track_id.0).await?;
    for_playlist_item(session, &item).await
}

pub async fn for_playlist_item(session: &mut Session, item: &PlaylistItem) -> anyhow::Result<TrackInfo> {
    if let Some(info) = media_item(session, &item) {
        return Ok(info);
    }

    if let Some(info) = radio_item(session, &item).await? {
        return Ok(info);
    }

    Ok(fallback_item(&item))
}

fn fallback_item(item: &PlaylistItem) -> TrackInfo {
    let primary_label = item.title.as_deref()
        .or(item.name.as_deref())
        .or(item.file.rsplit_once("/").map(|(_, filename)| filename))
        .unwrap_or(item.file.as_str())
        .to_string();

    TrackInfo {
        image_url: None,
        primary_label,
        secondary_label: None,
    }
}

async fn radio_item(session: &Session, item: &PlaylistItem) -> Result<Option<TrackInfo>, rusqlite::Error> {
    session.use_database(|conn| {
        let Some(station) = radio::find_by_url(conn, &item.file)? else {
            return Ok(None);
        };

        let image_url = assets::url(conn, session.config(), station.icon)?;
        let primary_label = station.name;
        let secondary_label = item.title.clone();

        Ok(Some(TrackInfo {
            image_url: Some(image_url),
            primary_label,
            secondary_label,
        }))
    }).await
}

fn media_item(session: &Session, item: &PlaylistItem) -> Option<TrackInfo> {
    let id = media_stream_url(&item.file)?;

    let state = session.app().lock_state();
    let media = state.media.get(&id)?;
    let metadata = media.metadata();

    let image_url = metadata.thumbnail.clone();

    let primary_label = metadata.title.clone()
        .or_else(|| Some(media.download.file.path().file_name()?.to_string_lossy().to_string()))?;

    let secondary_label = metadata.uploader.clone();

    Some(TrackInfo {
        image_url,
        primary_label,
        secondary_label,
    })
}

fn media_stream_url(url: &str) -> Option<MediaId> {
    lazy_static::lazy_static! {
        static ref URL_RE: Regex =
            Regex::new("^/media/(.*?)/stream$").unwrap();
    }

    let url = Url::parse(url).ok()?;

    if let Some(captures) = URL_RE.captures(url.path()) {
        let media_id = captures.get(1).unwrap().as_str();
        MediaId::from_str(media_id).ok()
    } else {
        None
    }
}
