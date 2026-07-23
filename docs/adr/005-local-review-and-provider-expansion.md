# ADR-005: Local Review and Provider Expansion

## Status

Accepted.

## Context

The final product needs ticket-scoped human and agent review integrated with GitHub, GitLab, and Gitea. Implementing three provider APIs before a useful local diff/review experience exists would multiply failure surfaces.

## Decision

The prototype ships local review:

- changeset summary and exact base/head identity;
- commits when present;
- file list and unified/side-by-side diff;
- configured format, typecheck, and test results;
- human findings and disposition;
- one read-only agent review run;
- separate approved fix runs;
- local commit or discard.

The internal `Changeset`, `ReviewRun`, and `Finding` models do not use provider-specific identifiers as primary keys.

Provider expansion follows this order:

1. Define the provider-neutral adapter contract from the proven local review model.
2. Implement GitHub first, including branch push verification, PR creation, comments/reviews, statuses, signed webhooks, and reconciliation.
3. Implement GitLab and Gitea only after provider-contract fixtures show which semantics can genuinely be normalized.

Provider PR/MR state remains authoritative for pushed refs, merge state, provider approvals, checks, and comments. Jet Black remains authoritative for its structured agent findings. Ticket completion, PR merge, human approval, and agent pass stay distinct unless an explicit workspace policy maps them.

## Consequences

- Local review is useful before any provider account is configured.
- Provider comments are projections of internal findings and require idempotency/reconciliation.
- Force pushes and moved diff anchors are future provider-phase concerns, not prototype blockers.
