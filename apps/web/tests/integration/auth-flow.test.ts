// @vitest-environment node

import { NextRequest } from "next/server";
import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { GET as callbackGET } from "@/app/api/auth/callback/route";
import { POST as loginPOST } from "@/app/api/auth/login/route";
import { SID_COOKIE_NAME, TEMP_COOKIE_NAME, verifyTempPayload } from "@/lib/session/cookie";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";

vi.mock("@/lib/oidc", async () => {
  const actual = await vi.importActual<typeof import("@/lib/oidc")>("@/lib/oidc");
  return {
    ...actual,
    exchangeCode: vi.fn(async () => ({
      access_token: "at-1",
      refresh_token: "rt-1",
      id_token: "it-1",
      expires_in: 300,
      jti: "jti-1",
      sub: "user-1",
      role: "Buyer",
    })),
  };
});

// Mock fetch so emitAuthEvent does not fail in test environment
vi.stubGlobal(
  "fetch",
  vi.fn(async () => new Response(null, { status: 200 })),
);

describe("auth flow integration", () => {
  beforeEach(async () => {
    // db 0 — isolated from unit tests (db 1 store, db 2 single-flight)
    await getRedis().select(0);
    await getRedis().flushdb();
  });

  afterAll(() => {
    __resetRedisForTest();
  });

  it("login → 302 → callback → session created", async () => {
    const loginReq = new NextRequest("http://localhost:3000/api/auth/login", {
      method: "POST",
      body: new FormData(),
    });
    const loginRes = await loginPOST(loginReq);
    expect(loginRes.status).toBe(302);
    const setCookie = loginRes.headers.get("set-cookie") ?? "";
    expect(setCookie).toContain(`${TEMP_COOKIE_NAME}=`);

    const tmpMatch = setCookie.match(new RegExp(`${TEMP_COOKIE_NAME}=([^;]+)`));
    expect(tmpMatch).not.toBeNull();
    const tmp = String(tmpMatch?.[1]);
    // C2: cookie is now HMAC-signed (payload.mac); use verifyTempPayload to decode
    const rawPayload = verifyTempPayload(tmp);
    expect(rawPayload).not.toBeNull();
    if (!rawPayload) throw new Error("verifyTempPayload returned null");
    const decoded = JSON.parse(rawPayload) as {
      state: string;
    };

    const callbackReq = new NextRequest(
      `http://localhost:3000/api/auth/callback?code=abc&state=${decoded.state}`,
      { headers: { cookie: `${TEMP_COOKIE_NAME}=${tmp}` } },
    );
    const callbackRes = await callbackGET(callbackReq);
    expect(callbackRes.status).toBe(302);
    const sidCookie = callbackRes.headers.get("set-cookie") ?? "";
    expect(sidCookie).toMatch(new RegExp(`${SID_COOKIE_NAME}=[0-9a-f]{64}`));
  });
});
