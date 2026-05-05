import Redis from "ioredis";
import { env } from "@/lib/env";

let _client: Redis | null = null;

export function getRedis(): Redis {
  if (_client === null) {
    _client = new Redis(env.REDIS_URL, {
      maxRetriesPerRequest: 3,
      enableReadyCheck: true,
      lazyConnect: false,
    });
  }
  return _client;
}

// 테스트용 — Redis client 강제 reset
export function __resetRedisForTest(): void {
  if (_client) {
    _client.disconnect();
    _client = null;
  }
}
