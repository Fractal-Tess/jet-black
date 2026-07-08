# web

This app reads secrets from Infisical path `/web`.

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

They already run through:

```bash
infisical run --path=/web -- ...
```
