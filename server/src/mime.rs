use std::path::Path;

use mime::Mime;

const FALLBACK: Mime = mime::APPLICATION_OCTET_STREAM;

pub fn from_path(path: &Path) -> Mime {
    path.extension()
        .and_then(|os_str| os_str.to_str())
        .map(from_extension)
        .unwrap_or(FALLBACK)
}

pub fn from_extension(ext: &str) -> Mime {
    raw_from_ext(ext).parse().unwrap()
}

fn raw_from_ext(ext: &str) -> &'static str {
    match ext {
        "aac" => "audio/aac",
        "flac" => "audio/x-flac",
        "gif" => "image/gif",
        "jpg" => "image/jpg",
        "m4a" => "audio/mp4",
        "mka" => "audio/x-matroska",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "opus" => "audio/ogg",
        "png" => "image/png",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        "webp" => "image/webp",
        _ => FALLBACK.as_ref(),
    }
}
