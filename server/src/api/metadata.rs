use std::str::FromStr;

use regex::Regex;
use url::Url;

use hailsplay_protocol::TrackInfo;

use crate::api::archive::MediaStreamId;
use crate::db::radio::{self, Station};
use crate::http::assets;
use crate::mpd::PlaylistItem;
use crate::api::Session;

pub async fn track_info(session: &mut Session, item: &TrackKind) -> anyhow::Result<TrackInfo> {
    match item {
        TrackKind::Media(id) => Ok(media_track_info(session, *id).await?),
        TrackKind::Radio(item) => Ok(radio_track_info(session, item).await?),
        TrackKind::Unknown(item) => Ok(fallback_item(item)),
    }
}

pub async fn identify(session: &mut Session, item: &PlaylistItem) -> anyhow::Result<TrackKind> {
    if let Some(id) = media_stream_item(session, &item).await? {
        return Ok(TrackKind::Media(id));
    }

    if let Some(station) = radio_item(session, &item).await? {
        return Ok(TrackKind::Radio(station));
    }

    Ok(TrackKind::Unknown(item.clone()))
}

pub enum TrackKind {
    Radio(RadioItem),
    Media(MediaStreamId),
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

async fn media_track_info(session: &Session, id: MediaStreamId) -> anyhow::Result<TrackInfo> {
    let media_record = session.app()
        .archive()
        .load(id)
        .await?;

    let Some(media_record) = media_record else {
        anyhow::bail!("can't find media id {id}");
    };

    let metadata = media_record.parse_metadata()?;

    let image_url = metadata.thumbnail.clone();

    let primary_label = metadata.title.clone()
        .unwrap_or_else(|| media_record.filename());

    let secondary_label = metadata.uploader.clone();

    Ok(TrackInfo {
        image_url,
        primary_label,
        secondary_label,
    })
}

async fn media_stream_item(session: &Session, item: &PlaylistItem)
    -> Result<Option<MediaStreamId>, rusqlite::Error>
{
    lazy_static::lazy_static! {
        static ref URL_RE: Regex =
            Regex::new("^/media/(.*?)/stream$").unwrap();
    }

    let parsed = Url::parse(&item.file).ok();

    let id = parsed.as_ref()
        .and_then(|url| URL_RE.captures(url.path()))
        .and_then(|captures| captures.get(1))
        .and_then(|capture| MediaStreamId::from_str(capture.as_str()).ok());

    let Some(id) = id else {
        return Ok(None);
    };

    // validate parsed id by trying to load it and seeing if it exists:
    let record = session.app().archive().load(id).await?;
    Ok(record.map(|_| id))
}
