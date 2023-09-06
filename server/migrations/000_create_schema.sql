CREATE TABLE assets (
    id INTEGER PRIMARY KEY,
    content_type TEXT NOT NULL,
    sha256 TEXT NOT NULL UNIQUE,
    blob BLOB NOT NULL
);

CREATE TABLE radio_stations (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    icon_asset_id INTEGER NOT NULL,
    stream_url TEXT NOT NULL
);
