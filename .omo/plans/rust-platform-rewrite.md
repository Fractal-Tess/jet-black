# Jet Black Rust Control-Plane Migration

## Status

Approved and in progress. ADR-007 is authoritative.

## Outcome

Jet Black becomes one Rust platform with a static Svelte client. A single
binary can run a local all-in-one product, a hosted server, or an
outbound-connected execution worker. Tauri embeds the same Rust core and
frontend. Rust and SQLite replace Convex and Better Auth completely.

## Milestone 0 — Preserve the execution foundation

- Validate and commit the inherited provider-selection work.
- Keep domain lifecycles, approvals, worktree confinement, recovery, review,
  artifacts, and generated TypeScript protocol contracts green.
- Record the superseding architecture and reference-reuse policy.

Gate: Rust workspace tests, execution-client unit tests, Svelte typecheck, web
build, and Ultracite pass.

## Milestone 1 — Rust product database and authentication

- Add versioned SQLite migrations for users, credentials, sessions, workspaces,
  memberships, projects, project memberships, states, labels, tickets,
  comments, attachments, sprints, modules, pages, feature settings, and the
  existing execution aggregates.
- Enable WAL, busy timeout, foreign keys, single-instance locking, backup,
  quotas, and deterministic startup recovery.
- Implement Argon2id password authentication, rotating opaque sessions, secure
  cookies, local launch-token exchange, CSRF/origin validation, and
  owner/admin/member/guest authorization.
- Seed deterministic development data. Existing Convex data and sessions may
  be discarded.

Gate: migration/reopen tests, auth/session tests, role matrix, cross-workspace
denials, account/workspace deletion invariants, and backup/restore tests pass.

## Milestone 2 — Product protocol and realtime server

- Extend generated Rust/TypeScript contracts with authenticated product
  commands, bounded snapshots, subscription topics, optimistic versions,
  structured errors, idempotency keys, and monotonic event cursors.
- Serve bootstrap/auth/upload/snapshot/health routes over HTTP.
- Add WebSocket command, subscription, acknowledgement, presence, and replay
  channels with bounded queues and reconnect recovery.
- Keep execution commands on the same domain boundary and protocol version.

Gate: transport contract fixtures, malformed/authz request tests, reconnect and
replay tests, backpressure limits, and HTTP/WebSocket parity tests pass.

## Milestone 3 — Static Svelte client cutover

- Convert the web application to a static client served by Rust and embeddable
  by Tauri.
- Replace Better Auth, `convex-svelte`, generated Convex API imports, SSR
  loaders, and Node-only environment access with one typed Jet Black client.
- Preserve URL-driven login, onboarding, workspaces, projects, tickets,
  comments, board/list views, intake, sprints, modules, pages, analytics, and
  settings.
- Use `Ticket` in new contracts while mapping transitional UI identifiers only
  at the cutover boundary.

Gate: static build, Rust-served deep-link fallback, unit tests, the complete
Playwright product suite, and agent-browser desktop/mobile walkthroughs pass.

## Milestone 4 — Ticket execution and local review

- Replace the standalone demo as the primary experience with ticket-linked
  execution attempts in the real product shell.
- Add repository selection, provider/model choice, timeline, approval,
  interrupt, artifacts, diff, checks, findings, commit/discard, recovery, and
  attempt history.
- Retain a minimal standalone harness only for transport and E2E fixtures.

Gate: ordinary tickets remain non-agentic, supervised fixture and real-provider
flows pass, restart recovery is visible, and browser validation covers the
complete ticket-to-review path.

## Milestone 5 — Remote workers

- Add separate device identity, pairing, rotation, revocation, capabilities,
  repository identities, presence, slots, leases, and outbound WebSocket
  connections.
- Route runs to explicit eligible workers. An accepted run may finish offline
  with its persisted authority but may not accept new commands or broaden
  scope.
- Persist a local outbox and reconcile events by command ID, sequence, fencing
  epoch, and expected head SHA. Divergence fails closed.

Gate: user/device identity separation, five-worker capability matching,
disconnect completion, replay, revocation, stale fencing, and no source/raw-log
upload tests pass.

## Milestone 6 — Desktop

- Add a Tauri entrypoint over the shared Rust core and static frontend.
- Support embedded-local and configured-remote instances.
- Keep filesystem, process, Git, credentials, and worker registration behind
  default-deny typed commands.
- Store instance and device credentials through the platform credential
  abstraction.

Gate: debug desktop build, capability matrix, credential isolation, restart,
local execution, and remote-worker browser-shell tests pass.

## Milestone 7 — Convex removal and release validation

- Delete Convex functions, Better Auth integration, Convex dependencies,
  migrator, Docker services, Node server adapter, transitional inventory, and
  dead compatibility code.
- Update development, deployment, backup, recovery, remote connection, and
  security documentation.
- Run dependency and vulnerability checks and produce source-free diagnostics.

Gate: clean install, Rust tests, frontend checks, static build, full Playwright,
agent-browser visual/accessibility QA, standalone smoke test, remote-worker
smoke test, desktop build, and production container smoke test all pass.

## Working agreement

- Inspect and reuse reference-project code whenever it already implements the
  required behavior; record project, revision, and files for substantial
  adaptations.
- Commit coherent green milestones directly to `main`.
- Use focused tests while iterating and full relevant gates before each commit.
- Do not commit agent session metadata, temporary screenshots, test accounts,
  worktrees, local databases, secrets, or generated runtime artifacts.
