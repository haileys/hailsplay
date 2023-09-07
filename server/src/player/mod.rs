pub mod session;
pub use session::Session;

pub mod metadata;

use serde::{Serialize, Deserialize};

use crate::mpd::{self, Seconds, Status};

use self::metadata::TrackInfo;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TrackId(pub mpd::Id);

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerStatus {
    pub track: Option<TrackId>,
    pub state: PlayState,
    pub position: Option<PlayPosition>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlayState {
    Stopped,
    Loading,
    Playing,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "kebab-case")]
pub enum PlayPosition {
    Streaming,
    Elapsed { time: f64, duration: f64 },
}

pub async fn status(session: &mut Session) -> anyhow::Result<PlayerStatus> {
    let status = session.mpd().status().await?;

    let track = status.song_id.clone().map(TrackId);

    Ok(PlayerStatus {
        track,
        state: play_state(&status),
        position: play_position(&status),
    })
}

fn play_state(status: &Status) -> PlayState {
    if status.state == mpd::PlayerState::Play {
        if status.audio_format.is_none() {
            // a state of playing with no audio format indicates that the
            // audio file/stream is still loading
            PlayState::Loading
        } else {
            PlayState::Playing
        }
    } else {
        PlayState::Stopped
    }
}

fn play_position(status: &Status) -> Option<PlayPosition> {
    match (status.elapsed, status.duration) {
        (None, None) => None,
        (Some(_), None) => Some(PlayPosition::Streaming),
        (Some(Seconds(time)), Some(Seconds(duration))) => Some(PlayPosition::Elapsed { time, duration }),
        (None, Some(Seconds(duration))) => {
            // would be unexpected, but lets do something sensible
            Some(PlayPosition::Elapsed { time: 0.0, duration: duration })
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: TrackId,
    pub position: i64,
    pub track: TrackInfo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Queue {
    pub items: Vec<QueueItem>,
}

pub async fn queue(session: &mut Session) -> anyhow::Result<Queue> {
    let playlist = session.mpd().playlistinfo().await?;

    let mut items = Vec::new();

    for item in playlist.items {
        let track = metadata::for_playlist_item(&mut *session, &item).await?;
        items.push(QueueItem {
            id: TrackId(item.id),
            position: item.pos,
            track,
        });
    }

    Ok(Queue { items })
}
