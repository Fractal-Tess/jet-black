# Architecture Decision Records

These records capture decisions required to build the local-first Jet Black prototype. The broader sequencing and verification plan lives in [`.omo/plans/rust-platform-rewrite.md`](../../.omo/plans/rust-platform-rewrite.md).

| ADR | Decision |
|---|---|
| [001](001-local-first-prototype.md) | Ship a supervised local execution workflow before remote infrastructure |
| [002](002-local-domain-lifecycle.md) | Use explicit lifecycle transitions for changesets and runs |
| [003](003-local-auth-secrets-and-trust.md) | Treat the prototype as single-user while protecting secrets and native boundaries |
| [004](004-repository-execution-and-privacy.md) | Isolate work in Git worktrees and keep detailed execution evidence local |
| [005](005-local-review-and-provider-expansion.md) | Ship local review first and add Git providers incrementally |
| [006](006-evolution-operations-and-licensing.md) | Preserve expansion seams without making production infrastructure a prototype blocker |
| [007](007-rust-control-plane.md) | Replace Convex with a single-process Rust control plane and configurable local or remote clients |

ADRs are amended by a new superseding record rather than silently rewritten after implementation depends on them.
