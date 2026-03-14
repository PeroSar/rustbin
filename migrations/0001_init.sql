CREATE TABLE IF NOT EXISTS pastes (
    id TEXT PRIMARY KEY,
    language TEXT,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_pastes_expiry
ON pastes (expires_at);
