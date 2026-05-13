// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  buildVectorTileSource,
  fetchVectorTileManifest,
  parseVectorTileManifest,
  resolveVectorTileAllowedOrigins,
  resolveVectorTileManifestUrl,
  resolveVectorTileRuntimeEnv,
} from "@/lib/map/vector-tile-manifest";

const manifestFixture = {
  schema_version: 1,
  current_version: "v42",
  previous_version: "v41",
  tiles_url_template: "https://static.example.com/gold/{version}/{layer}/{z}/{x}/{y}.pbf",
  published_at: "2026-05-12T00:00:00Z",
  artifacts: {
    parcels: {
      source_layer: "parcels",
      tile_min_zoom: 8,
      tile_max_zoom: 16,
      render_min_zoom: 10,
      render_max_zoom: 22,
      tilejson_object_key: "gold/v42/parcels.json",
      object_key_prefix: "gold/v42/parcels/",
      flat_tile_count: 10,
      flat_tile_total_bytes: 2048,
      lineage: {
        source_record_id: "018f0000-0000-7000-8000-000000000001",
        manifest_file_asset_id: "018f0000-0000-7000-8000-000000000002",
        tilejson_file_asset_id: "018f0000-0000-7000-8000-000000000003",
        source_file_asset_ids: ["018f0000-0000-7000-8000-000000000004"],
      },
    },
  },
};

describe("platform-core vector tile manifest consumer", () => {
  const livePlatformCoreBaseUrl = process.env.PLATFORM_CORE_MANIFEST_LIVE_BASE_URL;
  const liveIt = livePlatformCoreBaseUrl ? it : it.skip;

  it("requires the core parcels artifact but allows optional layers to be absent", () => {
    const manifest = parseVectorTileManifest(manifestFixture);

    expect(manifest.current_version).toBe("v42");
    expect(manifest.artifacts.parcels?.source_layer).toBe("parcels");
    expect(manifest.artifacts.admin).toBeUndefined();
  });

  it("rejects manifests without the core parcels artifact", () => {
    const withoutParcels = {
      ...manifestFixture,
      artifacts: {},
    };

    expect(() => parseVectorTileManifest(withoutParcels)).toThrow(/parcels/);
  });

  it("builds mapbox vector source from tiles_url_template, not per-layer TileJSON URLs", () => {
    const manifest = parseVectorTileManifest(manifestFixture);
    const source = buildVectorTileSource(manifest, "parcels", { promoteId: "PNU" });

    expect(source).toEqual({
      type: "vector",
      tiles: ["https://static.example.com/gold/v42/parcels/{z}/{x}/{y}.pbf"],
      minzoom: 8,
      maxzoom: 16,
      promoteId: "PNU",
    });
    expect(JSON.stringify(source)).not.toContain("parcels.json");
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

  liveIt("parses the live platform-core Catalog manifest contract", async () => {
    const manifest = await fetchVectorTileManifest(fetch, {
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: livePlatformCoreBaseUrl,
      NEXT_PUBLIC_TILES_MANIFEST_URL: undefined,
    });
    const source = buildVectorTileSource(manifest, "parcels", { promoteId: "PNU" });

    expect(manifest.schema_version).toBe(1);
    expect(manifest.current_version).toBeTruthy();
    expect(manifest.artifacts.parcels?.source_layer).toBe("parcels");
    expect(source.type).toBe("vector");
    expect(source.tiles[0]).toContain(manifest.current_version);
    expect(source.tiles[0]).toContain("parcels");
  });
});
