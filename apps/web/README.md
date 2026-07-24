# Jet Black web client

This package is a static Vite/Svelte client. It contains no server loaders and
does not own authentication or product data.

```bash
bun run --cwd apps/web typecheck
bun run --cwd apps/web build
bun run --cwd apps/web test
```

Production assets are emitted to `build-client` and served by the `jet-black`
Rust binary. Choosing another instance in Settings navigates to that server so
session cookies and WebSockets remain same-origin.
