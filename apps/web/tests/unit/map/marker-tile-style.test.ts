// @vitest-environment node
import { describe, expect, it } from "vitest";
import { GONGZZANG_MAP_ZOOM_POLICY } from "@/lib/map/map-zoom-policy";
import {
  buildListingMarkerDeltaLayerRegistration,
  buildListingMarkerLayerRegistration,
  buildParcelAnchorMarkerLayerRegistrations,
  LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID,
  LISTING_MARKER_DELTA_TILE_SOURCE_ID,
  LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
  LISTING_MARKER_TILE_SOURCE_ID,
  PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_CIRCLE_LAYER_ID,
  PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
  PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
  PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
} from "@/lib/map/marker-tile-style";
import { parseVectorTileManifest } from "@/lib/map/vector-tile-manifest";

const lineageFixture = {
  source_record_id: "018f0000-0000-7000-8000-000000000001",
  manifest_file_asset_id: "018f0000-0000-7000-8000-000000000002",
  tilejson_file_asset_id: "018f0000-0000-7000-8000-000000000003",
  source_file_asset_ids: ["018f0000-0000-7000-8000-000000000004"],
};

const anchorManifestFixture = {
  schema_version: 1,
  current_version: "019e5f6f-1e74-74f3-b5e4-3add804b4bae",
  previous_version: "019e5e71-c352-7c40-9621-4b34475c79eb",
  tiles_url_template: "https://static.example.com/{object_key_prefix}/{z}/{x}/{y}.pbf",
  published_at: "2026-05-27T00:00:00Z",
  artifacts: {
    parcel_anchor_aggregate: {
      source_layer: "parcel_anchor_aggregate",
      tile_min_zoom: 0,
      tile_max_zoom: 11,
      render_min_zoom: 0,
      render_max_zoom: 11,
      tilejson_object_key:
        "gold/parcel-marker-anchor-aggregate-pbf/019e649e-88b5-7f91-8574-3a35bcce84e4/tilejson.json",
      object_key_prefix:
        "gold/parcel-marker-anchor-aggregate-pbf/019e649e-88b5-7f91-8574-3a35bcce84e4",
      flat_tile_count: 914,
      flat_tile_total_bytes: 303565,
      lineage: lineageFixture,
    },
    parcel_anchor: {
      source_layer: "parcel_anchor",
      tile_min_zoom: 12,
      tile_max_zoom: 12,
      render_min_zoom: 12,
      render_max_zoom: 22,
      tilejson_object_key:
        "gold/parcel-marker-anchor-pbf/019e5f6f-1e74-74f3-b5e4-3add804b4bae/tilejson.json",
      object_key_prefix: "gold/parcel-marker-anchor-pbf/019e5f6f-1e74-74f3-b5e4-3add804b4bae",
      flat_tile_count: 2119,
      flat_tile_total_bytes: 2318455415,
      lineage: lineageFixture,
    },
  },
};

describe("parcel anchor marker tile map style", () => {
  it("registers aggregate and exact PNU-anchor layers from the platform-core manifest", () => {
    const registrations = buildParcelAnchorMarkerLayerRegistrations({
      manifest: parseVectorTileManifest(anchorManifestFixture),
    });

    expect(registrations).toHaveLength(2);
    expect(registrations[0]).toMatchObject({
      sourceId: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
      source: {
        type: "vector",
        tiles: [
          "https://static.example.com/gold/parcel-marker-anchor-aggregate-pbf/019e649e-88b5-7f91-8574-3a35bcce84e4/{z}/{x}/{y}.pbf",
        ],
        minzoom: 0,
        maxzoom: 11,
      },
      layers: [
        {
          id: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_CIRCLE_LAYER_ID,
          type: "circle",
          source: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
          "source-layer": "parcel_anchor_aggregate",
          minzoom: 0,
          maxzoom: 11,
        },
      ],
    });
    expect(registrations[1]).toMatchObject({
      sourceId: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
      source: {
        type: "vector",
        tiles: [
          "https://static.example.com/gold/parcel-marker-anchor-pbf/019e5f6f-1e74-74f3-b5e4-3add804b4bae/{z}/{x}/{y}.pbf",
        ],
        minzoom: 12,
        maxzoom: 12,
      },
      layers: [
        {
          id: PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
          type: "circle",
          source: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
          "source-layer": "parcel_anchor",
          minzoom: 12,
          maxzoom: 22,
        },
      ],
    });
    expect(JSON.stringify(registrations)).not.toContain("bbox=");
    expect(JSON.stringify(registrations)).not.toContain("bounds=");
    expect(JSON.stringify(registrations)).not.toContain("lat=");
    expect(JSON.stringify(registrations)).not.toContain("lng=");
  });

  it("registers Gongzzang listing marker source and circle layer without coordinate inputs", () => {
    const registration = buildListingMarkerLayerRegistration({
      filterHash: "all-active-v1",
      minzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom,
      maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
      origin: "http://localhost:3900",
    });

    expect(registration.sourceId).toBe(LISTING_MARKER_TILE_SOURCE_ID);
    expect(registration.source).toEqual({
      type: "vector",
      tiles: [
        "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 14,
      maxzoom: 22,
    });
    expect(registration.layers[0].id).toBe(LISTING_MARKER_TILE_CIRCLE_LAYER_ID);
    expect(registration.layers[0]["source-layer"]).toBe("listing");
    expect(registration.layers[0].minzoom).toBe(14);
    expect(registration.layers[0].maxzoom).toBe(22);
    expect(registration.source.tiles[0]).not.toContain("bbox=");
    expect(registration.source.tiles[0]).not.toContain("bounds=");
    expect(registration.source.tiles[0]).not.toContain("lat=");
    expect(registration.source.tiles[0]).not.toContain("lng=");
  });

  it("registers Gongzzang listing marker delta source with the listing delta layer", () => {
    const registration = buildListingMarkerDeltaLayerRegistration({
      baseVersion: 41,
      minzoom: 0,
      maxzoom: GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom,
      origin: "http://localhost:3900",
    });

    expect(registration.sourceId).toBe(LISTING_MARKER_DELTA_TILE_SOURCE_ID);
    expect(registration.source.tiles[0]).toBe(
      "http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41",
    );
    expect(registration.layers[0].id).toBe(LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID);
    expect(registration.layers[0]["source-layer"]).toBe("listing_delta");
  });
});
