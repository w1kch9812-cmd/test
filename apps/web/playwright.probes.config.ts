import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/probes",
  testMatch: "**/*.probe.ts",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://localhost:3000",
    trace: "retain-on-failure",
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
      ZITADEL_ISSUER: process.env.ZITADEL_ISSUER ?? "http://localhost:8443",
      ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID ?? "ci-placeholder",
      ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE ?? "ci-placeholder",
      ZITADEL_REDIRECT_URI:
        process.env.ZITADEL_REDIRECT_URI ?? "http://localhost:3000/api/auth/callback",
      REDIS_URL: process.env.REDIS_URL ?? "redis://localhost:6379",
      SESSION_SECRET: process.env.SESSION_SECRET ?? "ci-placeholder-secret-32-bytes-padding-ok",
    },
  },
});
