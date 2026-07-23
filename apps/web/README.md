# web

This app reads local configuration from the repository `.env.local` file.

Expected envs:

- `CONVEX_URL`
- `CONVEX_SITE_URL`
- `PUBLIC_CONVEX_URL` optional public deployment URL

Server env access uses SvelteKit's `$env/dynamic/private` module.

Use the existing scripts:

```bash
bun run --cwd apps/web dev
bun run --cwd apps/web build
bun run --cwd apps/web start
```

Copy the repository `.env.example` to `.env.local` and set the values needed by your local workflow.
