import { randomBytes } from "node:crypto";
import type { BrowserContext } from "@playwright/test";
import { Redis } from "ioredis";

const SID_COOKIE_NAME = "sid";
const SESSION_TTL_SEC = 60 * 60;

interface E2eSessionOptions {
  role?: string;
}

export async function plantAuthenticatedSession(
  context: BrowserContext,
  options: E2eSessionOptions = {},
): Promise<string> {
  const sid = randomBytes(32).toString("hex");
  const redis = new Redis(process.env.REDIS_URL || "redis://localhost:6379");
  const session = {
    sub: "playwright",
    jti: `jti-${sid}`,
    role: options.role ?? "Buyer",
    access_token: "playwright-access-token",
    refresh_token: "playwright-refresh-token",
    id_token: "playwright-id-token",
    exp: Math.floor(Date.now() / 1000) + SESSION_TTL_SEC,
  };

  try {
    await redis.set(`session:${sid}`, JSON.stringify(session), "EX", SESSION_TTL_SEC);
  } finally {
    redis.disconnect();
  }

  await context.addCookies([
    {
      name: SID_COOKIE_NAME,
      value: sid,
      url: "http://localhost:3000",
      httpOnly: true,
      sameSite: "Strict",
    },
  ]);

  return sid;
}
