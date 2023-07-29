use std::collections::HashMap;
use std::fmt::Display;

use anyhow::{Context, bail};
use tokio::io::{BufReader, AsyncRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct MpdReader<R> {
    r: BufReader<R>,
}

pub struct Protocol {
    pub version: String,
}

impl<R: AsyncRead + Unpin> MpdReader<R> {
    pub async fn open(r: R) -> anyhow::Result<(Self, Protocol)> {
        let mut r = BufReader::new(r);

        let mut line = String::new();
        r.read_line(&mut line).await?;

        let Some(proto) = prefixed("OK MPD ", &line) else {
            bail!("unexpected initial line from mpd: {line:?}")
        };

        let reader = MpdReader { r };
        let protocol = Protocol { version: proto.to_string() };

        Ok((reader, protocol))
    }

    pub async fn read_response(&mut self) -> anyhow::Result<Response> {
        let mut attributes = HashMap::new();
        let mut binary = None;

        let mut buff = String::new();
        loop {
            buff.truncate(0);
            self.r.read_line(&mut buff).await?;

            if buff == "OK\n" {
                return Ok(Ok(Ok {
                    attributes,
                    binary,
                }));
            }

            if let Some(line) = prefixed("ACK ", &buff) {
                let line = line.to_string();
                return Ok(Err(Error { line }));
            }
            
            if let Some(len) = prefixed("binary: ", &buff) {
                let len = len.parse().context("parsing length of binary data")?;
                let mut bin = Vec::with_capacity(len);
                self.r.read_exact(&mut bin).await.context("reading binary data")?;
                let nl = self.r.read_u8().await.context("reading binary trailing newline")?;
                if nl != b'\n' {
                    bail!("binary data did not end with trailing newline");
                }
                binary = Some(bin);
                continue;
            }

            if let Some((key, value)) = buff.split_once(": ") {
                attributes.insert(key.to_string(), value.to_string());
            } else {
                bail!("unrecognised response line from mpd: {buff:?}");
            }
        }
    }
}

fn prefixed<'a>(prefix: &str, s: &'a str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(&s[prefix.len()..].trim_end())
    } else {
        None
    }
}

pub type Response = Result<Ok, Error>;

#[derive(Debug)]
pub struct Error {
    pub line: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mpd error response: {}", self.line)
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Ok {
    pub attributes: HashMap<String, String>,
    pub binary: Option<Vec<u8>>,
}

pub struct MpdWriter<W> {
    w: W,
}

impl<W: AsyncWrite + Unpin> MpdWriter<W> {
    pub fn open(w: W) -> Self {
        MpdWriter { w }
    }

    pub async fn send_command(&mut self, cmd: &str, args: &[&str]) -> anyhow::Result<()> {
        let mut line = cmd.to_string();
        for arg in args {
            line.push(' ');
            line.push('"');
            for c in arg.chars() {
                match c {
                    '"' | '\\' => {
                        line.push('\\');
                        line.push(c);
                    }
                    '\n' => {
                        bail!("newline in command argument");
                    }
                    _ => {
                        line.push(c);
                    }
                }
            }
            line.push('"');
        }

        self.w.write_all(line.as_bytes()).await?;
        Ok(())
    }
}
