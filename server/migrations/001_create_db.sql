CREATE TABLE metadata (
    id INTEGER NOT NULL PRIMARY KEY,
    url TEXT NOT NULL,
    scraped_at TEXT NOT NULL,
    title TEXT NOT NULL,
    artist TEXT,
    thumbnail_url TEXT
);

CREATE INDEX idx_metadata_url ON metadata (url, scraped_at);
CREATE INDEX idx_metadata_scraped_at ON metadata (scraped_at);
