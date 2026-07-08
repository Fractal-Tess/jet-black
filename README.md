# jet-black

A full-stack Svelte workspace with SvelteKit, a shared shadcn-svelte-ready UI package, Tauri, Convex, Bun, and Crawl4AI.

## Stack

- `apps/web`: SvelteKit app with Better Auth and Convex data.
- `apps/desktop`: Tauri app with a Svelte renderer.
- `apps/worker`: Bun HTTP worker.
- `apps/scraper`: Python and Crawl4AI scraper.
- `databases/convex`: Convex schema, functions, auth, and seed data.
- `packages/shared`: shared TypeScript helpers.
- `packages/ui`: shared Svelte 5 components and Tailwind theme.

The React Native app was removed because the reference SveltePlex project does
not provide a supported Svelte mobile renderer.

## Quickstart

```bash
bun install
direnv allow
bun run dev
```

See [docs/environment.md](./docs/environment.md) for environment setup.

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
bun run --cwd apps/desktop typecheck
bun run --cwd apps/desktop build:web
bun x ultracite check
```
