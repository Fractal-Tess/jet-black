# ADR-001: Local-First Prototype Scope

## Status

Accepted.

## Context

Jet Black already has a working Svelte/Convex kanban but no Rust, Tauri, Git-worktree, or agent execution substrate. Building device pairing, remote realtime, multi-worker scheduling, provider webhooks, and backend migration before proving local execution would delay the first useful product and test the least certain value last.

## Decision

The first usable milestone is a deliberate single-user local workflow:

1. Launch `jet-black run --profile standalone` or the Tauri prototype.
2. Open an existing ticket while the current Convex kanban remains unchanged.
3. Register an existing local Git repository.
4. Start a supervised run using a mock provider or one supported local CLI.
5. Create an isolated worktree and stream normalized progress.
6. Require approval for an exact protected action.
7. Show the resulting diff and local review findings.
8. Commit or discard the changeset.
9. Recover understandable run state after restart.

Prototype release blockers are execution safety, persistence, recovery, narrow IPC/API surfaces, and a coherent UI flow. Remote control planes, pairing, multi-worker scheduling, cloud execution, provider webhooks, auto-updates, and Convex migration are deferred.

## Consequences

- Rust execution is additive; existing kanban behavior remains available and feature-disableable.
- Standalone HTTP and Tauri IPC must call the same domain services.
- The prototype may use one protocol version and one real CLI provider.
- Remote-ready fields and traits may exist, but unneeded services are not implemented.
- Phase 1 has a two-week stop/go gate before standalone UI and Tauri expansion.

## Related

- [Rust platform rewrite plan](../../.omo/plans/rust-platform-rewrite.md)
- [ADR-006](006-evolution-operations-and-licensing.md)
