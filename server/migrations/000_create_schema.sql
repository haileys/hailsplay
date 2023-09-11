CREATE TABLE assets (
    id INTEGER NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    digest_sha256 TEXT NOT NULL,
    FOREIGN KEY (digest_sha256) REFERENCES asset_blobs (digest_sha256)
);

CREATE TABLE asset_blobs (
    digest_sha256 TEXT NOT NULL PRIMARY KEY,
    blob BLOB NOT NULL
);

CREATE TABLE radio_stations (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    icon_id INTEGER NOT NULL,
    stream_url TEXT NOT NULL,
    FOREIGN KEY (icon_id) REFERENCES assets (id)
);
