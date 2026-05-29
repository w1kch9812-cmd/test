// @vitest-environment node
import { describe, expect, it } from "vitest";
import { ListingCardSchema } from "@/lib/listings/api";

describe("ListingCardSchema (zod)", () => {
  it("매매 매물 — deposit/rent/thumbnail 키 자체 없음 → parse PASS", () => {
    const json = {
      id: "lst_x",
      title: "공장 매매",
      listing_type: "factory",
      transaction_type: "sale",
      price_krw: 8_000_000_000,
      // deposit_krw, monthly_rent_krw, thumbnail_url 키 자체 없음
      area_m2: 3960,
      view_count: 0,
      bookmark_count: 0,
      is_bookmarked: false,
      created_at: "2026-04-12T09:30:00Z",
    };
    const parsed = ListingCardSchema.parse(json);
    expect(parsed.deposit_krw).toBeUndefined();
    expect(parsed.thumbnail_url).toBeUndefined();
  });

  it("월세 매물 — deposit + monthly_rent 둘 다 number", () => {
    const json = {
      id: "lst_y",
      title: "사무실 월세",
      listing_type: "office",
      transaction_type: "monthly_rent",
      price_krw: 1_000_000,
      deposit_krw: 30_000_000,
      monthly_rent_krw: 1_000_000,
      thumbnail_url: null, // null 도 OK
      area_m2: 100,
      view_count: 0,
      bookmark_count: 0,
      is_bookmarked: false,
      created_at: "2026-04-12T09:30:00Z",
    };
    const parsed = ListingCardSchema.parse(json);
    expect(parsed.deposit_krw).toBe(30_000_000);
    expect(parsed.monthly_rent_krw).toBe(1_000_000);
    expect(parsed.thumbnail_url).toBeNull();
  });
});
