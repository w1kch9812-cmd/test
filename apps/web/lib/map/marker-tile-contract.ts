import { API } from "@/lib/routes";

const LISTING_MARKER_TILE_ENDPOINT_TEMPLATE = API.proxy.listingMarkerTileTemplate;
const LISTING_MARKER_DELTA_TILE_ENDPOINT_PREFIX = API.proxy.listingMarkerDeltasPrefix;
const LISTING_MARKER_TOMBSTONE_ENDPOINT_PREFIX = API.proxy.listingMarkerTombstonesPrefix;
export const LISTING_MARKER_TILE_LAYER = "listing";
export const LISTING_MARKER_DELTA_TILE_LAYER = "listing_delta";
export const ALL_ACTIVE_MARKER_FILTER_HASH = "all-active-v1";

export type MarkerTileSource = {
  type: "vector";
  tiles: [string];
  minzoom: number;
  maxzoom: number;
};

export type BuildListingMarkerTileSourceInput = {
  filterHash: string;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export type BuildListingMarkerDeltaTileSourceInput = {
  baseVersion: number | null;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export type BuildListingMarkerTombstoneUrlInput = {
  z: number;
  x: number;
  y: number;
  baseVersion: number | null;
  origin?: string;
};

export type ListingMarkerOverlayState = {
  baseVersion: number | null;
  tombstoneIds: Set<string>;
  deltaSourceId: typeof LISTING_MARKER_DELTA_TILE_LAYER;
};

export type CreateListingMarkerOverlayStateInput = {
  baseVersion: number | null;
  tombstoneIds?: Iterable<string>;
};

export function buildListingMarkerTileSource(
  input: BuildListingMarkerTileSourceInput,
): MarkerTileSource {
  assertFilterHash(input.filterHash);
  assertSupportedListingFilterHash(input.filterHash);
  assertZoomRange(input.minzoom, input.maxzoom);

  return {
    type: "vector",
    tiles: [
      `${resolveSameOrigin(input.origin)}${LISTING_MARKER_TILE_ENDPOINT_TEMPLATE.replaceAll(
        "{hash}",
        encodeURIComponent(input.filterHash),
      )}`,
    ],
    minzoom: input.minzoom,
    maxzoom: input.maxzoom,
  };
}

export function buildListingMarkerDeltaTileSource(
  input: BuildListingMarkerDeltaTileSourceInput,
): MarkerTileSource {
  assertZoomRange(input.minzoom, input.maxzoom);

  return {
    type: "vector",
    tiles: [
      `${resolveSameOrigin(input.origin)}${LISTING_MARKER_DELTA_TILE_ENDPOINT_PREFIX}/{z}/{x}/{y}.pbf${buildBaseVersionQuery(
        input.baseVersion,
      )}`,
    ],
    minzoom: input.minzoom,
    maxzoom: input.maxzoom,
  };
}

export function buildListingMarkerTombstoneUrl(input: BuildListingMarkerTombstoneUrlInput): string {
  assertTileCoordinate(input.z, input.x, input.y);
  return `${resolveSameOrigin(input.origin)}${LISTING_MARKER_TOMBSTONE_ENDPOINT_PREFIX}/${input.z}/${input.x}/${input.y}${buildBaseVersionQuery(
    input.baseVersion,
  )}`;
}

export function createListingMarkerOverlayState(
  input: CreateListingMarkerOverlayStateInput,
): ListingMarkerOverlayState {
  return {
    baseVersion: input.baseVersion,
    tombstoneIds: new Set(input.tombstoneIds ?? []),
    deltaSourceId: LISTING_MARKER_DELTA_TILE_LAYER,
  };
}

function assertFilterHash(filterHash: string): void {
  if (/^[A-Za-z0-9._:-]+$/.test(filterHash)) return;
  throw new Error("marker tile filterHash must be a non-empty stable identifier");
}

function assertSupportedListingFilterHash(filterHash: string): void {
  if (filterHash === ALL_ACTIVE_MARKER_FILTER_HASH) return;
  if (/^lst_filter_v1_[0-9a-f]{64}$/.test(filterHash)) return;
  throw new Error(`unsupported listing marker tile filterHash: ${filterHash}`);
}

function assertZoomRange(minzoom: number, maxzoom: number): void {
  if (
    Number.isInteger(minzoom) &&
    Number.isInteger(maxzoom) &&
    minzoom >= 0 &&
    maxzoom <= 24 &&
    minzoom <= maxzoom
  ) {
    return;
  }

  throw new Error("marker tile zoom range must be 0..24 and minzoom <= maxzoom");
}

function assertTileCoordinate(z: number, x: number, y: number): void {
  if (!Number.isInteger(z) || z < 0 || z > 22) {
    throw new Error("marker tile z coordinate must be 0..22");
  }
  const axisLimit = 2 ** z;
  if (!Number.isInteger(x) || x < 0 || x >= axisLimit) {
    throw new Error("marker tile x coordinate is outside the z axis range");
  }
  if (!Number.isInteger(y) || y < 0 || y >= axisLimit) {
    throw new Error("marker tile y coordinate is outside the z axis range");
  }
}

function buildBaseVersionQuery(baseVersion: number | null): string {
  if (baseVersion === null) return "";
  if (!Number.isInteger(baseVersion) || baseVersion < 0) {
    throw new Error("marker tile baseVersion must be a non-negative integer");
  }
  return `?base_version=${encodeURIComponent(String(baseVersion))}`;
}

function stripTrailingSlash(value: string): string {
  return value.endsWith("/") ? value.slice(0, -1) : value;
}

function resolveSameOrigin(origin: string | undefined): string {
  const value = origin ?? (typeof window === "undefined" ? undefined : window.location.origin);
  if (!value) {
    throw new Error("browser origin is required for listing marker tile URLs");
  }
  return stripTrailingSlash(new URL(value).origin);
}
