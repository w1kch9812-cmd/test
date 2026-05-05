import { describe, expect, it } from "vitest";
import { sanitizeReturnTo } from "@/lib/url";

describe("sanitizeReturnTo", () => {
  it("allows relative path", () => {
    expect(sanitizeReturnTo("/profile")).toBe("/profile");
    expect(sanitizeReturnTo("/listings?page=2")).toBe("/listings?page=2");
  });
  it("blocks absolute http(s)", () => {
    expect(sanitizeReturnTo("https://evil.com")).toBe("/profile");
    expect(sanitizeReturnTo("http://evil.com/x")).toBe("/profile");
  });
  it("blocks protocol-relative", () => {
    expect(sanitizeReturnTo("//evil.com")).toBe("/profile");
  });
  it("blocks backslash trick", () => {
    expect(sanitizeReturnTo("/\\evil.com")).toBe("/profile");
  });
  it("falls back on null/empty", () => {
    expect(sanitizeReturnTo(null)).toBe("/profile");
    expect(sanitizeReturnTo("")).toBe("/profile");
    expect(sanitizeReturnTo(undefined)).toBe("/profile");
  });
});
