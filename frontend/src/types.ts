export type Id = string;
export type Url = string;

export type ServerMessage =
    | { "t": "queue", queue: Queue }
    | { "t": "track-change", track: TrackInfo | null }
    | { "t": "player", player: PlayerStatus }
    ;

export type TrackId = string;

export type Metadata = {
    title: string,
    artist: string | null,
    thumbnail: Url | null,
};

export type RadioStation = {
    name: string,
    icon_url: Url,
    stream_url: Url,
};

export type Queue = {
    items: QueueItem[],
};

export type QueueItem = {
    id: TrackId,
    position: number,
    track: TrackInfo,
};

export type TrackInfo = {
    imageUrl: Url,
    primaryLabel: string | null,
    secondaryLabel: string | null,
};

export type PlayerStatus = {
    track: TrackId,
    state: PlayState,
    position: PlayPosition,
};

export type PlayState =
    | "stopped"
    | "loading"
    | "playing"
    ;

export type PlayPosition =
    | { "t": "streaming" }
    | { "t": "elapsed", "time": number, "duration": number }
    ;
