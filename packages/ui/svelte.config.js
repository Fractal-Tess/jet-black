import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: { "@lib": "./src/lib", "@lib/*": "./src/lib/*" },
  },
};
