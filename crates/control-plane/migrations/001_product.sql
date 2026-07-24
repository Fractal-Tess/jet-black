CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    disabled_at_ms INTEGER,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE password_credentials (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    csrf_hash BLOB NOT NULL CHECK (length(csrf_hash) = 32),
    method TEXT NOT NULL CHECK (method IN ('password', 'launch_token')),
    issued_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    last_seen_at_ms INTEGER,
    revoked_at_ms INTEGER,
    replaced_by_id TEXT REFERENCES sessions(id),
    CHECK (expires_at_ms > issued_at_ms)
) STRICT;
CREATE INDEX sessions_active_idx
    ON sessions(token_hash, revoked_at_ms, expires_at_ms);
CREATE INDEX sessions_user_idx
    ON sessions(user_id, revoked_at_ms, expires_at_ms);

CREATE TABLE launch_tokens (
    id TEXT PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at_ms INTEGER NOT NULL,
    consumed_at_ms INTEGER
) STRICT;

CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL COLLATE NOCASE UNIQUE,
    name TEXT NOT NULL,
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE workspace_members (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'guest')),
    joined_at_ms INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, user_id)
) STRICT;
CREATE INDEX workspace_members_user_idx ON workspace_members(user_id, workspace_id);

CREATE TABLE workspace_invites (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    email TEXT NOT NULL COLLATE NOCASE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'member', 'guest')),
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    invited_by_id TEXT NOT NULL REFERENCES users(id),
    expires_at_ms INTEGER NOT NULL,
    accepted_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    identifier TEXT NOT NULL COLLATE NOCASE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    repository_identity TEXT,
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    archived_at_ms INTEGER,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (workspace_id, identifier)
) STRICT;
CREATE INDEX projects_workspace_idx ON projects(workspace_id, archived_at_ms);

CREATE TABLE project_members (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'member', 'guest')),
    joined_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, user_id)
) STRICT;

CREATE TABLE workflow_states (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    state_group TEXT NOT NULL CHECK (
        state_group IN ('backlog', 'unstarted', 'started', 'completed', 'cancelled')
    ),
    color TEXT NOT NULL,
    position REAL NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (project_id, name)
) STRICT;

CREATE TABLE labels (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (project_id, name)
) STRICT;

CREATE TABLE sprints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    starts_at_ms INTEGER,
    ends_at_ms INTEGER,
    status TEXT NOT NULL CHECK (status IN ('draft', 'active', 'completed', 'cancelled')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE modules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('backlog', 'planned', 'in_progress', 'paused', 'completed', 'cancelled')),
    target_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE tickets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    state_id TEXT REFERENCES workflow_states(id) ON DELETE SET NULL,
    priority TEXT NOT NULL DEFAULT 'none' CHECK (
        priority IN ('none', 'urgent', 'high', 'medium', 'low')
    ),
    assignee_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    parent_id TEXT REFERENCES tickets(id) ON DELETE SET NULL,
    sprint_id TEXT REFERENCES sprints(id) ON DELETE SET NULL,
    estimate INTEGER CHECK (estimate IS NULL OR estimate >= 0),
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    archived_at_ms INTEGER,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (project_id, sequence_number)
) STRICT;
CREATE INDEX tickets_project_idx ON tickets(project_id, archived_at_ms, updated_at_ms);
CREATE INDEX tickets_assignee_idx ON tickets(assignee_id, archived_at_ms);

CREATE TABLE ticket_labels (
    ticket_id TEXT NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (ticket_id, label_id)
) STRICT;

CREATE TABLE ticket_modules (
    ticket_id TEXT NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    module_id TEXT NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    PRIMARY KEY (ticket_id, module_id)
) STRICT;

CREATE TABLE ticket_comments (
    id TEXT PRIMARY KEY,
    ticket_id TEXT NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    author_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    body TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    deleted_at_ms INTEGER,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE pages (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    created_by_id TEXT NOT NULL REFERENCES users(id),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    archived_at_ms INTEGER,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE intake_items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    submitter_email TEXT COLLATE NOCASE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'declined')),
    ticket_id TEXT REFERENCES tickets(id) ON DELETE SET NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
) STRICT;

CREATE TABLE attachments (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    ticket_id TEXT REFERENCES tickets(id) ON DELETE CASCADE,
    comment_id TEXT REFERENCES ticket_comments(id) ON DELETE CASCADE,
    digest TEXT NOT NULL,
    file_name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    byte_length INTEGER NOT NULL CHECK (byte_length >= 0),
    storage_path TEXT NOT NULL,
    uploaded_by_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at_ms INTEGER NOT NULL,
    CHECK (ticket_id IS NOT NULL OR comment_id IS NOT NULL),
    UNIQUE (workspace_id, digest, storage_path)
) STRICT;

CREATE TABLE feature_settings (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    feature TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    updated_by_id TEXT NOT NULL REFERENCES users(id),
    updated_at_ms INTEGER NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    PRIMARY KEY (workspace_id, feature)
) STRICT;

CREATE TABLE product_events (
    cursor INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    aggregate_kind TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    aggregate_version INTEGER NOT NULL,
    event_kind TEXT NOT NULL,
    actor_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    idempotency_key TEXT,
    body TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    UNIQUE (workspace_id, idempotency_key)
) STRICT;
CREATE INDEX product_events_replay_idx ON product_events(workspace_id, cursor);

CREATE TABLE quota_usage (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    attachment_bytes INTEGER NOT NULL DEFAULT 0 CHECK (attachment_bytes >= 0),
    tickets_count INTEGER NOT NULL DEFAULT 0 CHECK (tickets_count >= 0),
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TRIGGER tickets_quota_insert
AFTER INSERT ON tickets
BEGIN
    INSERT INTO quota_usage (workspace_id, tickets_count, updated_at_ms)
    SELECT workspace_id, 1, CAST(unixepoch('subsec') * 1000 AS INTEGER)
    FROM projects WHERE id = NEW.project_id
    ON CONFLICT(workspace_id) DO UPDATE SET
        tickets_count = tickets_count + 1,
        updated_at_ms = excluded.updated_at_ms;
END;

CREATE TRIGGER tickets_quota_delete
AFTER DELETE ON tickets
BEGIN
    UPDATE quota_usage SET
        tickets_count = MAX(0, tickets_count - 1),
        updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
    WHERE workspace_id = (SELECT workspace_id FROM projects WHERE id = OLD.project_id);
END;

CREATE TRIGGER attachments_quota_insert
AFTER INSERT ON attachments
BEGIN
    INSERT INTO quota_usage (workspace_id, attachment_bytes, updated_at_ms)
    VALUES (
        NEW.workspace_id,
        NEW.byte_length,
        CAST(unixepoch('subsec') * 1000 AS INTEGER)
    )
    ON CONFLICT(workspace_id) DO UPDATE SET
        attachment_bytes = attachment_bytes + NEW.byte_length,
        updated_at_ms = excluded.updated_at_ms;
END;

CREATE TRIGGER attachments_quota_delete
AFTER DELETE ON attachments
BEGIN
    UPDATE quota_usage SET
        attachment_bytes = MAX(0, attachment_bytes - OLD.byte_length),
        updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
    WHERE workspace_id = OLD.workspace_id;
END;
