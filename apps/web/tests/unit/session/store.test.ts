import { afterAll, beforeEach, describe, expect, it } from "vitest";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";
import {
  createSession,
  deleteSession,
  getSession,
  refreshSession,
  type SessionData,
} from "@/lib/session/store";

const sample: SessionData = {
  sub: "user-uuid-1",
  jti: "jti-1",
  role: "Buyer",
  access_token: "at-1",
  refresh_token: "rt-1",
  id_token: "it-1",
  exp: Math.floor(Date.now() / 1000) + 300,
};

describe("SessionStore (Redis)", () => {
  beforeEach(async () => {
    // db 1 — isolated from single-flight tests (db 2) for parallel vitest workers
    await getRedis().select(1);
    await getRedis().flushdb();
  });

  afterAll(() => {
    __resetRedisForTest();
  });

  it("createSession returns 32-byte hex sid + persists in Redis", async () => {
    const sid = await createSession(sample, 300);
    expect(sid).toMatch(/^[0-9a-f]{64}$/);
    const got = await getSession(sid);
    expect(got).toEqual(sample);
  });

  it("getSession returns null for unknown sid", async () => {
    const got = await getSession("nope");
    expect(got).toBeNull();
  });

  it("deleteSession removes the entry", async () => {
    const sid = await createSession(sample, 300);
    await deleteSession(sid);
    expect(await getSession(sid)).toBeNull();
  });

  it("refreshSession overwrites + extends TTL", async () => {
    const sid = await createSession(sample, 300);
    const next: SessionData = {
      ...sample,
      jti: "jti-2",
      access_token: "at-2",
      exp: sample.exp + 300,
    };
    await refreshSession(sid, next, 300);
    expect((await getSession(sid))?.jti).toBe("jti-2");
  });

  it("getSession returns null on corrupted JSON", async () => {
    await getRedis().set("session:bad-sid", "{not valid json", "EX", 300);
    const got = await getSession("bad-sid");
    expect(got).toBeNull();
  });

  it("getSession returns null on missing fields", async () => {
    await getRedis().set("session:partial-sid", JSON.stringify({ sub: "u1" }), "EX", 300);
    const got = await getSession("partial-sid");
    expect(got).toBeNull();
  });
});
