use std::path::PathBuf;

use structopt::StructOpt;
use url::Url;

use crate::api::asset;
use crate::config::Config;
use crate::db;
use crate::db::radio::Station;

#[derive(StructOpt)]
pub enum Cmd {
    AddStation(AddStationOpt),
}

pub async fn run(cmd: Cmd, config: Config) -> anyhow::Result<()> {
    match cmd {
        Cmd::AddStation(opt) => add_station(opt, config).await,
    }
}

#[derive(StructOpt)]
pub struct AddStationOpt {
    #[structopt(long)]
    name: String,
    #[structopt(long)]
    icon: PathBuf,
    #[structopt(long)]
    stream_url: Url,
}

async fn add_station(opt: AddStationOpt, config: Config) -> anyhow::Result<()> {
    let database = db::open(&config.storage.database).await?;

    let asset = asset::upload(&opt.icon).await?;

    database.with(|conn| {
        let asset_id = asset.insert(conn)?;

        db::radio::insert_station(conn, Station {
            name: opt.name,
            icon: asset_id,
            stream_url: opt.stream_url,
        })
    }).await?;

    Ok(())
}
