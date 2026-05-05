import { createHmac, timingSafeEqual } from "node:crypto";
import { env } from "@/lib/env";

export const SID_COOKIE_NAME = "__Host-sid";
export const TEMP_COOKIE_NAME = "__Host-auth-tmp";

/**
 * Temp cookie payload 를 HMAC-SHA256 sign + base64url encode.
 * Format: "<base64url(payload)>.<base64url(hmac)>"
 */
export function signTempPayload(payload: string): string {
  const payloadB64 = Buffer.from(payload).toString("base64url");
  const mac = createHmac("sha256", env.SESSION_SECRET).update(payloadB64).digest();
  const macB64 = mac.toString("base64url");
  return `${payloadB64}.${macB64}`;
}

/**
 * HMAC 검증 후 payload 반환. tampering 시 null.
 */
export function verifyTempPayload(signed: string): string | null {
  const dot = signed.indexOf(".");
  if (dot === -1) return null;
  const payloadB64 = signed.slice(0, dot);
  const macB64 = signed.slice(dot + 1);
  const expectedMac = createHmac("sha256", env.SESSION_SECRET).update(payloadB64).digest();
  let actualMac: Buffer;
  try {
    actualMac = Buffer.from(macB64, "base64url");
  } catch {
    return null;
  }
  if (actualMac.length !== expectedMac.length) return null;
  if (!timingSafeEqual(actualMac, expectedMac)) return null;
  return Buffer.from(payloadB64, "base64url").toString("utf-8");
}

export function setSidCookie(sid: string, maxAgeSec: number): string {
  // __Host- prefix 는 Domain 속성 금지, Path=/ 필수, Secure 필수
  return [
    `${SID_COOKIE_NAME}=${sid}`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    `Max-Age=${maxAgeSec}`,
    "Partitioned",
  ].join("; ");
}

export function deleteSidCookie(): string {
  return [
    `${SID_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    "Max-Age=0",
    "Partitioned",
  ].join("; ");
}

export interface TempAuthState {
  code_verifier: string;
  state: string;
  nonce: string;
  return_to: string;
}

export function setTempCookie(payload: string, maxAgeSec: number): string {
  return [
    `${TEMP_COOKIE_NAME}=${payload}`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax", // OAuth callback 은 cross-site GET 이라 Strict 불가
    "Path=/api/auth/",
    `Max-Age=${maxAgeSec}`,
  ].join("; ");
}

export function deleteTempCookie(): string {
  return [
    `${TEMP_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax",
    "Path=/api/auth/",
    "Max-Age=0",
  ].join("; ");
}
