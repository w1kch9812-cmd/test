import { randomBytes } from "node:crypto";
import { getRedis } from "./redis";

/** Session sid cookie + Redis entry TTL (refresh_token 만료 일치 — 30일). */
export const REFRESH_TTL_SEC = 30 * 24 * 60 * 60;

export interface SessionData {
  sub: string; // Zitadel sub
  jti: string; // current access_token JTI
  role: string; // 'Buyer' | 'Seller' | 'Broker' | ...
  access_token: string;
  refresh_token: string;
  id_token: string; // back-channel logout 용
  exp: number; // access_token exp (epoch sec)
}

const KEY = (sid: string) => `session:${sid}`;

export async function createSession(data: SessionData, ttlSec: number): Promise<string> {
  const sid = randomBytes(32).toString("hex");
  await getRedis().set(KEY(sid), JSON.stringify(data), "EX", ttlSec);
  return sid;
}

export async function getSession(sid: string): Promise<SessionData | null> {
  const raw = await getRedis().get(KEY(sid));
  if (!raw) return null;
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return null;
  }
  if (
    typeof data !== "object" ||
    data === null ||
    typeof (data as SessionData).sub !== "string" ||
    typeof (data as SessionData).jti !== "string" ||
    typeof (data as SessionData).role !== "string" ||
    typeof (data as SessionData).access_token !== "string" ||
    typeof (data as SessionData).refresh_token !== "string" ||
    typeof (data as SessionData).id_token !== "string" ||
    typeof (data as SessionData).exp !== "number"
  ) {
    return null;
  }
  return data as SessionData;
}

export async function deleteSession(sid: string): Promise<void> {
  await getRedis().del(KEY(sid));
}

export async function refreshSession(
  sid: string,
  next: SessionData,
  ttlSec: number,
): Promise<void> {
  await getRedis().set(KEY(sid), JSON.stringify(next), "EX", ttlSec);
}
