// @vitest-environment node
import { beforeEach, describe, expect, it, vi } from "vitest";

describe("env schema (SP6-i extension)", () => {
  const originalNodeEnv = process.env.NODE_ENV;

  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
    if (originalNodeEnv !== undefined) {
      vi.stubEnv("NODE_ENV", originalNodeEnv);
    }
    delete process.env.INTERNAL_AUTH_SECRET;
  });

  it("parses ZITADEL_* and REDIS_URL when set", async () => {
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "demo-client";
    process.env.ZITADEL_AUDIENCE = "demo-client";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";

    const { env } = await import("@/lib/env");
    expect(env.ZITADEL_ISSUER).toBe("http://localhost:8443");
    expect(env.SESSION_SECRET.length).toBeGreaterThanOrEqual(32);
  });

  it("throws on missing NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID", async () => {
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";
    delete process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on placeholder NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID", async () => {
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-maps-placeholder";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on missing NEXT_PUBLIC_API_BASE_URL in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret";
    delete process.env.NEXT_PUBLIC_API_BASE_URL;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on missing SESSION_SECRET", async () => {
    delete process.env.SESSION_SECRET;
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on too-short SESSION_SECRET (< 32 chars)", async () => {
    process.env.SESSION_SECRET = "short";
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";

    await expect(import("@/lib/env")).rejects.toThrow();
  });

  it("throws on missing INTERNAL_AUTH_SECRET in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    delete process.env.INTERNAL_AUTH_SECRET;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on development INTERNAL_AUTH_SECRET in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.INTERNAL_AUTH_SECRET = "dev-internal-auth-must-be-shared";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });
});
