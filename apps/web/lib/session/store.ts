import { randomBytes } from "node:crypto";
import { getRedis } from "./redis";

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
  return JSON.parse(raw) as SessionData;
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
