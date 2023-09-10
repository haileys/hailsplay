use std::time::Duration;

use axum::TypedHeader;
use axum::extract::{State, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use headers::{ContentType, CacheControl};
use url::Url;

use crate::App;
use crate::config::Config;
use crate::db::{self, OptionalExt};
use crate::db::asset::{self, AssetId};
use crate::error::AppResult;

pub fn url(conn: &mut db::Connection, config: &Config, id: AssetId) -> Result<Url, db::Error> {
    let asset = asset::load_asset(conn, id)?;
    let path = format!("assets/{}/{}/{}", id.0, asset.digest.0, asset.filename);
    Ok(config.http.external_url.join(&path).unwrap())
}

pub async fn file(
    app: State<App>,
    Path((asset_id, _, _)): Path<(i64, String, String)>,
) -> AppResult<Result<impl IntoResponse, StatusCode>> {
    let result = app.database().diesel(|conn| {
        let asset = asset::load_asset(conn, AssetId(asset_id))?;
        let blob = asset::load_blob(conn, &asset.digest)?;
        Ok((asset, blob))
    }).await.optional()?;

    let Some((asset, blob)) = result else {
        return Ok(Err(StatusCode::NOT_FOUND));
    };

    let content_type = ContentType::from(asset.content_type());

    let cache_control = CacheControl::new()
        .with_public()
        // .with_immutable()
        .with_max_age(Duration::from_secs(315360000));

    let headers = (
        TypedHeader(content_type),
        TypedHeader(cache_control),
    );

    Ok(Ok((StatusCode::OK, headers, blob)))
}
