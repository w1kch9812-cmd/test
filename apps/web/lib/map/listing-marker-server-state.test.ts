// @vitest-environment node
import { describe, expect, it } from "vitest";
import type { ListingFilters } from "@/lib/listings/filters";
import {
  buildListingMarkerFilterRequest,
  buildListingMarkerServerKey,
} from "@/lib/map/listing-marker-server-state";

describe("listing marker server state", () => {
  it("builds a stable request coalescing key from filter and projection metadata", () => {
    expect(
      buildListingMarkerServerKey({
        filterHash: "all-active-v1",
        projectionVersion: 123,
        anchorSnapshotId: "snapshot-test-v1",
      }),
    ).toBe("listing|all-active-v1|123|snapshot-test-v1");
  });

  it("serializes fast marker filters to the backend snake_case request shape", () => {
    const filters: ListingFilters = {
      types: ["factory"],
      transactions: ["sale"],
      minAreaM2: 100,
      maxAreaM2: 500,
      minPriceKrw: 100_000_000,
      maxPriceKrw: 900_000_000,
      sort: "created_at_desc",
      adminCode: "11110101",
      landUseType: "factory_site",
    };

    expect(buildListingMarkerFilterRequest(filters)).toEqual({
      types: ["factory"],
      transactions: ["sale"],
      min_area_m2: 100,
      max_area_m2: 500,
      min_price_krw: 100_000_000,
      max_price_krw: 900_000_000,
    });
  });
});
