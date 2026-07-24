import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
  root: "src/client",
  publicDir: "../../static",
  plugins: [svelte()],
  build: {
    outDir: "../../build-client",
    emptyOutDir: true,
  },
  server: {
    host: "0.0.0.0",
    fs: { allow: ["../../../"] },
  },
});
