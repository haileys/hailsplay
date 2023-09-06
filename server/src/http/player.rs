use serde::{Serialize, Deserialize};
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct TrackInfo {
    pub image_url: Url,
    pub primary_label: String,
    pub secondary_label: Option<String>,
    pub stream: bool,
}
