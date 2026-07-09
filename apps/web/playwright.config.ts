import { defineConfig, devices } from "@playwright/test";

const port = 5173;
const chromiumExecutablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
const appOrigin = `http://127.0.0.1:${port}`;

export default defineConfig({
  fullyParallel: true,
  testDir: "./tests",
  timeout: 30_000,
  use: {
    baseURL: appOrigin,
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },
  webServer: {
    command: `bun run dev -- --port ${port} --strictPort`,
    port,
    reuseExistingServer: true,
    timeout: 180_000,
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        launchOptions: chromiumExecutablePath
          ? {
              executablePath: chromiumExecutablePath,
            }
          : undefined,
      },
    },
  ],
});
