import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

const webRoot = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  build: {
    emptyOutDir: true,
    outDir: `${webRoot}build-standalone`,
  },
  plugins: [
    tailwindcss(),
    svelte({ configFile: `${webRoot}svelte.config.js` }),
  ],
});
