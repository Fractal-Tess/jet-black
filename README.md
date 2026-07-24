# jet-black

An agile agentic development platform with a Rust control plane, SQLite, local
or remote execution workers, and a shared Svelte web/desktop interface.

## Stack

- `apps/standalone`: the current Rust composition root and local web host.
- `apps/web`: the shared Svelte interface, transitioning to a static Rust-served client.
- `crates/*`: domain, protocol, persistence, Git, execution, providers, review, policy, and HTTP services.
- `packages/shared`: TypeScript contracts generated from the Rust protocol.
- `packages/ui`: shared Svelte 5 components and Tailwind theme.
- `convex`: transitional backend retained only until Rust product-domain parity and cutover.

## Quickstart

```bash
bun install
direnv allow
bun run dev
```

The current web development command still uses the transitional Convex
backend. See [ADR-007](docs/adr/007-rust-control-plane.md) for the accepted
replacement architecture.

## UI package

The package follows SveltePlex's shared-package structure. Add components with:

```bash
bunx shadcn-svelte@latest add button -c packages/ui
```

Import components from the workspace:

```svelte
<script lang="ts">
  import { Button } from "@workspace/ui/components/button";
</script>
```

## Verification

```bash
bun run --cwd packages/ui typecheck
bun run --cwd apps/web typecheck
bun run --cwd apps/web build
bun x ultracite check
```
