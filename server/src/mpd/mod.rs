pub mod protocol;

use anyhow::{Result, bail};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use url::Url;

use crate::config;

use self::protocol::{MpdReader, MpdWriter};

pub struct Mpd {
    inner: Mutex<Inner>
}

struct Inner {
    reader: MpdReader<OwnedReadHalf>,
    writer: MpdWriter<OwnedWriteHalf>,
}

#[derive(Serialize, Deserialize)]
pub struct Id(String);

impl Mpd {
    pub async fn connect(config: &config::Mpd) -> Result<Mpd> {
        let sock = UnixStream::connect(&config.socket).await?;
        let (rx, tx) = sock.into_split();
        let (reader, proto) = MpdReader::open(rx).await?;
        let writer = MpdWriter::open(tx);
        let inner = Mutex::new(Inner { reader, writer });
        log::info!("Connected to mpd at {}, protocol version {}",
            config.socket.display(), proto.version);
        Ok(Mpd { inner })
    }

    pub async fn addid(&self, uri: &Url) -> Result<Id> {
        let mut inner = self.inner.lock().await;
        inner.writer.send_command("addid", &[&uri.to_string()]).await?;
        let mut resp = inner.reader.read_response().await??;
        let Some(id) = resp.attributes.remove("Id") else {
            bail!("no Id attribute in addid response");
        };
        Ok(Id(id))
    }
}
