CREATE TABLE archived_media (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    canonical_url TEXT NOT NULL,
    archived_at TEXT NOT NULL,
    stream_uuid TEXT NOT NULL,
    thumbnail_id INT NULL,
    metadata TEXT NOT NULL,
    FOREIGN KEY (thumbnail_id) REFERENCES assets(id)
);
