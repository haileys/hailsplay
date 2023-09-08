use std::str::FromStr;

use regex::Regex;
use url::Url;

use hailsplay_protocol::TrackInfo;

use crate::MediaId;
use crate::db::radio::{self, Station};
use crate::http::assets;
use crate::mpd::PlaylistItem;
use crate::api::Session;

pub async fn track_info(session: &mut Session, item: &TrackKind) -> anyhow::Result<TrackInfo> {
    match item {
        TrackKind::Media(id) => media_track_info(session, *id),
        TrackKind::Radio(item) => Ok(radio_track_info(session, item).await?),
        TrackKind::Unknown(item) => Ok(fallback_item(item)),
    }
}

pub async fn identify(session: &mut Session, item: &PlaylistItem) -> anyhow::Result<TrackKind> {
    if let Some(id) = media_stream_item(session, &item) {
        return Ok(TrackKind::Media(id));
    }

    if let Some(station) = radio_item(session, &item).await? {
        return Ok(TrackKind::Radio(station));
    }

    Ok(TrackKind::Unknown(item.clone()))
}

pub enum TrackKind {
    Radio(RadioItem),
    Media(MediaId),
    Unknown(PlaylistItem),
}

impl TrackKind {
    pub async fn load_info(&self, session: &mut Session) -> anyhow::Result<TrackInfo> {
        track_info(session, self).await
    }
}

pub struct RadioItem {
    pub station: Station,
    pub title: Option<String>,
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

async fn radio_track_info(session: &Session, item: &RadioItem) -> Result<TrackInfo, rusqlite::Error> {
    session.use_database(|conn| {
        let image_url = assets::url(conn, session.config(), item.station.icon)?;
        let primary_label = item.station.name.to_owned();
        let secondary_label = item.title.clone();

        Ok(TrackInfo {
            image_url: Some(image_url),
            primary_label,
            secondary_label,
        })
    }).await
}

async fn radio_item(session: &Session, item: &PlaylistItem) -> Result<Option<RadioItem>, rusqlite::Error> {
    session.use_database(|conn| {
        let title = item.title.to_owned();

        Ok(radio::find_by_url(conn, &item.file)?
            .map(|station| RadioItem { station, title }))
    }).await
}

fn media_track_info(session: &Session, id: MediaId) -> anyhow::Result<TrackInfo> {
    let state = session.app().lock_state();
    let Some(media) = state.media.get(&id) else {
        anyhow::bail!("can't find media id {id}");
    };

    let metadata = media.metadata();

    let image_url = metadata.thumbnail.clone();

    let primary_label = metadata.title.clone()
        .or_else(|| {
            media.download.file.path().file_name()
                .map(|filename| filename.to_string_lossy().to_string())
        })
        .unwrap_or_default();

    let secondary_label = metadata.uploader.clone();

    Ok(TrackInfo {
        image_url,
        primary_label,
        secondary_label,
    })
}

fn media_stream_item(session: &Session, item: &PlaylistItem) -> Option<MediaId> {
    lazy_static::lazy_static! {
        static ref URL_RE: Regex =
            Regex::new("^/media/(.*?)/stream$").unwrap();
    }

    let url = Url::parse(&item.file).ok()?;

    if let Some(captures) = URL_RE.captures(url.path()) {
        let id = captures.get(1).unwrap().as_str();
        let id = MediaId::from_str(id).ok()?;
        let state = session.app().lock_state();

        // ensure it exists:
        state.media.get(&id)?;

        Some(id)
    } else {
        None
    }
}
