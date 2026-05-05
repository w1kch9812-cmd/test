// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  deleteSidCookie,
  SID_COOKIE_NAME,
  setSidCookie,
  signTempPayload,
  verifyTempPayload,
} from "@/lib/session/cookie";

describe("__Host- sid cookie helpers", () => {
  it("uses __Host- prefix and security flags", () => {
    expect(SID_COOKIE_NAME).toBe("__Host-sid");
  });

  it("setSidCookie returns Set-Cookie with all required flags", () => {
    const setCookie = setSidCookie("abc123", 86400);
    expect(setCookie).toContain("__Host-sid=abc123");
    expect(setCookie).toContain("Secure");
    expect(setCookie).toContain("HttpOnly");
    expect(setCookie).toContain("SameSite=Strict");
    expect(setCookie).toContain("Path=/");
    expect(setCookie).toContain("Max-Age=86400");
    expect(setCookie).toContain("Partitioned");
  });

  it("deleteSidCookie returns Set-Cookie with Max-Age=0", () => {
    const setCookie = deleteSidCookie();
    expect(setCookie).toContain("__Host-sid=");
    expect(setCookie).toContain("Max-Age=0");
  });
});

describe("signTempPayload / verifyTempPayload", () => {
  it("roundtrips correctly", () => {
    const original = JSON.stringify({ x: 1 });
    const signed = signTempPayload(original);
    expect(verifyTempPayload(signed)).toBe(original);
  });

  it("returns null on tampered MAC", () => {
    const signed = signTempPayload("data");
    const tampered = `${signed.slice(0, -2)}XX`;
    expect(verifyTempPayload(tampered)).toBe(null);
  });

  it("returns null on tampered payload", () => {
    // Use different data so the base64url-encoded payload changes but MAC remains from original
    const signed = signTempPayload("data");
    const dot = signed.indexOf(".");
    const originalMac = signed.slice(dot);
    // "evil" base64url-encoded is "ZXZpbA" — different payload, same MAC → should fail
    const tampered = `ZXZpbA${originalMac}`;
    expect(verifyTempPayload(tampered)).toBe(null);
  });

  it("returns null when no dot separator", () => {
    expect(verifyTempPayload("nodothere")).toBe(null);
  });
});
