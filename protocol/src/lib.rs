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
    MetadataResponse(MetadataResponse),
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
