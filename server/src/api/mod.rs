pub mod session;
pub use session::Session;

pub mod metadata;

use hailsplay_protocol::{TrackId, PlayPosition, PlayState, PlayerStatus, Queue, QueueItem};
use crate::mpd::{self, Seconds, Status};

pub async fn status(session: &mut Session) -> anyhow::Result<PlayerStatus> {
    let status = session.mpd().status().await?;

    let track = status.song_id.clone().map(TrackId::from);

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

pub async fn queue(session: &mut Session) -> anyhow::Result<Queue> {
    let playlist = session.mpd().playlistinfo().await?;

    let mut items = Vec::new();

    for item in playlist.items {
        let track = metadata::for_playlist_item(&mut *session, &item).await?;
        items.push(QueueItem {
            id: item.id.into(),
            position: item.pos,
            track,
        });
    }

    Ok(Queue { items })
}
