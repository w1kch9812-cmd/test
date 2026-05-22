// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  ALL_ACTIVE_MARKER_FILTER_HASH,
  buildDefaultMarkerTileSource,
  buildListingMarkerTileSource,
  buildMarkerTileSource,
  fetchMarkerTileContract,
  LISTING_MARKER_TILE_LAYER,
  PARCEL_ANCHOR_MARKER_TILE_LAYER,
  parseMarkerTileContract,
  resolveMarkerTileAllowedOrigins,
  resolveMarkerTileContractUrl,
} from "@/lib/map/marker-tile-contract";

const contractFixture = {
  response_format: "mvt_pbf",
  position_source: "pnu_anchor",
  bbox_marker_runtime_forbidden: true,
  dropped_marker_success_forbidden: true,
  endpoint_template: "/map/v1/marker-tiles/{layer}/{z}/{x}/{y}.pbf?filter_hash={hash}",
  supported_layers: ["parcel_anchor"],
  default_filter_hash: "all-active-v1",
};

describe("platform-core marker tile contract consumer", () => {
  it("accepts only the PNU-anchor backed MVT/PBF marker contract", () => {
    const contract = parseMarkerTileContract(contractFixture);

    expect(contract.response_format).toBe("mvt_pbf");
    expect(contract.position_source).toBe("pnu_anchor");
    expect(contract.bbox_marker_runtime_forbidden).toBe(true);
    expect(contract.dropped_marker_success_forbidden).toBe(true);
    expect(contract.supported_layers).toEqual([PARCEL_ANCHOR_MARKER_TILE_LAYER]);
    expect(contract.default_filter_hash).toBe(ALL_ACTIVE_MARKER_FILTER_HASH);
  });

  it("rejects JSON, product-coordinate, bbox, or missing parcel-anchor defaults", () => {
    expect(() =>
      parseMarkerTileContract({
        ...contractFixture,
        response_format: "json",
      }),
    ).toThrow(/mvt_pbf/);
    expect(() =>
      parseMarkerTileContract({
        ...contractFixture,
        position_source: "listing_geom_point",
      }),
    ).toThrow(/pnu_anchor/);
    expect(() =>
      parseMarkerTileContract({
        ...contractFixture,
        bbox_marker_runtime_forbidden: false,
      }),
    ).toThrow(/bbox/);
    expect(() =>
      parseMarkerTileContract({
        ...contractFixture,
        supported_layers: ["listing"],
      }),
    ).toThrow(/parcel_anchor/);
    expect(() =>
      parseMarkerTileContract({
        ...contractFixture,
        default_filter_hash: "active-industrial-sale-v1",
      }),
    ).toThrow(/all-active-v1/);
  });

  it("resolves marker contract from platform-core base URL only", () => {
    expect(
      resolveMarkerTileContractUrl({
        NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: "https://platform-core.example.com/",
      }),
    ).toBe("https://platform-core.example.com/map/v1/marker-tiles/contract");

    expect(resolveMarkerTileContractUrl({})).toBeUndefined();
  });

  it("returns CSP origins for the platform-core marker contract and tile host", () => {
    const origins = resolveMarkerTileAllowedOrigins({
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: "https://platform-core.example.com/api",
    });

    expect(origins).toEqual(["https://platform-core.example.com"]);
  });

  it("fetches and parses the platform-core marker tile contract", async () => {
    const requestedUrls: string[] = [];
    const fetcher = async (input: RequestInfo | URL, init?: RequestInit) => {
      requestedUrls.push(String(input));
      expect(init?.headers).toEqual({ accept: "application/json" });
      expect(init?.cache).toBe("no-store");
      return Response.json(contractFixture);
    };

    const contract = await fetchMarkerTileContract(fetcher, {
      NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: "https://platform-core.example.com",
    });

    expect(requestedUrls).toEqual([
      "https://platform-core.example.com/map/v1/marker-tiles/contract",
    ]);
    expect(contract.position_source).toBe("pnu_anchor");
  });

  it("builds the default parcel-anchor vector source without bounds or bbox parameters", () => {
    const source = buildDefaultMarkerTileSource({
      contract: parseMarkerTileContract(contractFixture),
      platformCoreBaseUrl: "https://platform-core.example.com/",
      minzoom: 8,
      maxzoom: 18,
    });

    expect(source).toEqual({
      type: "vector",
      tiles: [
        "https://platform-core.example.com/map/v1/marker-tiles/parcel_anchor/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 8,
      maxzoom: 18,
    });
    expect(source.tiles[0]).not.toContain("bounds=");
    expect(source.tiles[0]).not.toContain("bbox=");
    expect(source.tiles[0]).not.toContain("lat=");
    expect(source.tiles[0]).not.toContain("lng=");
  });

  it("builds a validated vector source template from explicit contract values", () => {
    const source = buildMarkerTileSource({
      contract: parseMarkerTileContract(contractFixture),
      platformCoreBaseUrl: "https://platform-core.example.com/",
      layer: PARCEL_ANCHOR_MARKER_TILE_LAYER,
      filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
      minzoom: 8,
      maxzoom: 18,
    });

    expect(source).toEqual({
      type: "vector",
      tiles: [
        "https://platform-core.example.com/map/v1/marker-tiles/parcel_anchor/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 8,
      maxzoom: 18,
    });
    expect(source.tiles[0]).not.toContain("bounds=");
    expect(source.tiles[0]).not.toContain("bbox=");
    expect(source.tiles[0]).not.toContain("lat=");
    expect(source.tiles[0]).not.toContain("lng=");
  });

  it("builds the Gongzzang-owned listing marker vector source through same-origin proxy", () => {
    const source = buildListingMarkerTileSource({
      filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
      minzoom: 8,
      maxzoom: 18,
      origin: "http://localhost:3900",
    });

    expect(source).toEqual({
      type: "vector",
      tiles: [
        "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
      ],
      minzoom: 8,
      maxzoom: 18,
    });
    expect(new URL(source.tiles[0]).origin).toBe("http://localhost:3900");
    expect(LISTING_MARKER_TILE_LAYER).toBe("listing");
    expect(source.tiles[0]).not.toContain("bounds=");
    expect(source.tiles[0]).not.toContain("bbox=");
    expect(source.tiles[0]).not.toContain("lat=");
    expect(source.tiles[0]).not.toContain("lng=");
  });

  it("rejects explicit marker sources outside the platform-core contract", () => {
    const contract = parseMarkerTileContract(contractFixture);

    expect(() =>
      buildMarkerTileSource({
        contract,
        platformCoreBaseUrl: "https://platform-core.example.com/",
        layer: "listing",
        filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
        minzoom: 8,
        maxzoom: 18,
      }),
    ).toThrow(/unsupported marker tile layer/);

    expect(() =>
      buildMarkerTileSource({
        contract,
        platformCoreBaseUrl: "https://platform-core.example.com/",
        layer: PARCEL_ANCHOR_MARKER_TILE_LAYER,
        filterHash: "active-industrial-sale-v1",
        minzoom: 8,
        maxzoom: 18,
      }),
    ).toThrow(/unsupported marker tile filterHash/);
  });
});
