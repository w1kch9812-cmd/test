// @vitest-environment node

import { NextRequest } from "next/server";
import { afterAll, beforeEach, describe, expect, it } from "vitest";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";
import { createSession } from "@/lib/session/store";
import { middleware } from "@/middleware";

describe("middleware", () => {
  beforeEach(async () => {
    await getRedis().select(4); // middleware 전용 db
    await getRedis().flushdb();
  });

  afterAll(() => __resetRedisForTest());

  it("allows public paths without sid", async () => {
    const req = new NextRequest("http://localhost:3000/login");
    const res = await middleware(req);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-security-policy")).toContain("default-src 'self'");
  });

  it("redirects unauthenticated to /login with returnTo", async () => {
    const req = new NextRequest("http://localhost:3000/profile");
    const res = await middleware(req);
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toContain("/login?returnTo=%2Fprofile");
  });

  it("redirects to /forbidden when role mismatch on /admin", async () => {
    const sid = await createSession(
      {
        sub: "u1",
        jti: "j1",
        role: "Buyer",
        access_token: "at",
        refresh_token: "rt",
        id_token: "it",
        exp: Math.floor(Date.now() / 1000) + 300,
      },
      300,
    );
    const req = new NextRequest("http://localhost:3000/admin/users", {
      headers: { cookie: `${SID_COOKIE_NAME}=${sid}` },
    });
    const res = await middleware(req);
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toContain("/forbidden");
  });

  it("rate limits /api/auth/login", async () => {
    for (let i = 0; i < 5; i++) {
      const req = new NextRequest("http://localhost:3000/api/auth/login", {
        method: "POST",
        headers: { "x-forwarded-for": "1.2.3.4" },
      });
      const r = await middleware(req);
      expect(r.status).not.toBe(429);
    }
    const req = new NextRequest("http://localhost:3000/api/auth/login", {
      method: "POST",
      headers: { "x-forwarded-for": "1.2.3.4" },
    });
    const r = await middleware(req);
    expect(r.status).toBe(429);
  });
});
