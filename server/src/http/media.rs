use std::io;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::cmp;

use axum::{extract::{State, Path}, response::IntoResponse, TypedHeader};
use axum::http::StatusCode;
use derive_more::From;
use headers::Range;
use pin_project::pin_project;
use tokio::{fs::File, io::ReadBuf};
use tokio::sync::watch::Receiver;
use tokio::io::AsyncRead;
use tokio_stream::wrappers::WatchStream;
use futures::{StreamExt, stream::Fuse, ready};

use crate::{App, MediaId};
use crate::error::AppError;
use crate::ytdlp::{Progress, DownloadHandle};

use axum_range::{Ranged, RangeNotSatisfiable, RangeBody, AsyncSeekStart};

pub async fn stream(
    app: State<App>,
    Path(media_id): Path<MediaId>,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, MediaStreamError> {
    let media = {
        let state = app.0.0.state.lock().unwrap();
        match state.media.get(&media_id) {
            Some(media) => media.clone(),
            None => {
                return Err(MediaStreamError::NotFound);
            }
        }
    };

    log::info!("Serving stream title={:?} id={:?}", media.download.metadata.title, media_id);

    let range = range.map(|header| header.0);

    let headers = [
        ("content-type", content_type_for_ext(&media.download.metadata.ext)),
    ];

    let stream = media_stream(&media.download, range).await?;

    Ok((headers, stream).into_response())
}

fn content_type_for_ext(ext: &str) -> &'static str {
    match ext {
        "aac" => "audio/aac",
        "flac" => "audio/x-flac",
        "m4a" => "audio/mp4",
        "mka" => "audio/x-matroska",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        _ => "application/octet-stream",
    }
}

#[derive(From)]
pub enum MediaStreamError {
    NotFound,
    RangeNotSatisfiable(RangeNotSatisfiable),
    Io(io::Error),
}

impl IntoResponse for MediaStreamError {
    fn into_response(self) -> axum::response::Response {
        match self {
            MediaStreamError::Io(e) => AppError::from(e).into_response(),
            MediaStreamError::NotFound => {
                (
                    StatusCode::NOT_FOUND,
                    "not found"
                ).into_response()
            }
            MediaStreamError::RangeNotSatisfiable(response) => {
                response.into_response()
            }
        }
    }
}

async fn media_stream(handle: &DownloadHandle, range: Option<Range>)
    -> Result<Ranged<StreamingDownload>, io::Error>
{
    let file = File::open(handle.file.path()).await?;
    let progress = handle.progress.clone();
    let body = StreamingDownload::new(file, progress);
    Ok(Ranged::new(range, body))
}

#[pin_project]
struct StreamingDownload {
    #[pin]
    file: File,
    seek: Seek,
    progress: PollProgress,
}

impl StreamingDownload {
    pub fn new(file: File, progress: Receiver<Progress>) -> Self {
        let progress = PollProgress::new(progress);
        let seek = Seek::At(0);
        StreamingDownload { file, seek, progress }
    }

    fn total_bytes(&self) -> u64 {
        self.progress.current.total_bytes
    }
}

enum Seek {
    At(u64),
    SeekTo(u64),
    Seeking(u64),
}

impl RangeBody for StreamingDownload {
    fn byte_size(&self) -> u64 {
        self.total_bytes()
    }
}

impl AsyncSeekStart for StreamingDownload {
    fn start_seek(self: Pin<&mut Self>, position: u64) -> io::Result<()> {
        let seek_to = cmp::min(self.total_bytes(), position);
        let this = self.project();

        if let Seek::At(_) = this.seek {
            *this.seek = Seek::SeekTo(seek_to);
            Ok(())
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "start seek while already seeking",
            ));
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();

        if let Seek::At(_) = this.seek {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "poll complete while not seeking",
            )));
        }

        if let Seek::SeekTo(pos) = *this.seek {
            ready!(this.progress.poll_ready(cx, pos));
            AsyncSeekStart::start_seek(this.file.as_mut(), pos)?;
            *this.seek = Seek::Seeking(pos);
        }

        if let Seek::Seeking(pos) = *this.seek {
            ready!(AsyncSeekStart::poll_complete(this.file.as_mut(), cx))?;
            *this.seek = Seek::At(pos);
            return Poll::Ready(Ok(()));
        }

        unreachable!();
    }
}

impl AsyncRead for StreamingDownload {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>)
        -> Poll<io::Result<()>>
    {
        let mut this = self.project();

        let Seek::At(pos) = *this.seek else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "read while seeking",
            )));
        };

        ready!(this.progress.poll_ready(cx, pos));

        let filled_before_read = buf.filled().len();
        let () = ready!(AsyncRead::poll_read(this.file.as_mut(), cx, buf))?;
        let filled_after_read = buf.filled().len();

        let bytes_read = u64::try_from(filled_after_read - filled_before_read)
            .expect("conversion from usize to u64 to always succeed");

        *this.seek = Seek::At(pos + bytes_read);

        Poll::Ready(Ok(()))
    }
}

struct PollProgress {
    current: Progress,
    watch: Fuse<WatchStream<Progress>>,
}

impl PollProgress {
    pub fn new(watch: Receiver<Progress>) -> Self {
        let current = watch.borrow().clone();
        let watch = WatchStream::new(watch).fuse();
        PollProgress { current, watch }
    }

    pub fn poll_ready(&mut self, cx: &mut Context<'_>, position: u64) -> Poll<()> {
        loop {
            if self.current.complete() {
                return Poll::Ready(());
            }

            if position < self.current.downloaded_bytes {
                return Poll::Ready(());
            }

            match ready!(self.watch.poll_next_unpin(cx)) {
                Some(progress) => { self.current = progress; }
                None => { return Poll::Ready(()); }
            }
        }
    }
}
