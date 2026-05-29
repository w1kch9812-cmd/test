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
      // URL 검증 통과용 fallback — 실 Zitadel 미연결 시 ZITADEL_ISSUER 미설정으로
      // test.skip(!process.env.ZITADEL_ISSUER) 가 작동해 Zitadel 의존 e2e 는 graceful skip.
      NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080",
      NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID:
        process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID ?? "ci-e2e-naver-client",
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL:
        process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL ?? "http://localhost:18082",
      ZITADEL_ISSUER: process.env.ZITADEL_ISSUER ?? "http://localhost:8443",
      ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID ?? "ci-placeholder",
      ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE ?? "ci-placeholder",
      ZITADEL_REDIRECT_URI:
        process.env.ZITADEL_REDIRECT_URI ?? "http://localhost:3000/api/auth/callback",
      REDIS_URL: process.env.REDIS_URL ?? "redis://localhost:6379",
      SESSION_SECRET: process.env.SESSION_SECRET ?? "ci-placeholder-secret-32-bytes-padding-ok",
      INTERNAL_AUTH_SECRET:
        process.env.INTERNAL_AUTH_SECRET ?? "ci-e2e-internal-auth-secret-32-valid",
      PLATFORM_CORE_WEBHOOK_SECRET:
        process.env.PLATFORM_CORE_WEBHOOK_SECRET ?? "ci-e2e-platform-core-webhook-secret-32",
    },
  },
});
