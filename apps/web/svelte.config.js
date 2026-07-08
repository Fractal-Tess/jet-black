import adapter from "@sveltejs/adapter-node";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: {
      "@ui": "../../packages/ui/src/lib",
      "@ui/*": "../../packages/ui/src/lib/*",
    },
  },
};
