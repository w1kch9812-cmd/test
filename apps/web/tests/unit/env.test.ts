// @vitest-environment node
import { beforeEach, describe, expect, it, vi } from "vitest";

describe("env schema (SP6-i extension)", () => {
  const originalNodeEnv = process.env.NODE_ENV;

  const setValidProductionEnv = () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "production-session-secret-32-bytes-valid";
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";
    process.env.PLATFORM_CORE_WEBHOOK_SECRET = "production-platform-core-webhook-secret-32-valid";
  };

  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
    if (originalNodeEnv !== undefined) {
      vi.stubEnv("NODE_ENV", originalNodeEnv);
    }
    delete process.env.INTERNAL_AUTH_SECRET;
    delete process.env.PLATFORM_CORE_WEBHOOK_SECRET;
    delete process.env.NEXT_PUBLIC_TILES_MANIFEST_URL;
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

  it("parses valid production environment", async () => {
    setValidProductionEnv();

    const { env } = await import("@/lib/env");
    expect(env.REDIS_URL).toBe("rediss://redis.gongzzang.test:6379");
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
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";
    delete process.env.NEXT_PUBLIC_API_BASE_URL;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on missing NEXT_PUBLIC_PLATFORM_CORE_BASE_URL in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";
    delete process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on missing SESSION_SECRET", async () => {
    delete process.env.SESSION_SECRET;
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on too-short SESSION_SECRET (< 32 chars)", async () => {
    process.env.SESSION_SECRET = "short";
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";

    await expect(import("@/lib/env")).rejects.toThrow();
  });

  it("throws on example SESSION_SECRET in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "change-me-to-random-32-byte-base64-string-aaaaaaaaaa";
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on repeated-character SESSION_SECRET in production", async () => {
    setValidProductionEnv();
    process.env.SESSION_SECRET = "x".repeat(32);

    await expect(import("@/lib/env")).rejects.toThrow(/SESSION_SECRET/);
  });

  it("throws on missing INTERNAL_AUTH_SECRET in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    delete process.env.INTERNAL_AUTH_SECRET;

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on missing PLATFORM_CORE_WEBHOOK_SECRET in production", async () => {
    setValidProductionEnv();
    delete process.env.PLATFORM_CORE_WEBHOOK_SECRET;

    await expect(import("@/lib/env")).rejects.toThrow(/PLATFORM_CORE_WEBHOOK_SECRET/);
  });

  it("throws on repeated-character PLATFORM_CORE_WEBHOOK_SECRET in production", async () => {
    setValidProductionEnv();
    process.env.PLATFORM_CORE_WEBHOOK_SECRET = "x".repeat(32);

    await expect(import("@/lib/env")).rejects.toThrow(/PLATFORM_CORE_WEBHOOK_SECRET/);
  });

  it("throws on development INTERNAL_AUTH_SECRET in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    process.env.INTERNAL_AUTH_SECRET = "dev-internal-auth-must-be-shared";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on too-short INTERNAL_AUTH_SECRET in production", async () => {
    setValidProductionEnv();
    process.env.INTERNAL_AUTH_SECRET = "production-short-secret";

    await expect(import("@/lib/env")).rejects.toThrow(/INTERNAL_AUTH_SECRET/);
  });

  it("throws on repeated-character INTERNAL_AUTH_SECRET in production", async () => {
    setValidProductionEnv();
    process.env.INTERNAL_AUTH_SECRET = "x".repeat(32);

    await expect(import("@/lib/env")).rejects.toThrow(/INTERNAL_AUTH_SECRET/);
  });

  it("throws on placeholder ZITADEL_CLIENT_ID in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "ci-placeholder";
    process.env.ZITADEL_AUDIENCE = "production-audience";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "production-session-secret-32-bytes-valid";
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on placeholder ZITADEL_AUDIENCE in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.ZITADEL_ISSUER = "https://auth.gongzzang.test";
    process.env.ZITADEL_CLIENT_ID = "production-client";
    process.env.ZITADEL_AUDIENCE = "ci-placeholder";
    process.env.ZITADEL_REDIRECT_URI = "https://gongzzang.test/api/auth/callback";
    process.env.REDIS_URL = "rediss://redis.gongzzang.test:6379";
    process.env.SESSION_SECRET = "production-session-secret-32-bytes-valid";
    process.env.NEXT_PUBLIC_API_BASE_URL = "https://api.gongzzang.test";
    process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "naver-client";
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.gongzzang.test";
    process.env.INTERNAL_AUTH_SECRET = "production-internal-auth-secret-32-valid";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on localhost NEXT_PUBLIC_API_BASE_URL in production", async () => {
    setValidProductionEnv();
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on localhost NEXT_PUBLIC_PLATFORM_CORE_BASE_URL in production", async () => {
    setValidProductionEnv();
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "http://127.0.0.1:18082";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on http ZITADEL_ISSUER in production", async () => {
    setValidProductionEnv();
    process.env.ZITADEL_ISSUER = "http://auth.gongzzang.test";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on http ZITADEL_REDIRECT_URI in production", async () => {
    setValidProductionEnv();
    process.env.ZITADEL_REDIRECT_URI = "http://gongzzang.test/api/auth/callback";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on localhost NEXT_PUBLIC_TILES_MANIFEST_URL in production", async () => {
    setValidProductionEnv();
    process.env.NEXT_PUBLIC_TILES_MANIFEST_URL = "http://localhost:18082/v1/map/tiles/manifest";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on localhost REDIS_URL in production", async () => {
    setValidProductionEnv();
    process.env.REDIS_URL = "redis://localhost:6379";

    await expect(import("@/lib/env")).rejects.toThrow(/REDIS_URL/);
  });

  it("throws on non-TLS REDIS_URL in production", async () => {
    setValidProductionEnv();
    process.env.REDIS_URL = "redis://redis.gongzzang.test:6379";

    await expect(import("@/lib/env")).rejects.toThrow(/REDIS_URL/);
  });
});
