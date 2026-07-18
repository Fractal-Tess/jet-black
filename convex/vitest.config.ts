import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    exclude: ["**/messages.test.ts", "**/node_modules/**"],
  },
});
