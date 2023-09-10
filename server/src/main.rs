mod api;
mod config;
mod db;
mod error;
mod frontend;
mod fs;
mod http;
mod maint;
mod mime;
mod mpd;
mod tools;
mod ytdlp;

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;

use api::archive::Archive;
use log::LevelFilter;
use structopt::StructOpt;

use crate::config::Config;
use crate::fs::WorkingDirectory;
use crate::mpd::Mpd;

#[derive(StructOpt)]
enum Cmd {
    Server,
    #[structopt(flatten)]
    Tool(tools::Cmd),
}


#[tokio::main]
async fn main() -> ExitCode {
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("hailsplay"), LevelFilter::Debug)
        .filter(None, LevelFilter::Info)
        .parse_default_env()
        .init();

    let cmd = Cmd::from_args();
    let config = config::load();

    let result = match cmd {
        Cmd::Server => run(config).await,
        Cmd::Tool(cmd) => tools::run(cmd, config).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("fatal error: {e:?}\n{}", e.backtrace());
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> anyhow::Result<()> {
    let working = WorkingDirectory::open_or_create(&config.storage.working).await?;
    let database = db::open(&config.storage.database).await?;

    let app = App::new(config, working, database);
    let router = http::routes(app.clone());
    let router = frontend::serve(router);

    let fut = axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    log::info!("Listening on 0.0.0.0:3000");

    // start maintenance task
    let _maint = maint::start(app.clone());

    fut.await?;

    Ok(())
}

#[derive(Clone)]
pub struct App(pub Arc<AppShared>);

impl App {
    pub async fn session(&self) -> anyhow::Result<api::Session> {
        api::session::Session::new(self.clone()).await
    }

    pub async fn mpd(&self) -> anyhow::Result<Mpd> {
        Ok(Mpd::connect(&self.0.config.mpd).await?)
    }

    pub async fn use_database<R>(&self, f: impl FnOnce(&mut rusqlite::Connection) -> R) -> R {
        self.0.database.with(f).await
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }

    pub fn working_dir(&self) -> &WorkingDirectory {
        &self.0.working
    }

    pub fn archive(&self) -> Archive {
        self.0.archive.clone()
    }
}

pub struct AppShared {
    pub config: Config,
    pub working: WorkingDirectory,
    pub archive: Archive,
    pub database: db::Pool,
}

impl App {
    pub fn new(config: Config, working: WorkingDirectory, database: db::Pool) -> Self {
        let archive = Archive::new(database.clone(), working.clone());

        App(Arc::new(AppShared {
            config,
            working,
            archive,
            database,
        }))
    }
}
