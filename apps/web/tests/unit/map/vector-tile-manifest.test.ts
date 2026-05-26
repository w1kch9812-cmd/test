// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  buildVectorTileSource,
  fetchVectorTileManifest,
  PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER,
  PARCEL_ANCHOR_VECTOR_TILE_LAYER,
  parseVectorTileManifest,
  resolveVectorTileAllowedOrigins,
  resolveVectorTileManifestUrl,
  resolveVectorTileRuntimeEnv,
} from "@/lib/map/vector-tile-manifest";

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

describe("platform-core vector tile manifest consumer", () => {
  const livePlatformCoreBaseUrl = process.env.PLATFORM_CORE_MANIFEST_LIVE_BASE_URL;
  const liveIt = livePlatformCoreBaseUrl ? it : it.skip;

  it("accepts the active PNU-anchor manifest without requiring parcel polygon artifacts", () => {
    const manifest = parseVectorTileManifest(anchorManifestFixture);

    expect(manifest.current_version).toBe("019e5f6f-1e74-74f3-b5e4-3add804b4bae");
    expect(manifest.artifacts.parcels).toBeUndefined();
    expect(manifest.artifacts.parcel_anchor?.source_layer).toBe("parcel_anchor");
    expect(manifest.artifacts.parcel_anchor_aggregate?.source_layer).toBe(
      "parcel_anchor_aggregate",
    );
  });

  it("rejects legacy version/layer tile templates instead of keeping a second SSOT", () => {
    expect(() =>
      parseVectorTileManifest({
        ...anchorManifestFixture,
        tiles_url_template: "https://static.example.com/{version}/{layer}/{z}/{x}/{y}.pbf",
      }),
    ).toThrow(/object_key_prefix/);
  });

  it("builds aggregate and exact anchor sources from artifact object_key_prefix", () => {
    const manifest = parseVectorTileManifest(anchorManifestFixture);

    expect(buildVectorTileSource(manifest, PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER)).toEqual({
      type: "vector",
      tiles: [
        "https://static.example.com/gold/parcel-marker-anchor-aggregate-pbf/019e649e-88b5-7f91-8574-3a35bcce84e4/{z}/{x}/{y}.pbf",
      ],
      minzoom: 0,
      maxzoom: 11,
    });
    expect(buildVectorTileSource(manifest, PARCEL_ANCHOR_VECTOR_TILE_LAYER)).toEqual({
      type: "vector",
      tiles: [
        "https://static.example.com/gold/parcel-marker-anchor-pbf/019e5f6f-1e74-74f3-b5e4-3add804b4bae/{z}/{x}/{y}.pbf",
      ],
      minzoom: 12,
      maxzoom: 12,
    });
  });

  it("resolves root-relative public manifest tile templates against the manifest origin", async () => {
    const fetcher = async (input: RequestInfo | URL, init?: RequestInit) => {
      expect(String(input)).toBe("https://static.example.com/gold/manifest.json");
      expect(init?.headers).toEqual({ accept: "application/json" });
      expect(init?.cache).toBe("no-store");
      return Response.json({
        ...anchorManifestFixture,
        tiles_url_template: "/{object_key_prefix}/{z}/{x}/{y}.pbf",
      });
    };

    const manifest = await fetchVectorTileManifest(fetcher, {
      NEXT_PUBLIC_TILES_MANIFEST_URL: "https://static.example.com/gold/manifest.json",
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: undefined,
    });

    expect(buildVectorTileSource(manifest, PARCEL_ANCHOR_VECTOR_TILE_LAYER).tiles).toEqual([
      "https://static.example.com/gold/parcel-marker-anchor-pbf/019e5f6f-1e74-74f3-b5e4-3add804b4bae/{z}/{x}/{y}.pbf",
    ]);
  });

  it("resolves platform-core Catalog endpoint before legacy static tile base", () => {
    const url = resolveVectorTileManifestUrl({
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: "https://platform-core.internal/",
      NEXT_PUBLIC_TILES_BASE_URL: "https://legacy.example.com/gold/v41/",
    });

    expect(url).toBe("https://platform-core.internal/catalog/v1/vector-tiles/manifest");
  });

  it("returns CSP origins for platform-core manifest and optional public tile host", () => {
    const origins = resolveVectorTileAllowedOrigins({
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: "https://platform-core.example.com/api",
      NEXT_PUBLIC_TILES_MANIFEST_URL: "https://static.example.com/gold/manifest.json",
    });

    expect(origins).toEqual(["https://platform-core.example.com", "https://static.example.com"]);
  });

  it("builds the default browser runtime env from direct public env references", () => {
    const previousPlatformCoreBase = process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL;
    const previousManifestUrl = process.env.NEXT_PUBLIC_TILES_MANIFEST_URL;
    process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = "https://platform-core.example.com";
    process.env.NEXT_PUBLIC_TILES_MANIFEST_URL = "";

    try {
      expect(resolveVectorTileManifestUrl(resolveVectorTileRuntimeEnv())).toBe(
        "https://platform-core.example.com/catalog/v1/vector-tiles/manifest",
      );
    } finally {
      process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL = previousPlatformCoreBase;
      process.env.NEXT_PUBLIC_TILES_MANIFEST_URL = previousManifestUrl;
    }
  });

  liveIt("parses the live platform-core Catalog anchor manifest contract", async () => {
    const manifest = await fetchVectorTileManifest(fetch, {
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: livePlatformCoreBaseUrl,
      NEXT_PUBLIC_TILES_MANIFEST_URL: undefined,
    });
    const source = buildVectorTileSource(manifest, PARCEL_ANCHOR_VECTOR_TILE_LAYER);

    expect(manifest.schema_version).toBe(1);
    expect(manifest.current_version).toBeTruthy();
    expect(manifest.artifacts.parcel_anchor?.source_layer).toBe("parcel_anchor");
    expect(source.type).toBe("vector");
    expect(source.tiles[0]).toContain("parcel-marker-anchor");
  });
});
