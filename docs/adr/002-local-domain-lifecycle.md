# ADR-002: Local Domain Lifecycle

## Status

Accepted.

## Context

UI labels, process state, Git state, and durable run state can easily diverge unless transitions are explicit. The prototype needs deterministic recovery more than a generalized workflow engine.

## Decision

Use small domain state machines enforced in Rust and persisted transactionally with their semantic events.

### Changeset

```text
created → active → reviewable → committed | discarded
                    └────────→ failed
```

- `created`: exact repository and base SHA recorded.
- `active`: one local mutation guard owns the worktree.
- `reviewable`: agent stopped and a diff/checkpoint exists.
- `committed`: user approved a local commit.
- `discarded`: worktree intentionally removed.
- `failed`: recovery requires an explicit retry or discard.

### Run

```text
queued → starting → running → awaiting_approval
                   ├────────→ reviewable → completed
                   └────────→ interrupted | failed
```

- Only Rust domain methods transition state.
- Every transition records a monotonic event in the same SQLite transaction.
- A protected action records its proposal digest before entering `awaiting_approval`.
- Approval is valid only for the unchanged proposal, repository, changeset, base/head SHA, and expiry.
- Restart recovery maps each nonterminal state to one documented resume, interrupt, or fail result; it never guesses that a process is still alive.
- One changeset permits one mutating run. Read-only observation and review may coexist.

### Review finding

```text
open → resolved | dismissed | superseded
```

A finding records path, blob/base identity, range when available, category, severity, message, and evidence. Applying a fix creates a new approved run rather than mutating during review.

## Consequences

- State behavior is testable without HTTP, Tauri, or a real provider.
- The prototype does not need XState or a generic workflow DSL.
- Remote lease/fencing states extend these machines later rather than replacing them.
