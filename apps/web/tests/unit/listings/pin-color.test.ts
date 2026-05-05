import { describe, expect, it } from "vitest";
import { getPinColor, LISTING_TYPE_COLORS } from "@/lib/listings/pin-color";

describe("getPinColor", () => {
  it("6 종 매물 모두 hex color 반환", () => {
    expect(getPinColor("factory")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("warehouse")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("office")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("knowledge_industry_center")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("industrial_land")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("logistics_center")).toMatch(/^#[0-9a-f]{6}$/i);
  });
  it("6 종 모두 unique color", () => {
    const colors = new Set(Object.values(LISTING_TYPE_COLORS));
    expect(colors.size).toBe(6);
  });
});
