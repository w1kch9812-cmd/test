import { randomBytes } from "node:crypto";
import { getRedis } from "./redis";

const KEY = (k: string) => `lock:${k}`;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function acquireLock(key: string, ttlSec: number): Promise<string | null> {
  const token = randomBytes(16).toString("hex");
  const got = await getRedis().set(KEY(key), token, "EX", ttlSec, "NX");
  return got === "OK" ? token : null;
}

// CAS 삭제 — 다른 holder 의 lock 은 지우지 않음
const RELEASE_LUA = `
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
else
  return 0
end`;

export async function releaseLock(key: string, token: string): Promise<void> {
  await getRedis().eval(RELEASE_LUA, 1, KEY(key), token);
}

export interface WithLockOptions<T> {
  onLocked?: () => Promise<T>;
  maxRetries?: number; // default 3
  retryDelayMs?: number; // default 100
}

export async function withLock<T>(
  key: string,
  ttlSec: number,
  run: () => Promise<T>,
  options: WithLockOptions<T> = {},
): Promise<T> {
  const maxRetries = options.maxRetries ?? 3;
  const retryDelayMs = options.retryDelayMs ?? 100;

  for (let i = 0; i <= maxRetries; i++) {
    const tok = await acquireLock(key, ttlSec);
    if (tok) {
      try {
        return await run();
      } finally {
        await releaseLock(key, tok);
      }
    }
    if (i < maxRetries) await sleep(retryDelayMs);
  }
  if (options.onLocked) return options.onLocked();
  throw new Error(`failed to acquire lock '${key}' after ${maxRetries} retries`);
}
