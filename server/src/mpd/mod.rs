pub mod protocol;

use std::{str::FromStr, convert::Infallible};

use anyhow::{Result, Context, bail};
use derive_more::FromStr;
use hailsplay_protocol::TrackId;
use serde::{Serialize, Deserialize};
use tokio::net::UnixStream;
use url::Url;

use crate::config;

use self::protocol::{MpdReader, MpdWriter, Protocol, Response, Attributes};

pub struct Mpd {
    conn: Conn,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Id(String);

impl FromStr for Id {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Infallible> {
        Ok(Id(s.to_string()))
    }
}

impl From<Id> for TrackId {
    fn from(id: Id) -> TrackId {
        TrackId(id.0)
    }
}

impl From<TrackId> for Id {
    fn from(id: TrackId) -> Id {
        Id(id.0)
    }
}

#[derive(Debug)]
pub struct Playlist {
    pub items: Vec<PlaylistItem>,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub file: String,
    pub pos: i64,
    pub id: Id,
    pub name: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug)]
pub struct Changed {
    subsystems: Vec<String>,
}

impl Changed {
    pub fn events(&self) -> impl Iterator<Item = MpdEvent> + '_ {
        self.subsystems.iter()
            .filter_map(|subsystem| {
                match subsystem.parse() {
                    Ok(event) => Some(event),
                    Err(()) => {
                        log::warn!("unknown subsystem: {subsystem}");
                        None
                    }
                }
            })
    }
}

#[derive(Debug)]
pub enum MpdEvent {
    Playlist,
    Player,
}

impl FromStr for MpdEvent {
    type Err = ();

    fn from_str(s: &str) -> Result<MpdEvent, ()> {
        match s {
            "player" => Ok(MpdEvent::Player),
            "playlist" => Ok(MpdEvent::Playlist),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlayerState {
    Stop,
    Pause,
    Play,
}

#[derive(Debug, Copy, Clone, FromStr)]
pub struct Seconds(pub f64);

#[derive(Debug)]
pub struct Status {
    pub state: PlayerState,
    pub song_id: Option<Id>,
    pub elapsed: Option<Seconds>,
    pub duration: Option<Seconds>,
    pub audio_format: Option<String>,
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
        let resp = self.command("addid", &[&uri.to_string()]).await??;
        resp.attributes.get("Id")
    }

    pub async fn deleteid(&mut self, id: &Id) -> Result<()> {
        self.command("deleteid", &[&id.0]).await??;
        Ok(())
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.command("clear", &[]).await??;
        Ok(())
    }

    pub async fn playlistinfo(&mut self) -> Result<Playlist> {
        let resp = self.command("playlistinfo", &[]).await??;

        let items = resp.attributes.split_at("file")
            .into_iter()
            .map(parse_playlist_item)
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

    pub async fn play(&mut self) -> Result<()> {
        self.command("play", &[]).await??;
        Ok(())
    }

    pub async fn playid(&mut self, id: Id) -> Result<()> {
        self.command("playid", &[&id.0]).await??;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.command("stop", &[]).await??;
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<()> {
        self.command("pause", &[]).await??;
        Ok(())
    }

    pub async fn next(&mut self) -> Result<()> {
        self.command("next", &[]).await??;
        Ok(())
    }

    pub async fn previous(&mut self) -> Result<()> {
        self.command("previous", &[]).await??;
        Ok(())
    }

    pub async fn status(&mut self) -> Result<Status> {
        let resp = self.command("status", &[]).await??;

        let state = match resp.attributes.get_one("state") {
            Some("play") => PlayerState::Play,
            Some("pause") => PlayerState::Pause,
            Some("stop") => PlayerState::Stop,
            Some(state) => bail!("unknown player state: {state}"),
            None => bail!("missing player state"),
        };

        Ok(Status {
            state,
            song_id: resp.attributes.get_opt("songid")?,
            elapsed: resp.attributes.get_opt("elapsed")?,
            duration: resp.attributes.get_opt("duration")?,
            audio_format: resp.attributes.get_opt("audio")?,
        })
    }

    pub async fn playlistid(&mut self, id: &Id) -> Result<PlaylistItem> {
        let resp = self.command("playlistid", &[&id.0]).await??;
        parse_playlist_item(resp.attributes)
    }
}

fn parse_playlist_item(attrs: Attributes) -> Result<PlaylistItem> {
    Ok(PlaylistItem {
        file: attrs.get("file")?,
        pos: attrs.get("Pos")?,
        id: attrs.get("Id")?,
        title: attrs.get_one("Title").map(str::to_owned),
        name: attrs.get_one("Name").map(str::to_owned),
    })
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
