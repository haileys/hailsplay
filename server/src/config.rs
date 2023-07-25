use std::{net::SocketAddr, path::{PathBuf, Path}};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub http: Http,
    pub storage: Storage,
}

#[derive(Serialize, Deserialize)]
pub struct Http {
    pub listen: SocketAddr,
}

#[derive(Serialize, Deserialize)]
pub struct Storage {
    pub database: PathBuf,
    pub archive: PathBuf,
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
