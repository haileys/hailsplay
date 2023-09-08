use std::time::Duration;

use futures::{future, FutureExt};
use tokio::{sync::oneshot};

use crate::{App, api::{Session, metadata::{self, TrackKind}}, mpd::MpdEvent};

/// mpd maintenance tasks
/// these run in the background while the app is running

pub struct MaintenaceTask {
    // closes on drop:
    _handle: oneshot::Sender<()>,
}

pub async fn start(app: App) -> MaintenaceTask {
    let (keepalive_tx, keepalive_rx) = oneshot::channel();

    let task = task(app.clone());
    let keepalive = keepalive_rx.map(|_| ());
    let fut = future::join(task, keepalive);
    tokio::task::spawn(fut);

    MaintenaceTask {
        _handle: keepalive_tx,
    }
}

async fn task(app: App) {
    loop {
        match app.session().await {
            Ok(mut session) => {
                match run_session(&mut session).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("maintenance exited abnormally: {e:?}");
                    }
                }
            }
            Err(e) => {
                log::warn!("could not open app session in maint task, sleeping 5 seconds: {e:?}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        };
    }
}

enum NoReturn {}

async fn run_session(session: &mut Session) -> anyhow::Result<NoReturn> {
    loop {
        let changed = session.mpd().idle().await?;

        for event in changed.events() {
            match event {
                MpdEvent::Player => {
                    clear_radio_stations_from_history(session).await?;
                }
                MpdEvent::Playlist => {}
            }
        }
    }
}

// clears all radio stations from history except the current, if any
async fn clear_radio_stations_from_history(session: &mut Session)
    -> anyhow::Result<()>
{
    let status = session.mpd().status().await?;
    let playlist = session.mpd().playlistinfo().await?;

    for item in &playlist.items {
        if Some(&item.id) == status.song_id.as_ref() {
            continue;
        }

        match metadata::identify(session, item).await? {
            TrackKind::Radio(_) => {
                session.mpd().deleteid(&item.id).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
