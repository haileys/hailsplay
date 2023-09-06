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
    position: number,
    track: TrackInfo,
};

export type TrackInfo = {
    imageUrl: Url,
    primaryLabel: string | null,
    secondaryLabel: string | null,
};

export type PlayerStatus = {
    playing: boolean,
    position: PlayPosition,
    track: TrackId,
};

export type PlayPosition =
    | { "t": "streaming" }
    | { "t": "elapsed", "time": number, "duration": number }
    ;
