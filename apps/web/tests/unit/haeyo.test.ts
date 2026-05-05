import { describe, expect, it } from "vitest";
import {
  formatAreaM2,
  formatCount,
  formatKrw,
  formatNumber,
  formatRelativeTime,
} from "@/lib/i18n/haeyo";

describe("haeyo utils", () => {
  it("formatKrw — 천 단위 콤마 + 원 기호", () => {
    expect(formatKrw(1234567)).toMatch(/1,234,567/);
  });

  it("formatNumber — 한국어 천 단위 콤마", () => {
    expect(formatNumber(1234567)).toBe("1,234,567");
  });

  it("formatAreaM2 — m² 단위", () => {
    expect(formatAreaM2(100)).toBe("100m²");
    expect(formatAreaM2(123.456)).toBe("123.5m²");
  });

  it("formatCount — n + 단위", () => {
    expect(formatCount(3, "개")).toBe("3개");
    expect(formatCount(10000, "건")).toBe("10,000건");
  });

  it("formatRelativeTime — 5분 전", () => {
    const fiveMinAgo = new Date(Date.now() - 5 * 60 * 1000);
    const result = formatRelativeTime(fiveMinAgo);
    expect(result).toMatch(/분/);
  });
});
