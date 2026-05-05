import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false, // auth flow 는 sequential (login state 공유)
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
    env: {
      ZITADEL_ISSUER: process.env.ZITADEL_ISSUER ?? "",
      ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID ?? "",
      ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE ?? "",
      ZITADEL_REDIRECT_URI: process.env.ZITADEL_REDIRECT_URI ?? "",
      REDIS_URL: process.env.REDIS_URL ?? "",
      SESSION_SECRET: process.env.SESSION_SECRET ?? "",
    },
  },
});
