use axum::extract::{State, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use headers::{HeaderMap, HeaderValue};
use rusqlite::{Connection, OptionalExtension};
use url::Url;

use crate::App;
use crate::config::Config;
use crate::db::asset::{self, AssetId};
use crate::error::AppResult;

pub fn url(conn: &mut Connection, config: &Config, id: AssetId) -> Result<Url, rusqlite::Error> {
    let asset = asset::load_asset(conn, id)?;
    let path = format!("assets/{}/{}/{}", id.0, asset.digest.0, asset.filename);
    Ok(config.http.external_url.join(&path).unwrap())
}

pub async fn file(
    app: State<App>,
    Path((asset_id, _, _)): Path<(i64, String, String)>,
) -> AppResult<Result<impl IntoResponse, StatusCode>> {
    let result = app.use_database(|conn| {
        let asset = asset::load_asset(conn, AssetId(asset_id))?;
        let blob = asset::load_blob(conn, &asset.digest)?;
        Ok((asset, blob))
    }).await.optional()?;

    let Some((asset, blob)) = result else {
        return Ok(Err(StatusCode::NOT_FOUND));
    };

    let content_type = HeaderValue::from_str(&asset.content_type)
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));

    let mut headers = HeaderMap::new();
    headers.insert("content-type", content_type);
    headers.insert("cache-control", HeaderValue::from_static("public, max-age=315360000"));

    Ok(Ok((StatusCode::OK, headers, blob)))
}
