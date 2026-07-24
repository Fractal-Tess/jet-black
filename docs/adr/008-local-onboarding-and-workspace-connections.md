# ADR-008: Local Onboarding and Workspace Connections

## Status

Implemented.

## Context

Desktop users should not create credentials merely to use projects on their own
machine. Shared control planes need the opposite boundary: named accounts,
administrator approval, explicit workspace membership, and a safe way for a
desktop client to connect without retaining the user's password.

A workspace is a collection of projects. A project may refer to an existing
repository on the desktop filesystem or to a remote Git repository. The local
path is useful only to the instance that owns that filesystem; its Git origin
is the stable identity used across execution boundaries.

## Decision

### Local desktop identity

The desktop profile creates one idempotent, passwordless local owner identity.
The client exchanges no secret and automatically receives a same-origin local
session. First run asks only for a workspace name and slug. Local workspaces and
projects remain in the desktop instance's SQLite database.

### Shared control-plane identity

The first registered account becomes the instance administrator. Later
registrations remain pending and cannot authenticate until an instance
administrator approves them. Workspace access is assigned separately with the
existing owner, administrator, member, and guest roles.

An authenticated user may generate a random, single-use desktop pairing token
that expires after ten minutes. Exchanging it creates a revocable, long-lived
desktop session. The desktop sends that session as a bearer credential plus the
session's CSRF credential for mutations. Worker enrollment and worker
credentials remain separate and never inherit user authority.

### Workspace connections and authority

The desktop always retains its embedded local control plane and may save
connections to additional Jet Black instances. Selecting a connection loads
that instance's authorized workspaces; selecting a workspace never merges or
replicates databases. Every workspace has exactly one authoritative control
plane.

### Repository links

Projects may have no repository, a local repository, or a remote repository.
Only a desktop control plane accepts local paths. It canonicalizes the path and
invokes Git without a shell to read the `origin` remote. The canonical path is
used as a local fallback identity when no origin exists. Remote links accept
HTTP, HTTPS, SSH, and Git-style URLs.

Repository paths stay inside the filesystem-owning control plane. Remote
control planes receive stable repository identities and worker-advertised
capabilities, not paths from another machine.

## Reference implementations

The pairing/bootstrap split follows the behavior and terminology in T3 Code's
`packages/contracts/src/auth.ts`,
`packages/client-runtime/src/connection/onboarding.ts`, and associated tests.
Jet Black adapts that pattern to its existing opaque SQLite sessions and
workspace authorization rather than adopting the reference relay topology.

## Consequences

- A local desktop has no login or password recovery ceremony.
- Account approval and workspace assignment are distinct administrative acts.
- Pairing tokens are not ordinary access tokens and cannot be replayed.
- Desktop user sessions do not authorize native worker execution.
- A project can display both its local location and inferred origin while
  preserving one stable repository identity.
