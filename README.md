# Jet Black

Jet Black is an agile agentic development platform. One Rust process owns the
product control plane, SQLite database, realtime transport, Git worktrees,
provider execution, approvals, artifacts, and local review. The same static
Svelte client runs in a browser or the Tauri desktop shell.

## Repository

- `apps/jet-black`: the Rust composition root for standalone, server, and
  desktop profiles.
- `apps/web`: the static Svelte client and Playwright product tests.
- `apps/desktop/src-tauri`: the native shell; it starts the same Rust platform
  in-process.
- `apps/worker`: an outbound-only device daemon for fenced remote execution.
- `crates/control-plane`: users, sessions, roles, projects, tickets, planning
  records, workers, quotas, events, and migrations.
- `crates/protocol`: versioned Rust contracts and generated TypeScript.
- `crates/persistence`, `git`, `execution`, `agents`, `orchestration`, and
  `review`: the supervised execution substrate.
- `packages/shared`: generated browser-safe protocol types.

Convex, Better Auth, SvelteKit server routes, and the Node production server
have been removed. See [ADR-007](docs/adr/007-rust-control-plane.md).

## Run locally

```bash
bun install
bun run dev
```

The development seed is enabled by that command:

- email: `dev@jet-black.local`
- password: `jet-black-development`
- URL: `http://127.0.0.1:4317`

Approve repositories for local execution:

```bash
JET_BLACK_REPOSITORY_ROOTS=/absolute/repository bun run dev
```

Multiple roots use the platform path separator (`:` on Linux/macOS). Provider
selection uses `JET_BLACK_PROVIDER=mock|claude_code|codex|open_code`.

The Tauri desktop needs no account for local use. It creates a local owner and
guides the user through creating a workspace, then projects can link to a local
Git path (with `origin` inferred automatically) or a remote repository URL.
Shared instances support signup with administrator approval and workspace role
assignment. Generate a single-use desktop token from Settings to attach those
remote workspaces to a desktop client. See
[ADR-008](docs/adr/008-local-onboarding-and-workspace-connections.md).

Remote worker enrollment is intentionally separate from user login. An
administrator creates a device credential through the control-plane API, then
starts `jet-black-worker` with its control-plane origin, token, advertised
repositories, and an absolute handler executable. The handler protocol and
deployment details are in [docs/operations.md](docs/operations.md).

## Build and verify

```bash
bun run build
cargo test --workspace
bun run --cwd apps/web typecheck
bun run --cwd apps/web test
bun x ultracite check
```

On Linux, checking the Tauri target requires GTK/WebKit development packages.
With Nix:

```bash
nix-shell -p pkg-config gtk3 webkitgtk_4_1 --run \
  'cargo check -p jet-black-desktop'
```

The Linux desktop defaults to the X11 GTK backend because WebKitGTK can
miscalculate CSS pixel scaling under fractional-scale Wayland compositors.
Set `JET_BLACK_GDK_BACKEND=wayland` to opt into native Wayland where it renders
correctly.

Development and operations details are in
[docs/development.md](docs/development.md) and
[docs/operations.md](docs/operations.md).
