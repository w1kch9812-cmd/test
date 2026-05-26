// @vitest-environment node
import { describe, expect, it } from "vitest";
import { buildListingMarkerLayerFilter } from "@/lib/map/listing-marker-filter";

describe("buildListingMarkerLayerFilter", () => {
  it("returns an all expression when fast filters are empty", () => {
    expect(
      buildListingMarkerLayerFilter({
        types: [],
        transactions: [],
        minAreaM2: undefined,
        maxAreaM2: undefined,
        minPriceKrw: undefined,
        maxPriceKrw: undefined,
        sort: "created_at_desc",
        adminCode: undefined,
        landUseType: undefined,
      }),
    ).toEqual(["all"]);
  });

  it("builds type transaction price and area predicates", () => {
    expect(
      buildListingMarkerLayerFilter({
        types: ["factory", "industrial_land"],
        transactions: ["sale"],
        minAreaM2: 300,
        maxAreaM2: 1000,
        minPriceKrw: 100_000_000,
        maxPriceKrw: 5_000_000_000,
        sort: "created_at_desc",
        adminCode: undefined,
        landUseType: undefined,
      }),
    ).toEqual([
      "all",
      ["in", ["get", "listing_type"], ["literal", ["factory", "industrial_land"]]],
      ["in", ["get", "transaction_type"], ["literal", ["sale"]]],
      [">=", ["to-number", ["get", "area_m2"]], 300],
      ["<=", ["to-number", ["get", "area_m2"]], 1000],
      [">=", ["to-number", ["get", "price_krw"]], 100_000_000],
      ["<=", ["to-number", ["get", "price_krw"]], 5_000_000_000],
    ]);
  });
});
