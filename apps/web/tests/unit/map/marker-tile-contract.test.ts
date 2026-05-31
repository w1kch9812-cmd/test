// @vitest-environment node
import { describe, expect, it } from "vitest";
import { GONGZZANG_MAP_ZOOM_POLICY } from "@/lib/map/map-zoom-policy";
import {
  ALL_ACTIVE_MARKER_FILTER_HASH,
  buildListingMarkerDeltaTileSource,
  buildListingMarkerTileSource,
  buildListingMarkerTombstoneUrl,
  createListingMarkerOverlayState,
  LISTING_MARKER_DELTA_TILE_LAYER,
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

  it("builds delta source and tombstone URL without viewport-bound request shapes", () => {
    const delta = buildListingMarkerDeltaTileSource({
      baseVersion: 41,
      minzoom: 0,
      maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
      origin: "http://localhost:3900",
    });
    const tombstoneUrl = buildListingMarkerTombstoneUrl({
      z: 14,
      x: 13970,
      y: 6344,
      baseVersion: 41,
      origin: "http://localhost:3900",
    });

    expect(delta.tiles[0]).toBe(
      "http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41",
    );
    expect(tombstoneUrl).toBe(
      "http://localhost:3900/api/proxy/map/v1/marker-tombstones/listing/14/13970/6344?base_version=41",
    );
    expect(LISTING_MARKER_DELTA_TILE_LAYER).toBe("listing_delta");
    expect(`${delta.tiles[0]} ${tombstoneUrl}`).not.toContain("bbox=");
    expect(`${delta.tiles[0]} ${tombstoneUrl}`).not.toContain("bounds=");
  });

  it("creates overlay state with tombstones as the hide set", () => {
    const state = createListingMarkerOverlayState({
      baseVersion: 41,
      tombstoneIds: ["lm_lst_01HXY3NK0Z9F6S1B2C3D4E5F6G"],
    });

    expect(state.baseVersion).toBe(41);
    expect(state.deltaSourceId).toBe("listing_delta");
    expect([...state.tombstoneIds]).toEqual(["lm_lst_01HXY3NK0Z9F6S1B2C3D4E5F6G"]);
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
