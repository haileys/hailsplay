use uuid::Uuid;
use url::Url;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    MetadataRequest(MetadataRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataRequest {
    pub request_id: Uuid,
    pub url: Url,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Playlist(Playlist),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataResponse {
    pub request_id: Uuid,
    pub result: Result<Metadata, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub thumbnail: Option<Url>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Playlist {
    pub items: Vec<PlaylistItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaylistItemId(pub String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaylistItem {
    pub id: PlaylistItemId,
    pub meta: Metadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddParams {
    pub url: Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddResponse {
    pub mpd_id: PlaylistItemId,
}
