# Development

## Architecture

`jet-black` opens the execution and product stores against one SQLite file,
composes provider discovery, repository policy, orchestration, review, HTTP,
WebSocket, and static assets, then owns the listener for the lifetime of the
process. Product and execution migrations use separate migration ledgers in the
same database.

The browser imports only `packages/shared/src/protocol.generated.ts`. Change
Rust contracts first, then regenerate:

```bash
cargo run -p protocol --bin generate-types -- \
  packages/shared/src/protocol.generated.ts
```

The drift test fails if the checked-in TypeScript does not match Rust.

## Workflow

1. Make one coherent change.
2. Run the smallest relevant Rust or browser test.
3. Run `bun x ultracite fix` on touched frontend files.
4. Before a milestone commit, run typecheck/build, relevant Cargo tests, and
   Playwright for user-visible work.
5. Use `agent-browser` for the actual interaction path when execution,
   approvals, responsive layout, or remote navigation changes.

The generated frontend is static. Do not add SvelteKit server routes, a Node
adapter, Convex, or a second authentication authority.

## Runtime profiles

- `standalone`: loopback static UI plus local executor.
- `server`: public-origin control plane with an optional local executor.
- `desktop`: Tauri starts the same Rust composition in-process.
- remote workers authenticate with device bearer credentials, not user
  cookies; they heartbeat, claim explicit fenced assignments, and replay
  ordered outbox events.

Repository paths remain worker-local. Product records contain stable repository
identities, never another device's filesystem path.

## Worker handler contract

`jet-black-worker` is an outbound transport and fencing boundary. It never
accepts inbound network connections or evaluates shell text. For each claimed
assignment it starts the absolute executable configured by
`JET_BLACK_WORKER_HANDLER` directly and writes a JSON object containing the
full versioned assignment to standard input.

The handler returns one JSON document on standard output:

```json
{
  "events": [
    {"event_kind": "run.progress", "body": "{\"message\":\"working\"}"}
  ],
  "terminal_status": "completed"
}
```

Terminal status must be `completed`, `failed`, or `cancelled`. Event bodies are
serialized JSON strings so the worker can forward them without changing their
meaning. A non-zero handler exit is reported as `run.handler_failed` followed
by a failed terminal event.

## Reference reuse

Reference projects are recorded in `governance/provenance.json`. Prefer their
proven behavior when it matches an accepted ADR, and record substantial
adaptations with revision and source paths.
