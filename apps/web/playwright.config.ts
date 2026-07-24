import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.PLAYWRIGHT_WEB_PORT ?? 4319);
const chromiumExecutablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
const appOrigin = `http://127.0.0.1:${port}`;

export default defineConfig({
  fullyParallel: true,
  testDir: "./tests-client",
  timeout: 120_000,
  use: {
    baseURL: appOrigin,
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },
  webServer: {
    command: `bun run build && JET_BLACK_BIND=127.0.0.1:${port} JET_BLACK_PUBLIC_ORIGIN=${appOrigin} JET_BLACK_DATA_DIR=.playwright-data JET_BLACK_STATIC_ASSETS_DIR=build-client JET_BLACK_DEV_SEED=1 JET_BLACK_DEV_PASSWORD=jet-black-development cargo run -p jet-black`,
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
