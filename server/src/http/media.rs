use std::{io::{self, SeekFrom}, ops::Bound, cmp};

use axum::{extract::{State, Path}, response::IntoResponse, body::StreamBody, TypedHeader};
use axum::http::StatusCode;
use bytes::{Bytes, BytesMut};
use headers::{ContentLength, AcceptRanges, Range, ContentRange};
use tokio::{fs::File, sync::{mpsc, watch}};
use tokio::io::{AsyncSeekExt, AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;
use futures::FutureExt;

use crate::ytdlp::{Progress, DownloadHandle};
use crate::error::AppError;
use crate::App;

const READ_BUFFER_SIZE: usize = 64 * 1024;

pub async fn stream(
    app: State<App>, 
    Path(media_id): Path<Uuid>,
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
    let stream = media_stream(&media.download, range).await?;

    let response = (
        StatusCode::OK,
        [("content-type", content_type_for_ext(&media.download.metadata.ext)),],
        stream.content_range.map(TypedHeader),
        TypedHeader(stream.content_length),
        TypedHeader(AcceptRanges::bytes()),
        StreamBody::new(stream.stream),
    ).into_response();
    Ok(response)
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

pub struct MediaStream {
    pub content_range: Option<ContentRange>,
    pub content_length: ContentLength,
    pub stream: ReceiverStream<io::Result<Bytes>>,
}

pub enum MediaStreamError {
    NotFound,
    RangeNotSatisfiable(ContentRange),
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
            MediaStreamError::RangeNotSatisfiable(content_range) => {
                (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    TypedHeader(content_range),
                    ()
                ).into_response()
            }
        }
    }
}

pub async fn media_stream(handle: &DownloadHandle, range: Option<Range>)
    -> Result<MediaStream, MediaStreamError>
{
    let file = File::open(handle.file.path()).await
        .map_err(MediaStreamError::Io)?;

    let progress = handle.progress.clone();
    let total_bytes = progress.borrow().total_bytes;

    // we don't support multiple byte ranges, only none or one
    let range = range.and_then(|header| header.iter().nth(0));

    // see if we need to seek to begin at all
    let seek_start = match range {
        Some((Bound::Included(seek_start), _)) => seek_start,
        _ => 0,
    };

    let seek_end_excl = match range {
        // HTTP byte ranges are inclusive, so we translate to exclusive by adding 1:
        Some((_, Bound::Included(end))) => end + 1,
        _ => total_bytes,
    };

    // if seek start is out of range error straight away
    if seek_start > total_bytes {
        let content_range = ContentRange::unsatisfied_bytes(total_bytes);
        return Err(MediaStreamError::RangeNotSatisfiable(content_range));
    }

    let seek = SeekInfo {
        start: seek_start,
        end: seek_end_excl,
    };

    let (tx, rx) = mpsc::channel(1);

    tokio::spawn(run_stream(file, seek, tx, progress));
    
    let content_range = range.map(|_| {
        ContentRange::bytes(seek_start..seek_end_excl, total_bytes).unwrap()
    });

    let stream = MediaStream {
        content_range,
        content_length: ContentLength(total_bytes),
        stream: tokio_stream::wrappers::ReceiverStream::new(rx),
    };
    
    Ok(stream)
}

struct SeekInfo {
    pub start: u64,
    pub end: u64,
}

async fn run_stream(
    mut file: File,
    seek: SeekInfo,
    tx: mpsc::Sender<io::Result<Bytes>>,
    mut progress: watch::Receiver<Progress>,
) {
    log::debug!("run_stream start, waiting for stream catch up: seek.start = {}", seek.start);

    // wait for stream to catch up with where we want to seek to
    let _ = progress.wait_for(|p| p.downloaded_bytes >= seek.start).await;

    log::debug!("stream caught up, seeking");

    // seek the file
    match file.seek(SeekFrom::Start(seek.start)).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("io error seeking in media_stream: {e:?}");
            let _ = tx.send(Err(e));
            return;
        }
    }

    log::debug!("seeked!");

    let mut pos = seek.start;

    loop {
        let remaining = seek.end - pos;
        let cap = cmp::min(READ_BUFFER_SIZE, remaining as usize);
        let mut buf = BytesMut::with_capacity(cap);

        let target = pos + READ_BUFFER_SIZE as u64;

        let read_fut = progress
            // wait for a full buffer to be ready before reading
            .wait_for(|p| p.complete() || p.downloaded_bytes >= target)
            .then(|_| file.read_buf(&mut buf));
        futures::pin_mut!(read_fut);

        let closed_fut = tx.closed().map(|()| {
            // fake an EOF if rx hung up on us
            Ok(0)
        });
        futures::pin_mut!(closed_fut);
        
        let result = futures::future::select(read_fut, closed_fut)
            .await
            .factor_first()
            .0;

        match result {
            Ok(0) => { break; }
            Ok(n) => {
                // no need to check err here, we automatically get cancelled
                // on rx hangup
                let _: Result<_, _> = tx.send(Ok(buf.freeze())).await;
                pos += n as u64;
            }
            Err(e) => {
                log::error!("io error in media_stream: {e:?}");
                let _ = tx.send(Err(e));
                break;
            }
        }
    }
}
