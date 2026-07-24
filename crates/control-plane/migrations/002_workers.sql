CREATE TABLE worker_devices (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash BLOB NOT NULL UNIQUE,
    protocol_version TEXT NOT NULL,
    capabilities_json TEXT NOT NULL,
    repositories_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('online', 'offline', 'draining', 'revoked')),
    last_seen_at_ms INTEGER,
    revoked_at_ms INTEGER,
    fencing_epoch INTEGER NOT NULL DEFAULT 0,
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE INDEX worker_devices_workspace_status
    ON worker_devices(workspace_id, status, updated_at_ms);

CREATE TABLE worker_assignments (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    worker_id TEXT NOT NULL REFERENCES worker_devices(id) ON DELETE CASCADE,
    ticket_id TEXT NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    repository_identity TEXT NOT NULL,
    provider TEXT NOT NULL,
    command_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'leased', 'running', 'awaiting_approval', 'completed', 'failed', 'cancelled')
    ),
    fencing_epoch INTEGER NOT NULL,
    lease_expires_at_ms INTEGER,
    next_event_sequence INTEGER NOT NULL DEFAULT 1,
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE(worker_id, fencing_epoch)
) STRICT;

CREATE INDEX worker_assignments_worker_status
    ON worker_assignments(worker_id, status, created_at_ms);

CREATE TABLE worker_outbox (
    assignment_id TEXT NOT NULL REFERENCES worker_assignments(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    fencing_epoch INTEGER NOT NULL,
    event_kind TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY(assignment_id, sequence)
) STRICT;
