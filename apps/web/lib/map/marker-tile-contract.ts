import { API } from "@/lib/routes";

const LISTING_MARKER_TILE_ENDPOINT_TEMPLATE = API.proxy.listingMarkerTileTemplate;
export const LISTING_MARKER_TILE_LAYER = "listing";
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
