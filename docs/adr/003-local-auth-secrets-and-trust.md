# ADR-003: Local Auth, Secrets, and Trust

## Status

Accepted.

## Context

The prototype is single-user and local, but the WebView, browser, repository contents, prompts, agent output, and agent subprocess remain untrusted inputs. Full remote identity and device pairing are unnecessary for the first milestone.

## Decision

- Bind standalone HTTP to loopback by default and use an unguessable per-launch browser session token for state-changing endpoints.
- Tauri uses default-deny capabilities and narrow typed commands. The WebView receives no arbitrary shell or filesystem API.
- Repository access begins with explicit user selection and Rust-side canonical path validation.
- Provider secrets come from environment variables or the OS credential store when available. Do not create an unencrypted long-lived secrets file as a fallback.
- Secret wrapper types redact debug output. Child processes receive an explicit environment allowlist, not the full parent environment.
- Agent actions do not inherit Git-provider, database, control-plane, or unrelated provider credentials.
- Local approval binds the exact action and expires. UI button visibility is never authorization.
- The prototype records one local actor identity for audit display but does not claim enterprise authentication.

Deferred remote expansion adds user sessions, device credentials, one-time pairing, revocation, role checks, and worker trust classes. Personal-local workers will not be treated as proof that company policy was honestly enforced.

## Consequences

- Local mode remains convenient without being an unauthenticated network service.
- Some providers may require the user to configure credentials outside the app initially.
- Secret-store portability is not allowed to weaken process isolation or logging rules.
