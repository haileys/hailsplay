use axum::extract::Query;
use axum::Json;
use hailsplay_protocol::Metadata;
use serde::Deserialize;
use url::Url;

use crate::error::AppResult;
use crate::ytdlp;

#[derive(Deserialize)]
pub struct MetadataParams {
    url: Url,
}

pub async fn metadata(params: Query<MetadataParams>) -> AppResult<Json<Metadata>> {
    log::info!("Fetching metadata for {}", params.url);
    let metadata = request_metadata(&params.url).await?;
    Ok(Json(metadata))
}

async fn request_metadata(url: &Url) -> anyhow::Result<Metadata> {
    let metadata = ytdlp::fetch_metadata(url).await?;

    Ok(Metadata {
        title: metadata.title.unwrap_or_else(|| url.to_string()),
        artist: metadata.uploader,
        thumbnail: metadata.thumbnail,
    })
}
