#[macro_use] mod macros;

use serde::{Serialize, Deserialize};
use url::Url;

protocol! {
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "t", rename_all = "kebab-case")]
    pub struct ClientMessage {}

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "t", rename_all = "kebab-case")]
    pub enum ServerMessage {
        Queue { queue: Queue },
        TrackChange { track: Option<TrackInfo> },
        Player { player: PlayerStatus },
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct QueueItem {
        pub id: TrackId,
        pub position: i64,
        pub track: TrackInfo
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Queue {
        pub items: Vec<QueueItem>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    pub struct TrackId(pub String);

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrackInfo {
        pub image_url: Option<Url>,
        pub primary_label: String,
        pub secondary_label: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct Metadata {
        pub title: String,
        pub artist: Option<String>,
        pub thumbnail: Option<Url>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PlayerStatus {
        pub track: Option<TrackId>,
        pub state: PlayState,
        pub position: Option<PlayPosition>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PlayState {
        Stopped,
        Loading,
        Playing,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "t", rename_all = "kebab-case")]
    pub enum PlayPosition {
        Streaming,
        Elapsed { time: f64, duration: f64 },
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AddParams {
        pub url: Url,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AddResponse {
        pub mpd_id: TrackId,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TuneParams {
        pub url: Url,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RadioStation {
        pub name: String,
        pub icon_url: Url,
        pub stream_url: Url,
    }

}
