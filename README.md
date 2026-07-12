# jet-black

A full-stack Svelte workspace with SvelteKit, a shared shadcn-svelte-ready UI package, Convex, and Bun.

## Stack

- `apps/web`: SvelteKit app with Better Auth and Convex data.
- `convex`: standalone workspace for Convex schema, functions, auth, and the production migrator.
- `packages/shared`: shared TypeScript helpers.
- `packages/ui`: shared Svelte 5 components and Tailwind theme.

## Quickstart

```bash
bun install
direnv allow
bun run dev
```

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
