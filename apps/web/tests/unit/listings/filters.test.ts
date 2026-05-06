// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  type ListingFilters,
  parseFiltersFromSearchParams,
  toSearchParams,
} from "@/lib/listings/filters";

describe("parseFiltersFromSearchParams", () => {
  it("default filter (모두 빈 값)", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams());
    expect(f.types).toEqual([]);
    expect(f.transactions).toEqual([]);
    expect(f.minAreaM2).toBeUndefined();
    expect(f.sort).toBe("created_at_desc");
  });
  it("comma-separated types", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams("types=factory,warehouse"));
    expect(f.types).toEqual(["factory", "warehouse"]);
  });
  it("range parsing", () => {
    const f = parseFiltersFromSearchParams(
      new URLSearchParams(
        "min_area_m2=100&max_area_m2=2000&min_price_krw=0&max_price_krw=5000000000",
      ),
    );
    expect(f.minAreaM2).toBe(100);
    expect(f.maxAreaM2).toBe(2000);
    expect(f.minPriceKrw).toBe(0);
    expect(f.maxPriceKrw).toBe(5_000_000_000);
  });
});

describe("toSearchParams (round trip)", () => {
  it("filter → URLSearchParams → 동일 filter", () => {
    const f: ListingFilters = {
      types: ["factory", "office"],
      transactions: ["sale"],
      minAreaM2: 200,
      maxAreaM2: undefined,
      minPriceKrw: undefined,
      maxPriceKrw: undefined,
      sort: "price_asc",
      pnu: undefined,
      adminCode: undefined,
      landUseType: undefined,
    };
    const sp = toSearchParams(f);
    const back = parseFiltersFromSearchParams(sp);
    expect(back.types).toEqual(f.types);
    expect(back.transactions).toEqual(f.transactions);
    expect(back.minAreaM2).toBe(200);
    expect(back.sort).toBe("price_asc");
  });
});
