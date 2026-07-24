# Operations

## Data

Each instance owns one app-data directory and one `jet-black.sqlite3`. SQLite is
opened in WAL mode with foreign keys, busy timeouts, explicit product and
execution migration ledgers, and a single product-instance lock. Worktrees and
artifacts live below the same canonical app-data root.

Never share or replicate the SQLite file between processes. To back up an
instance, use the control-plane online backup operation or stop the process and
copy the entire data directory.

## Public server

Set an explicit public origin when binding beyond loopback:

```bash
JET_BLACK_PROFILE=server \
JET_BLACK_BIND=0.0.0.0:4317 \
JET_BLACK_PUBLIC_ORIGIN=https://jet-black.example.com \
JET_BLACK_DATA_DIR=/var/lib/jet-black \
./jet-black
```

Terminate TLS at a reverse proxy and preserve WebSocket upgrades for `/api/ws`.
The configured origin and incoming Host must agree. Password and launch login
set secure, HTTP-only, SameSite cookies under HTTPS; mutations also require the
session CSRF token.

## Health and recovery

- `GET /api/health` runs SQLite integrity validation.
- startup prints deterministic execution recovery counts;
- interrupted runs and leases are reconciled by the orchestration layer;
- worker tokens can be revoked independently from user sessions;
- stale worker events are rejected by assignment fencing epoch and sequence.

Raw provider logs and uncommitted files stay on the executor. Only bounded,
redacted artifacts and semantic events cross the control-plane boundary.

## Remote worker

Run the worker daemon on the machine that owns the repository and provider
credentials:

```bash
JET_BLACK_CONTROL_PLANE=https://jet-black.example.com \
JET_BLACK_WORKER_TOKEN='<device credential>' \
JET_BLACK_WORKER_HANDLER=/opt/jet-black/bin/worker-handler \
JET_BLACK_WORKER_CAPABILITIES=git,agent \
JET_BLACK_WORKER_REPOSITORIES=repository-stable-identity \
jet-black-worker
```

The worker makes outbound HTTPS requests only. It heartbeats every ten seconds,
claims an explicit assignment, and attaches the assignment fencing epoch and
monotonic event sequence to every result. Revoking the device credential stops
new heartbeats, claims, and event delivery. Rotate credentials independently
from user sessions.

Run the daemon under a service manager with a private environment file. The
handler path must be absolute. Do not place device credentials in command-line
arguments, browser storage, repository configuration, or the product database
of a different instance.
