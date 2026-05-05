import { afterAll, beforeEach, describe, expect, it } from "vitest";
import { checkRate } from "@/lib/ratelimit";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";

describe("Redis sliding window ratelimit", () => {
  beforeEach(async () => {
    await getRedis().select(3); // T2 store db 1, single-flight db 2 — ratelimit db 3
    await getRedis().flushdb();
  });

  afterAll(() => __resetRedisForTest());

  it("allows up to limit then denies", async () => {
    for (let i = 0; i < 5; i++) {
      const r = await checkRate("login:1.2.3.4", 5, 60);
      expect(r.allowed).toBe(true);
      expect(r.remaining).toBe(5 - i - 1);
    }
    const denied = await checkRate("login:1.2.3.4", 5, 60);
    expect(denied.allowed).toBe(false);
    expect(denied.remaining).toBe(0);
  });

  it("isolates keys", async () => {
    for (let i = 0; i < 5; i++) await checkRate("a", 5, 60);
    const r = await checkRate("b", 5, 60);
    expect(r.allowed).toBe(true);
  });
});
