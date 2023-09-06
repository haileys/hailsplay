use std::path::{PathBuf, Path};

use structopt::StructOpt;
use url::Url;

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
    let mut conn = database.get().await;

    let filename = opt.icon.file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_string();

    let content_type = match mime_type_from_image_filename(&opt.icon) {
        Some(content_type) => content_type.to_string(),
        None => { anyhow::bail!("unsupported image type: {}", opt.icon.display()); }
    };

    tokio::task::block_in_place(|| -> anyhow::Result<()> {
        let data = std::fs::read(&opt.icon)?;
        let asset_id = db::asset::create(&mut conn, filename, content_type.to_string(), &data)?;

        db::radio::insert_station(&mut conn, Station {
            name: opt.name,
            icon: asset_id,
            stream_url: opt.stream_url,
        })?;

        Ok(())
    })?;

    Ok(())
}

fn mime_type_from_image_filename(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;

    match ext {
        "png" => Some("image/png"),
        "jpg" => Some("image/jpg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}
