/* tslint:disable */
/* eslint-disable */
export interface RadioStation {
    name: string;
    iconUrl: Url;
    streamUrl: Url;
}

export interface TuneParams {
    url: Url;
}

export interface AddResponse {
    mpd_id: TrackId;
}

export interface AddParams {
    url: Url;
}

export type PlayPosition = { t: "streaming" } | { t: "elapsed"; time: number; duration: number };

export type PlayState = "stopped" | "loading" | "playing";

export interface PlayerStatus {
    track: TrackId | null;
    state: PlayState;
    position: PlayPosition | null;
}

export interface Metadata {
    title: string;
    artist: string | null;
    thumbnail: Url | null;
}

export interface TrackInfo {
    imageUrl: Url | null;
    primaryLabel: string;
    secondaryLabel: string | null;
}

export type TrackId = string;

export interface Queue {
    items: QueueItem[];
}

export interface QueueItem {
    id: TrackId;
    position: number;
    track: TrackInfo;
}

export type ServerMessage = { t: "queue"; queue: Queue } | { t: "track-change"; track: TrackInfo | null } | { t: "player"; player: PlayerStatus };

export interface ClientMessage {
    t: "ClientMessage";
}

export type Url = string;

