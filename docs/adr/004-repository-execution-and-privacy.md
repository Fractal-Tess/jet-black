# ADR-004: Repository Execution and Privacy

## Status

Accepted.

## Context

The highest-risk prototype functionality is running an agent against a developer repository. Repository edge cases, process cleanup, and accidental data retention matter before remote collaboration.

## Decision

### Repository onboarding

- Accept existing non-bare local Git repositories selected explicitly by the user.
- Record canonical path, repository identity, primary remote when present, default branch, and current base SHA.
- Detect and explain dirty state, missing commits, submodules, Git LFS, linked worktrees, and unsupported repository forms. Do not silently normalize them.
- The prototype supports one repository per changeset. Multi-repository workspaces and automatic cloning are deferred.

### Execution

- Every mutating run uses an app-owned isolated worktree beneath the configured Jet Black data directory.
- Clear Git environment variables that can redirect repository context. Validate canonical paths and symlink targets before each filesystem operation.
- Spawn agents in an owned process group with cancellation, timeout, bounded output, and deterministic cleanup.
- Use a mock provider for deterministic tests and one real CLI provider for the prototype.
- Setup, format, typecheck, and test commands are explicit repository configuration; no shell snippets are inferred from repository text.

### Persistence and privacy

- SQLite stores repositories, changesets, runs, semantic events, approvals, checkpoints, and findings.
- Raw stdout/stderr is stored locally in size-capped files. Model thinking is not a durable product record.
- The prototype does not upload source, raw logs, diffs, prompts, or artifacts.
- Provide visible per-run deletion and a configurable local storage limit before calling the prototype complete. Automatic age-based retention may follow later.
- Structured logs avoid source content and secrets by default.

## Consequences

- Local operation works without a cloud artifact service.
- Large or unusual repositories may be rejected with an actionable explanation rather than partially supported.
- Remote semantic-event sync and artifact retention can be added later without changing local process ownership.
