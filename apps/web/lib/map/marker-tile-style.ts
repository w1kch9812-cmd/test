import { LISTING_TYPE_COLOR_FALLBACK, MAP_LAYER_COLORS } from "@gongzzang/ui/tokens.js";
import {
  buildDefaultMarkerTileSource,
  buildListingMarkerTileSource,
  LISTING_MARKER_TILE_LAYER,
  type MarkerTileContract,
  type MarkerTileSource,
  PARCEL_ANCHOR_MARKER_TILE_LAYER,
} from "@/lib/map/marker-tile-contract";

export const PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID = PARCEL_ANCHOR_MARKER_TILE_LAYER;
export const PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID = "parcel-anchor-markers-circle";
export const LISTING_MARKER_TILE_SOURCE_ID = LISTING_MARKER_TILE_LAYER;
export const LISTING_MARKER_TILE_CIRCLE_LAYER_ID = "listing-markers-circle";

type ParcelAnchorMarkerLayer = {
  id: typeof PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID;
  type: "circle";
  source: typeof PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID;
  "source-layer": typeof PARCEL_ANCHOR_MARKER_TILE_LAYER;
  minzoom: number;
  maxzoom: number;
  paint: {
    "circle-color": string;
    "circle-opacity": number;
    "circle-radius": unknown[];
    "circle-stroke-color": string;
    "circle-stroke-opacity": number;
    "circle-stroke-width": unknown[];
  };
};

export type ParcelAnchorMarkerLayerRegistration = {
  sourceId: typeof PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ParcelAnchorMarkerLayer];
};

type ListingMarkerLayer = {
  id: typeof LISTING_MARKER_TILE_CIRCLE_LAYER_ID;
  type: "circle";
  source: typeof LISTING_MARKER_TILE_SOURCE_ID;
  "source-layer": typeof LISTING_MARKER_TILE_LAYER;
  minzoom: number;
  maxzoom: number;
  paint: {
    "circle-color": string;
    "circle-opacity": number;
    "circle-radius": unknown[];
    "circle-stroke-color": string;
    "circle-stroke-opacity": number;
    "circle-stroke-width": unknown[];
  };
};

export type ListingMarkerLayerRegistration = {
  sourceId: typeof LISTING_MARKER_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ListingMarkerLayer];
};

export type BuildParcelAnchorMarkerLayerRegistrationInput = {
  contract: MarkerTileContract;
  platformCoreBaseUrl: string;
  minzoom: number;
  maxzoom: number;
};

export type BuildListingMarkerLayerRegistrationInput = {
  filterHash: string;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export function buildParcelAnchorMarkerLayerRegistration(
  input: BuildParcelAnchorMarkerLayerRegistrationInput,
): ParcelAnchorMarkerLayerRegistration {
  return {
    sourceId: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
    source: buildDefaultMarkerTileSource(input),
    layers: [
      {
        id: PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
        type: "circle",
        source: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
        "source-layer": PARCEL_ANCHOR_MARKER_TILE_LAYER,
        minzoom: input.minzoom,
        maxzoom: input.maxzoom,
        paint: {
          "circle-color": MAP_LAYER_COLORS.parcel.fill,
          "circle-opacity": 0.92,
          "circle-radius": ["interpolate", ["linear"], ["zoom"], 8, 3, 14, 5, 18, 7],
          "circle-stroke-color": "#ffffff",
          "circle-stroke-opacity": 0.95,
          "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 8, 0.75, 14, 1, 18, 1.5],
        },
      },
    ],
  };
}

export function buildListingMarkerLayerRegistration(
  input: BuildListingMarkerLayerRegistrationInput,
): ListingMarkerLayerRegistration {
  return {
    sourceId: LISTING_MARKER_TILE_SOURCE_ID,
    source: buildListingMarkerTileSource(input),
    layers: [
      {
        id: LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
        type: "circle",
        source: LISTING_MARKER_TILE_SOURCE_ID,
        "source-layer": LISTING_MARKER_TILE_LAYER,
        minzoom: input.minzoom,
        maxzoom: input.maxzoom,
        paint: {
          "circle-color": LISTING_TYPE_COLOR_FALLBACK,
          "circle-opacity": 0.96,
          "circle-radius": ["interpolate", ["linear"], ["zoom"], 8, 4, 14, 6, 18, 9],
          "circle-stroke-color": "#ffffff",
          "circle-stroke-opacity": 0.96,
          "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 8, 1, 14, 1.5, 18, 2],
        },
      },
    ],
  };
}
