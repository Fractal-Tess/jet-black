# ADR-006: Evolution, Operations, and Licensing

## Status

Accepted.

## Context

The architecture must leave room for company-hosted control planes and many workers without burdening the local prototype with premature distributed systems and enterprise operations.

## Decision

### Compatibility

- Prototype messages include protocol version `0.1`, but only one version is supported.
- Standalone HTTP and Tauri IPC use the same domain commands/events.
- Before remote distribution, define a supported version window, capability negotiation, signed updates, and drain/rollback policy.

### Operations

Prototype operations are limited to:

- one process;
- SQLite and an app data directory;
- structured logs with configurable level;
- startup validation;
- visible recovery/cleanup reporting;
- exportable diagnostics that exclude secrets and source by default.

Health endpoints, Postgres backup objectives, metrics/alerts, high availability, remote artifact storage, rate limits, and disaster recovery enter with the hosted control-plane phase.

### Deferred scheduling and trust

- Remote prototype scheduling begins with explicit worker selection, one active mutation per changeset, no preemption, and no automatic reassignment after execution starts.
- Capability matching, fairness, priorities, slot scheduling, drain, and cloud trust tiers follow after two or more real hosts are needed.

### Licensing and references

- The cloned projects are architectural references, not blanket sources for copied code.
- Record provenance and license review for adapted implementations.
- Prefer clean-room implementation when license compatibility is unclear, especially for AGPL reference code.
- Add dependency license and vulnerability checks before distribution.
- Licensing enforcement, billing, and commercial entitlements are not prototype features.

## Consequences

- The prototype is intentionally easy to run and diagnose.
- Remote compatibility is preserved through typed interfaces and version fields, not implemented infrastructure.
- Production requirements become release gates when their corresponding deployment profile is enabled.
