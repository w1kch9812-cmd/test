import { createHmac, timingSafeEqual } from "node:crypto";
import { env } from "@/lib/env";

const isProd = process.env.NODE_ENV === "production";

// Production: __Host-/__Secure- prefix + Secure flag (HTTPS strict).
// Dev (localhost HTTP): prefix + Secure 제거 (browser 가 HTTP 에서 거부).
export const SID_COOKIE_NAME = isProd ? "__Host-sid" : "sid";
export const TEMP_COOKIE_NAME = isProd ? "__Secure-auth-tmp" : "auth-tmp";

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

function buildSidCookie(value: string, maxAgeSec: number): string {
  const parts = [`${SID_COOKIE_NAME}=${value}`];
  if (isProd) parts.push("Secure", "Partitioned");
  parts.push("HttpOnly", "SameSite=Strict", "Path=/", `Max-Age=${maxAgeSec}`);
  return parts.join("; ");
}

export function setSidCookie(sid: string, maxAgeSec: number): string {
  return buildSidCookie(sid, maxAgeSec);
}

export function deleteSidCookie(): string {
  return buildSidCookie("", 0);
}

export interface TempAuthState {
  code_verifier: string;
  state: string;
  nonce: string;
  return_to: string;
}

function buildTempCookie(value: string, maxAgeSec: number): string {
  const parts = [`${TEMP_COOKIE_NAME}=${value}`];
  if (isProd) parts.push("Secure");
  parts.push("HttpOnly", "SameSite=Lax", "Path=/api/auth/", `Max-Age=${maxAgeSec}`);
  return parts.join("; ");
}

export function setTempCookie(payload: string, maxAgeSec: number): string {
  return buildTempCookie(payload, maxAgeSec);
}

export function deleteTempCookie(): string {
  return buildTempCookie("", 0);
}
