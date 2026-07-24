# ADR-007: Rust Control Plane and Configurable Clients

## Status

Accepted.

## Supersedes

This record supersedes the parts of ADR-001 and ADR-006 that preserve Convex as
the collaborative control plane or treat its replacement as optional. Their
local execution, safety, lifecycle, privacy, and incremental delivery decisions
remain in force.

## Context

Jet Black began as a SvelteKit and Convex project tracker. The product has
expanded into an agile agentic platform that must run locally, as a hosted web
application, as a desktop client, and as an execution worker. Keeping Convex for
collaboration while running a separate Rust execution service creates two
authorities, two processes, duplicated authentication, and an unnecessary
synchronization boundary.

The Rust crates already implement the safety-critical execution foundation:
typed commands and events, SQLite persistence, repository registration,
isolated worktrees, provider adapters, approvals, recovery, review, and bounded
artifacts.

## Decision

### One Rust platform

The `jet-black` executable is the product backend. Runtime profiles compose the
same domain services:

- `standalone`: static web UI, control plane, SQLite, and local executor.
- `server`: static web UI, authenticated control plane, and optional executor.
- `worker`: outbound-connected execution target without product-data authority.
- `desktop`: Tauri shell using the same frontend and Rust core; it may use its
  embedded instance or connect to another instance.

One process owns one SQLite database. SQLite files are never shared by multiple
processes or replicated between instances.

### Client and worker boundaries

The Svelte frontend is a static client served or embedded by Rust. It connects
to a selected local or remote instance through typed HTTP and WebSocket
transports. SvelteKit server loaders and the Node adapter are removed.

A desktop connected to a remote instance has separate user and device
identities. User credentials authorize product actions. Device credentials
authorize advertised execution capabilities and repository-scoped work. A
browser may control an eligible worker but cannot provide native execution.

### Authority and synchronization

The control plane owns users, sessions, workspaces, projects, tickets,
permissions, commands, approvals, and semantic run history. The worker owns
local repository paths, worktrees, live processes, provider credentials,
uncommitted files, and raw logs.

An accepted run may finish after disconnect using only its already persisted
command, approval scope, repository identity, and expected base/head revision.
It cannot accept new commands or expand authority while offline. Reconnection
replays idempotent events in sequence and rejects divergent repository or
fencing state.

### Storage and transport

SQLite runs in WAL mode with explicit migrations, busy timeouts, transactional
domain events, optimistic versions, backup/restore, quotas, and startup
recovery. Large artifacts and attachments live in an app-owned filesystem store
referenced from SQLite by digest.

HTTP serves bootstrap, authentication, static assets, uploads, bounded
snapshots, diagnostics, and health endpoints. WebSockets carry authenticated
commands, subscriptions, ordered events, worker presence, acknowledgements,
and replay cursors.

### Authentication and product migration

Rust owns password authentication, secure cookie sessions, local launch-token
exchange, role enforcement, worker registration, and revocation. The first
Convex-free release preserves team workspaces and authorization. Existing
development data and sessions may be reset; no Convex importer is required.

The Rust model uses `Ticket` as the canonical name. Tickets and execution
attempts remain separate aggregates. Convex is removed after Rust behavior
parity and frontend cutover; there is no permanent dual-write mode.

### Reference implementations

The projects recorded in `governance/provenance.json` are reusable source
material. Prefer a proven reference implementation when it matches Jet Black's
requirement, copying or adapting it as needed rather than rewriting it merely
to be different. Substantial adaptations record the project, revision, and
relevant files for traceability.

## Delivery policy

- Commit each coherent, validated milestone directly to `main`.
- Run focused tests during development and full relevant gates before commits.
- UI milestones include Playwright coverage and agent-browser interaction,
  accessibility inspection, and screenshots.
- Preserve the safety and recovery gates from ADR-002 through ADR-005.

## Consequences

- Convex, Better Auth's Convex adapter, and the Node production server are
  transitional dependencies to be deleted.
- The protocol expands from execution-only messages to the complete product
  domain.
- Runtime profiles share code without conflating client, control-plane, and
  worker authority.
- Remote collaboration remains single-node per control-plane instance until a
  later ADR authorizes another persistence topology.
