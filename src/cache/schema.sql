-- Nimbus Cache Database Schema

CREATE TABLE IF NOT EXISTS resources (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    region TEXT NOT NULL,
    data TEXT NOT NULL,
    cached_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_provider ON resources(provider);
CREATE INDEX IF NOT EXISTS idx_type ON resources(resource_type);
CREATE INDEX IF NOT EXISTS idx_region ON resources(region);
CREATE INDEX IF NOT EXISTS idx_cached_at ON resources(cached_at);

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO metadata (key, value, updated_at) VALUES ('version', '1', 0);