// @vitest-environment node
import { describe, expect, it } from "vitest";
import { GONGZZANG_MAP_ZOOM_POLICY } from "@/lib/map/map-zoom-policy";

describe("Gongzzang map zoom policy", () => {
  it("keeps product-owned listing marker visibility at parcel zoom while platform-core anchors can start earlier", () => {
    expect(GONGZZANG_MAP_ZOOM_POLICY.platformCore.exactParcelAnchorMinZoom).toBe(12);
    expect(GONGZZANG_MAP_ZOOM_POLICY.levels.parcel.min).toBe(14);
    expect(GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom).toBe(
      GONGZZANG_MAP_ZOOM_POLICY.levels.parcel.min,
    );
    expect(GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom).toBe(
      GONGZZANG_MAP_ZOOM_POLICY.levels.parcel.max,
    );
  });
});
