CREATE TABLE pending_accounts (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    requested_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE instance_admins (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE desktop_pairing_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    device_name TEXT NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    consumed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE desktop_devices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL UNIQUE REFERENCES sessions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    last_seen_at_ms INTEGER,
    revoked_at_ms INTEGER
) STRICT;
CREATE INDEX desktop_devices_user_idx
    ON desktop_devices(user_id, revoked_at_ms, created_at_ms);

ALTER TABLE projects ADD COLUMN repository_kind TEXT
    CHECK (repository_kind IS NULL OR repository_kind IN ('local', 'remote'));
ALTER TABLE projects ADD COLUMN repository_location TEXT;
ALTER TABLE projects ADD COLUMN repository_origin TEXT;
