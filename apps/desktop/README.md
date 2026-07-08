# desktop

Tauri desktop app with a Svelte 5 + Vite frontend.

This app uses `tauri-specta` to generate typed command bindings at
`src/lib/bindings.ts` when running `tauri dev` in debug mode.

## Commands

```bash
bun run --cwd apps/desktop dev
bun run --cwd apps/desktop build
```
