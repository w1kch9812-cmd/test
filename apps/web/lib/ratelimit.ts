import { getRedis } from "./session/redis";

// Sliding-window: ZSET 에 timestamp, ZREMRANGEBYSCORE 로 window 밖 제거 후 ZCARD 검사.
// member 고유성: INCR counter (per-key sequence) — math.random() 은 Lua sandbox 에서
// deterministic seed 로 동시 호출 시 같은 member 가 생성되어 ZADD update 가 되는 버그 수정.
const RATE_LUA = `
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
redis.call("ZREMRANGEBYSCORE", key, 0, now - window_ms)
local count = redis.call("ZCARD", key)
if count >= limit then
  return {0, 0}
end
local seq = redis.call("INCR", key .. ":seq")
redis.call("EXPIRE", key .. ":seq", math.ceil(window_ms / 1000) + 60)
redis.call("ZADD", key, now, now .. ":" .. seq)
redis.call("PEXPIRE", key, window_ms)
return {1, limit - count - 1}
`;

export interface RateResult {
  allowed: boolean;
  remaining: number;
}

export async function checkRate(
  key: string,
  limit: number,
  windowSec: number,
): Promise<RateResult> {
  const r = (await getRedis().eval(
    RATE_LUA,
    1,
    `rate:${key}`,
    Date.now(),
    windowSec * 1000,
    limit,
  )) as [number, number];
  return { allowed: r[0] === 1, remaining: r[1] };
}
