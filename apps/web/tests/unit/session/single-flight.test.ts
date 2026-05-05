import { afterAll, beforeEach, describe, expect, it } from "vitest";
import { __resetRedisForTest, getRedis } from "@/lib/session/redis";
import { acquireLock, releaseLock, withLock } from "@/lib/session/single-flight";

describe("Single-flight Redis mutex", () => {
  beforeEach(async () => {
    // db 2 — isolated from store tests (db 1) for parallel vitest workers
    await getRedis().select(2);
    await getRedis().flushdb();
  });

  afterAll(() => {
    __resetRedisForTest();
  });

  it("acquireLock succeeds on first call", async () => {
    const got = await acquireLock("k1", 10);
    expect(got).not.toBeNull();
  });

  it("acquireLock returns null when already held", async () => {
    await acquireLock("k2", 10);
    const second = await acquireLock("k2", 10);
    expect(second).toBeNull();
  });

  it("releaseLock with correct token deletes the lock", async () => {
    const tok = await acquireLock("k3", 10);
    if (!tok) throw new Error("expected lock token");
    await releaseLock("k3", tok);
    const next = await acquireLock("k3", 10);
    expect(next).not.toBeNull();
  });

  it("releaseLock with wrong token does NOT delete (compare-and-delete)", async () => {
    await acquireLock("k4", 10);
    await releaseLock("k4", "wrong-token");
    const next = await acquireLock("k4", 10);
    expect(next).toBeNull();
  });

  it("withLock acquires + runs + releases", async () => {
    const result = await withLock("k5", 10, async () => "value");
    expect(result).toBe("value");
    const next = await acquireLock("k5", 10);
    expect(next).not.toBeNull();
  });

  it("withLock retries when locked, eventually returning lockHeld result", async () => {
    await acquireLock("k6", 10);
    const result = await withLock("k6", 10, async () => "ran", {
      onLocked: async () => "skipped",
      maxRetries: 0,
    });
    expect(result).toBe("skipped");
  });
});
