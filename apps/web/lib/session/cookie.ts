import { createHmac, timingSafeEqual } from "node:crypto";
import { env } from "@/lib/env";

const isProd = process.env.NODE_ENV === "production";

// Production: __Host-/__Secure- prefix + Secure flag (HTTPS strict).
// Dev runs on localhost HTTP, so the Secure-only prefixes are disabled there.
export const SID_COOKIE_NAME = isProd ? "__Host-sid" : "sid";
export const AUTH_STATE_COOKIE_NAME = isProd ? "__Secure-auth-state" : "auth-state";

/**
 * HMAC-SHA256 sign and base64url encode the OAuth auth-state cookie payload.
 * Format: "<base64url(payload)>.<base64url(hmac)>"
 */
export function signAuthStatePayload(payload: string): string {
  const payloadB64 = Buffer.from(payload).toString("base64url");
  const mac = createHmac("sha256", env.SESSION_SECRET).update(payloadB64).digest();
  const macB64 = mac.toString("base64url");
  return `${payloadB64}.${macB64}`;
}

/**
 * Verify the HMAC signature and return the payload, or null when tampered.
 */
export function verifyAuthStatePayload(signed: string): string | null {
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

export interface AuthStateCookiePayload {
  code_verifier: string;
  state: string;
  nonce: string;
  return_to: string;
}

function buildAuthStateCookie(value: string, maxAgeSec: number): string {
  const parts = [`${AUTH_STATE_COOKIE_NAME}=${value}`];
  if (isProd) parts.push("Secure");
  parts.push("HttpOnly", "SameSite=Lax", "Path=/api/auth/", `Max-Age=${maxAgeSec}`);
  return parts.join("; ");
}

export function setAuthStateCookie(payload: string, maxAgeSec: number): string {
  return buildAuthStateCookie(payload, maxAgeSec);
}

export function deleteAuthStateCookie(): string {
  return buildAuthStateCookie("", 0);
}
