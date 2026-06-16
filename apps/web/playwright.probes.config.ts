import { defineConfig, devices } from "@playwright/test";
import { DEFAULT_PROBE_PLAYWRIGHT_PORT, resolvePlaywrightRuntime } from "./playwright-runtime";

const runtime = resolvePlaywrightRuntime({
  env: process.env,
  defaultPort: DEFAULT_PROBE_PLAYWRIGHT_PORT,
});

export default defineConfig({
  testDir: "./tests/probes",
  testMatch: "**/*.probe.ts",
  outputDir: runtime.outputDir,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [["list"], ["html", { open: "never", outputFolder: runtime.reportDir }]],
  use: {
    baseURL: runtime.baseURL,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: runtime.command,
    url: runtime.webServerUrl,
    reuseExistingServer: runtime.reuseExistingServer,
    timeout: 120000,
    env: {
      ZITADEL_ISSUER: process.env.ZITADEL_ISSUER ?? "http://localhost:8443",
      ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID ?? "ci-placeholder",
      ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE ?? "ci-placeholder",
      ZITADEL_REDIRECT_URI: process.env.ZITADEL_REDIRECT_URI ?? runtime.zitadelRedirectUri,
      REDIS_URL: process.env.REDIS_URL ?? "redis://localhost:6379",
      SESSION_SECRET: process.env.SESSION_SECRET ?? "ci-placeholder-secret-32-bytes-padding-ok",
    },
  },
});
