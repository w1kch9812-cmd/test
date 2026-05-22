// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("__Host- sid cookie helpers (production)", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_API_BASE_URL", "https://api.gongzzang.test");
    vi.stubEnv("NEXT_PUBLIC_PLATFORM_CORE_BASE_URL", "https://platform-core.gongzzang.test");
    vi.stubEnv("ZITADEL_ISSUER", "https://auth.gongzzang.test");
    vi.stubEnv("ZITADEL_CLIENT_ID", "production-client");
    vi.stubEnv("ZITADEL_AUDIENCE", "production-audience");
    vi.stubEnv("ZITADEL_REDIRECT_URI", "https://gongzzang.test/api/auth/callback");
    vi.stubEnv("REDIS_URL", "rediss://redis.gongzzang.test:6379");
    vi.stubEnv("SESSION_SECRET", "production-session-secret-32-bytes-valid");
    vi.stubEnv("INTERNAL_AUTH_SECRET", "production-internal-auth-secret-32-valid");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("uses __Host- prefix and security flags", async () => {
    const { SID_COOKIE_NAME } = await import("@/lib/session/cookie");
    expect(SID_COOKIE_NAME).toBe("__Host-sid");
  });

  it("setSidCookie returns Set-Cookie with all required flags", async () => {
    const { setSidCookie } = await import("@/lib/session/cookie");
    const setCookie = setSidCookie("abc123", 86400);
    expect(setCookie).toContain("__Host-sid=abc123");
    expect(setCookie).toContain("Secure");
    expect(setCookie).toContain("HttpOnly");
    expect(setCookie).toContain("SameSite=Strict");
    expect(setCookie).toContain("Path=/");
    expect(setCookie).toContain("Max-Age=86400");
    expect(setCookie).toContain("Partitioned");
  });

  it("deleteSidCookie returns Set-Cookie with Max-Age=0", async () => {
    const { deleteSidCookie } = await import("@/lib/session/cookie");
    const setCookie = deleteSidCookie();
    expect(setCookie).toContain("__Host-sid=");
    expect(setCookie).toContain("Max-Age=0");
  });
});

describe("dev cookie helpers (no prefix, localhost HTTP 호환)", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.stubEnv("NODE_ENV", "development");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("uses 'sid' (no __Host- prefix) in dev", async () => {
    const { SID_COOKIE_NAME } = await import("@/lib/session/cookie");
    expect(SID_COOKIE_NAME).toBe("sid");
  });

  it("uses 'auth-state' (no __Secure- prefix) in dev", async () => {
    const { AUTH_STATE_COOKIE_NAME } = await import("@/lib/session/cookie");
    expect(AUTH_STATE_COOKIE_NAME).toBe("auth-state");
  });

  it("setSidCookie omits Secure + Partitioned in dev", async () => {
    const { setSidCookie } = await import("@/lib/session/cookie");
    const setCookie = setSidCookie("abc123", 86400);
    expect(setCookie).toContain("sid=abc123");
    expect(setCookie).not.toContain("Secure");
    expect(setCookie).not.toContain("Partitioned");
    expect(setCookie).toContain("HttpOnly");
    expect(setCookie).toContain("SameSite=Strict");
    expect(setCookie).toContain("Path=/");
  });
});

describe("signAuthStatePayload / verifyAuthStatePayload", () => {
  it("roundtrips correctly", async () => {
    const { signAuthStatePayload, verifyAuthStatePayload } = await import("@/lib/session/cookie");
    const original = JSON.stringify({ x: 1 });
    const signed = signAuthStatePayload(original);
    expect(verifyAuthStatePayload(signed)).toBe(original);
  });

  it("returns null on tampered MAC", async () => {
    const { signAuthStatePayload, verifyAuthStatePayload } = await import("@/lib/session/cookie");
    const signed = signAuthStatePayload("data");
    const tampered = `${signed.slice(0, -2)}XX`;
    expect(verifyAuthStatePayload(tampered)).toBe(null);
  });

  it("returns null on tampered payload", async () => {
    const { signAuthStatePayload, verifyAuthStatePayload } = await import("@/lib/session/cookie");
    const signed = signAuthStatePayload("data");
    const dot = signed.indexOf(".");
    const originalMac = signed.slice(dot);
    const tampered = `ZXZpbA${originalMac}`;
    expect(verifyAuthStatePayload(tampered)).toBe(null);
  });

  it("returns null when no dot separator", async () => {
    const { verifyAuthStatePayload } = await import("@/lib/session/cookie");
    expect(verifyAuthStatePayload("nodothere")).toBe(null);
  });
});
