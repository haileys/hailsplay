pub mod protocol;

use std::{str::FromStr, convert::Infallible};

use anyhow::{Result, anyhow, bail, Context};
use serde::{Serialize, Deserialize};
use tokio::net::UnixStream;
use url::Url;

use crate::config;

use self::protocol::{MpdReader, MpdWriter, Protocol, Response};

pub struct Mpd {
    conn: Conn,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Id(String);

impl FromStr for Id {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Infallible> {
        Ok(Id(s.to_string()))
    }
}

#[derive(Debug)]
pub struct Playlist {
    pub items: Vec<PlaylistItem>,
}

#[derive(Debug)]
pub struct PlaylistItem {
    pub file: Url,
    pub pos: i64,
    pub id: Id,
}

#[derive(Debug)]
pub struct Changed {
    pub subsystems: Vec<String>,
}

impl Mpd {
    pub async fn connect(config: &config::Mpd) -> Result<Mpd> {
        let (conn, proto) = Conn::connect(config).await?;
        log::info!("Connected to mpd at {}, protocol version {}",
            config.socket.display(), proto.version);
        Ok(Mpd { conn })
    }

    async fn command(&mut self, cmd: &str, args: &[&str]) -> Result<Response> {
        self.conn.writer.send_command(cmd, args).await?;
        let response = self.conn.reader.read_response().await?;
        Ok(response)
    }

    pub async fn addid(&mut self, uri: &Url) -> Result<Id> {
        let mut resp = self.command("addid", &[&uri.to_string()]).await??;

        let Some(id) = resp.attributes.get_one("Id") else {
            bail!("no Id attribute in addid response");
        };

        Ok(Id(id.to_owned()))
    }

    pub async fn playlistinfo(&mut self) -> Result<Playlist> {
        let resp = self.command("playlistinfo", &[]).await??;

        let items = resp.attributes.split_at("file")
            .into_iter()
            .map(|attrs| {
                Ok(PlaylistItem {
                    file: attrs.get("file")?,
                    pos: attrs.get("Pos")?,
                    id: attrs.get("Id")?,
                })
            })
            .collect::<Result<Vec<_>>>()
            .context("parsing playlist info response")?;

        Ok(Playlist { items })
    }

    pub async fn idle(&mut self) -> Result<Changed> {
        let resp = self.command("idle", &[]).await??;
        let subsystems = resp.attributes.get_all("changed")
            .map(|v| v.to_string())
            .collect();
        Ok(Changed { subsystems })
    }
}

struct Conn {
    reader: MpdReader,
    writer: MpdWriter,
}

impl Conn {
    pub async fn connect(config: &config::Mpd) -> Result<(Conn, Protocol)> {
        let sock = UnixStream::connect(&config.socket).await?;
        let (rx, tx) = sock.into_split();
        let (reader, proto) = MpdReader::open(rx).await?;
        let writer = MpdWriter::open(tx);
        Ok((Conn { reader, writer }, proto))
    }
}