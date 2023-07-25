use crate::db::Pool;
use crate::db::types;
use crate::ytdlp::Thumbnail;
use chrono::Utc;
use sqlx::Row;
use url::Url;

#[derive(sqlx::FromRow)]
struct Metadata {
    pub id: i64,
    pub url: types::Url,
    pub scraped_at: types::Timestamp,
    pub title: String,
    pub artist: Option<String>,
    pub thumbnail_url: Option<types::Url>,
}

pub async fn fetch(pool: &Pool, url: &Url) -> anyhow::Result<Metadata> {
    let meta = sqlx::query!("SELECT * FROM metadata LIMIT 1")
        .try_map(|row| {
            let url: types::Url = row.url.try_into()?;
            let scraped_at: types::Timestamp = row.url.try_into()?;
            let thumbnail_url: types::URL = row.url.try_into()?;
            Ok(Metadata {
                id: row.id,
                url: url,
                scraped_at: scraped_at,
                title: row.title,
                artist: row.artist,
                thumbnail_url: thumbnail_url,
            })
        })
        .fetch_optional(pool)
        .await?;

    if let Some(meta) = meta {
        return Ok(meta);
    }

    log::info!("Fetching metadata from {url}");

    let meta = match crate::ytdlp::fetch_metadata(url).await {
        Ok(meta) => meta,
        Err(e) => {
            log::warn!("Error fetching metadata from {url}: {e:?}");
            return Err(e);
        }
    };

    let now = Utc::now();

    let thumbnail = meta.thumbnails.into_iter()
        .max_by_key(|th| th.width + th.height);
    
    let thumbnail_url = thumbnail.map(|th| types::Url(th.url));

    let id = sqlx::query!(r"
        INSERT INTO metadata (url, scraped_at, title, artist, thumbnail_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
    ", types::Url(url.to_owned()), now, meta.title, meta.uploader, thumbnail_url)
        .fetch_one(pool)
        .await?;

    Ok(Metadata {
        id,
        url: url.clone(),
        scraped_at: now,
        title: meta.title,
        artist: Some(meta.uploader),
        thumbnail_url: thumbnail_url,
    })
}
