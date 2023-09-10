use std::io;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::cmp;

use axum::response::Response;
use axum::{extract::{State, Path}, response::IntoResponse, TypedHeader};
use axum::http::StatusCode;
use derive_more::From;
use headers::{Range, ContentType};
use pin_project::pin_project;
use tokio::{fs::File, io::ReadBuf};
use tokio::sync::watch::Receiver;
use tokio::io::AsyncRead;
use tokio_stream::wrappers::WatchStream;
use futures::{StreamExt, stream::Fuse, ready};

use crate::{App, Config};
use crate::api::archive::{MediaStreamId, RecordKind, MetadataParseError};
use crate::ytdlp::Progress;
use crate::error::AppError;

use axum_range::{Ranged, RangeNotSatisfiable, RangeBody, AsyncSeekStart, KnownSize};

pub async fn stream(
    app: State<App>,
    Path(media_id): Path<MediaStreamId>,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, MediaStreamError> {
    let range = range.map(|TypedHeader(range)| range);

    let media = match app.archive().load(media_id).await {
        Ok(Some(media)) => media,
        Ok(None) => { return Err(MediaStreamError::NotFound); }
        Err(e) => { return Err(MediaStreamError::Database(e)); }
    };

    let metadata = media.parse_metadata()
        .map_err(MediaStreamError::ParseMetadata)?;

    log::info!("Serving stream title={:?} id={:?}", metadata.title, media_id);

    let content_type = TypedHeader(ContentType::from(media.content_type()));

    let response = ranged_response(&media, range, app.config()).await?;

    Ok((content_type, response).into_response())
}

async fn ranged_response(media: &RecordKind, range: Option<Range>, config: &Config) -> io::Result<Response> {
    let path = media.disk_path(config);
    let file = tokio::fs::File::open(path).await?;

    match media {
        RecordKind::Archive(_, _) => {
            let body = KnownSize::file(file).await?;
            Ok(Ranged::new(range, body).into_response())
        }
        RecordKind::Memory(record) => {
            let progress = record.download.progress.clone();
            let body = StreamingDownload::new(file, progress);
            Ok(Ranged::new(range, body).into_response())
        }
    }
}

#[derive(From)]
pub enum MediaStreamError {
    NotFound,
    RangeNotSatisfiable(RangeNotSatisfiable),
    Io(io::Error),
    Database(rusqlite::Error),
    ParseMetadata(MetadataParseError)
}

impl IntoResponse for MediaStreamError {
    fn into_response(self) -> axum::response::Response {
        match self {
            MediaStreamError::Io(e) => AppError::from(e).into_response(),
            MediaStreamError::Database(e) => AppError::from(e).into_response(),
            MediaStreamError::ParseMetadata(e) => AppError::from(e).into_response(),
            MediaStreamError::NotFound => StatusCode::NOT_FOUND.into_response(),
            MediaStreamError::RangeNotSatisfiable(response) => response.into_response(),
        }
    }
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
