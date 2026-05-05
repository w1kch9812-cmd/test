// @vitest-environment node
import { describe, expect, it } from "vitest";
import { formatAreaM2, formatAreaPyeong, formatPriceKrw, m2ToPyeong } from "@/lib/listings/format";

describe("formatPriceKrw — 한국 가격 표기", () => {
  it("1조 이상", () => {
    expect(formatPriceKrw(1_500_000_000_000)).toBe("1조 5,000억원");
  });
  it("억 + 만원", () => {
    expect(formatPriceKrw(8_500_000_000)).toBe("85억원");
    expect(formatPriceKrw(123_450_000)).toBe("1억 2,345만원");
  });
  it("만원 단위", () => {
    expect(formatPriceKrw(50_000_000)).toBe("5,000만원");
  });
  it("원 단위", () => {
    expect(formatPriceKrw(800_000)).toBe("800,000원");
  });
  it("0", () => {
    expect(formatPriceKrw(0)).toBe("0원");
  });
});

describe("m2ToPyeong + formatAreaPyeong", () => {
  it("1평 = 3.305 m²", () => {
    expect(m2ToPyeong(3.305)).toBeCloseTo(1.0, 1);
  });
  it("formatAreaPyeong 소수점 1자리", () => {
    expect(formatAreaPyeong(330.5)).toBe("100.0평");
    expect(formatAreaPyeong(33.05)).toBe("10.0평");
  });
});

describe("formatAreaM2", () => {
  it("정수 + 천단위 콤마", () => {
    expect(formatAreaM2(3960.5)).toBe("3,961㎡");
  });
});
