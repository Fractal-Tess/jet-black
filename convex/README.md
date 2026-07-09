# convex

This package owns the Convex schema, functions, tests, and local seed command.

## Commands

```bash
bun run --cwd convex dev
bun run --cwd convex dashboard
bun run --cwd convex seed
bun run --cwd convex test
```

`seed` runs `convex run init:seed '{}'` against the active deployment and is idempotent.
