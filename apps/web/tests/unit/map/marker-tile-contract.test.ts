// @vitest-environment node
import { describe, expect, it } from "vitest";
import { GONGZZANG_MAP_ZOOM_POLICY } from "@/lib/map/map-zoom-policy";
import {
  ALL_ACTIVE_MARKER_FILTER_HASH,
  buildListingMarkerTileSource,
  LISTING_MARKER_TILE_LAYER,
} from "@/lib/map/marker-tile-contract";

describe("Gongzzang listing marker tile source", () => {
  it("builds the Gongzzang-owned listing marker vector source through same-origin proxy", () => {
    const source = buildListingMarkerTileSource({
      filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
      minzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom,
      maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
      origin: "http://localhost:3900",
    });

    expect(source).toEqual({
      type: "vector",
      tiles: [
        "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 14,
      maxzoom: 22,
    });
    expect(new URL(source.tiles[0]).origin).toBe("http://localhost:3900");
    expect(LISTING_MARKER_TILE_LAYER).toBe("listing");
    expect(source.tiles[0]).not.toContain("bounds=");
    expect(source.tiles[0]).not.toContain("bbox=");
    expect(source.tiles[0]).not.toContain("lat=");
    expect(source.tiles[0]).not.toContain("lng=");
  });

  it("accepts Gongzzang-owned registered listing marker filter hashes", () => {
    const registeredHash = `lst_filter_v1_${"a".repeat(64)}`;

    const source = buildListingMarkerTileSource({
      filterHash: registeredHash,
      minzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom,
      maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
      origin: "http://localhost:3900",
    });

    expect(source).toEqual({
      type: "vector",
      tiles: [
        `http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=${registeredHash}`,
      ],
      minzoom: 14,
      maxzoom: 22,
    });
  });

  it("rejects listing marker sources with unstable filter identifiers or invalid zoom ranges", () => {
    expect(() =>
      buildListingMarkerTileSource({
        filterHash: "listing filter with spaces",
        minzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom,
        maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
        origin: "http://localhost:3900",
      }),
    ).toThrow(/filterHash/);

    expect(() =>
      buildListingMarkerTileSource({
        filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
        minzoom: 19,
        maxzoom: 18,
        origin: "http://localhost:3900",
      }),
    ).toThrow(/zoom range/);
  });
});
