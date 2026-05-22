// @vitest-environment node
import { describe, expect, it } from "vitest";
import { parseMarkerTileContract } from "@/lib/map/marker-tile-contract";
import {
  buildListingMarkerLayerRegistration,
  buildParcelAnchorMarkerLayerRegistration,
  LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
  LISTING_MARKER_TILE_SOURCE_ID,
  PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
  PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
} from "@/lib/map/marker-tile-style";

const contractFixture = {
  response_format: "mvt_pbf",
  position_source: "pnu_anchor",
  bbox_marker_runtime_forbidden: true,
  dropped_marker_success_forbidden: true,
  endpoint_template: "/map/v1/marker-tiles/{layer}/{z}/{x}/{y}.pbf?filter_hash={hash}",
  supported_layers: ["parcel_anchor"],
  default_filter_hash: "all-active-v1",
};

describe("parcel anchor marker tile map style", () => {
  it("registers a PNU-anchor PBF vector source and circle layer without bbox inputs", () => {
    const registration = buildParcelAnchorMarkerLayerRegistration({
      contract: parseMarkerTileContract(contractFixture),
      platformCoreBaseUrl: "https://platform-core.example.com/",
      minzoom: 8,
      maxzoom: 18,
    });

    expect(registration.sourceId).toBe(PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID);
    expect(registration.source).toEqual({
      type: "vector",
      tiles: [
        "https://platform-core.example.com/map/v1/marker-tiles/parcel_anchor/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 8,
      maxzoom: 18,
    });
    expect(registration.source.tiles[0]).not.toContain("bbox=");
    expect(registration.source.tiles[0]).not.toContain("bounds=");
    expect(registration.source.tiles[0]).not.toContain("lat=");
    expect(registration.source.tiles[0]).not.toContain("lng=");

    expect(registration.layers).toEqual([
      {
        id: PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
        type: "circle",
        source: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
        "source-layer": "parcel_anchor",
        minzoom: 8,
        maxzoom: 18,
        paint: {
          "circle-color": "#10b981",
          "circle-opacity": 0.92,
          "circle-radius": ["interpolate", ["linear"], ["zoom"], 8, 3, 14, 5, 18, 7],
          "circle-stroke-color": "#ffffff",
          "circle-stroke-opacity": 0.95,
          "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 8, 0.75, 14, 1, 18, 1.5],
        },
      },
    ]);
  });

  it("registers Gongzzang listing marker source and circle layer without coordinate inputs", () => {
    const registration = buildListingMarkerLayerRegistration({
      filterHash: "all-active-v1",
      minzoom: 8,
      maxzoom: 18,
      origin: "http://localhost:3900",
    });

    expect(registration.sourceId).toBe(LISTING_MARKER_TILE_SOURCE_ID);
    expect(registration.source).toEqual({
      type: "vector",
      tiles: [
        "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 8,
      maxzoom: 18,
    });
    expect(registration.layers[0].id).toBe(LISTING_MARKER_TILE_CIRCLE_LAYER_ID);
    expect(registration.layers[0]["source-layer"]).toBe("listing");
    expect(registration.source.tiles[0]).not.toContain("bbox=");
    expect(registration.source.tiles[0]).not.toContain("bounds=");
    expect(registration.source.tiles[0]).not.toContain("lat=");
    expect(registration.source.tiles[0]).not.toContain("lng=");
  });
});
