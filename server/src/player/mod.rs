pub mod session;
pub use session::Session;

pub mod metadata;

use serde::{Serialize, Deserialize};

use crate::mpd::{self, Seconds};

use self::metadata::TrackInfo;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TrackId(pub mpd::Id);

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerStatus {
    pub track: Option<TrackId>,
    pub playing: bool,
    pub position: Option<PlayPosition>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "kebab-case")]
pub enum PlayPosition {
    Streaming,
    Elapsed { time: f64, duration: f64 },
}

pub async fn status(session: &mut Session) -> anyhow::Result<PlayerStatus> {
    let status = session.mpd().status().await?;

    let track = status.song_id.map(TrackId);

    let playing = match status.state {
        | mpd::PlayerState::Play => true,
        | mpd::PlayerState::Pause => false,
        | mpd::PlayerState::Stop => false,
    };

    let position = match (status.elapsed, status.duration) {
        (None, None) => None,
        (Some(_), None) => Some(PlayPosition::Streaming),
        (Some(Seconds(time)), Some(Seconds(duration))) => Some(PlayPosition::Elapsed { time, duration }),
        (None, Some(Seconds(duration))) => {
            // would be unexpected, but lets do something sensible
            Some(PlayPosition::Elapsed { time: 0.0, duration: duration })
        }
    };

    Ok(PlayerStatus {
        track,
        playing,
        position,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueItem {
    pub position: i64,
    pub track: TrackInfo
}

pub async fn queue(session: &mut Session) -> anyhow::Result<Vec<QueueItem>> {
    let playlist = session.mpd().playlistinfo().await?;

    let mut queue = Vec::new();

    for item in playlist.items {
        let track = metadata::for_playlist_item(session, &item).await?;

        queue.push(QueueItem {
            position: item.pos,
            track,
        });
    }

    Ok(queue)
}
