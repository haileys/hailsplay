use crate::{App, Config};
use crate::mpd::Mpd;

pub struct Session {
    app: App,
    mpd: Mpd,
}

impl Session {
    pub async fn new(app: App) -> Result<Self, anyhow::Error> {
        let mpd = app.mpd().await?;

        Ok(Session {
            app,
            mpd,
        })
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn config(&self) -> &Config {
        self.app.config()
    }

    pub fn mpd(&mut self) -> &mut Mpd {
        &mut self.mpd
    }

    pub async fn use_database<R>(
        &self,
        func: impl FnOnce(&mut rusqlite::Connection) -> R,
    ) -> R {
        self.app.use_database(func).await
    }
}

