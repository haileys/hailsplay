use std::{net::SocketAddr, path::{PathBuf, Path}};

use serde::{Serialize, Deserialize};
use url::Url;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub http: Http,
    pub mpd: Mpd,
    pub storage: Storage,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Http {
    pub listen: SocketAddr,
    pub internal_url: Url,
    pub external_url: Url,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Mpd {
    pub socket: PathBuf,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Storage {
    pub archive: PathBuf,
    pub working: PathBuf,
}

fn try_config(path: &Path) -> Option<Config> {
    let config_toml = match std::fs::read_to_string(path) {
        Ok(contents) => {
            log::info!("Using config at {}", path.display());
            contents
        }
        Err(e) => {
            log::debug!("Looking for config at {}: {e:?}", path.display());
            return None;
        }
    };

    match toml::from_str(&config_toml) {
        Ok(config) => Some(config),
        Err(e) => {
            log::error!("Error in config file: {e:?}");
            std::process::exit(1);
        }
    }
}

pub fn load() -> Config {
    let current_dir = std::env::current_dir().unwrap();
    
    if let Some(config) = try_config(&current_dir.join("config.toml")) {
        return config;
    }

    log::error!("Missing config file");
    std::process::exit(1);
}
