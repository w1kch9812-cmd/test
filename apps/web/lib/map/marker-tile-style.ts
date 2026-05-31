import { LISTING_TYPE_COLOR_FALLBACK, MAP_LAYER_COLORS } from "@gongzzang/ui/tokens.js";
import {
  buildListingMarkerDeltaTileSource,
  buildListingMarkerTileSource,
  LISTING_MARKER_DELTA_TILE_LAYER,
  LISTING_MARKER_TILE_LAYER,
  type MarkerTileSource,
} from "@/lib/map/marker-tile-contract";
import {
  buildVectorTileSource,
  getVectorTileArtifact,
  PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER,
  PARCEL_ANCHOR_VECTOR_TILE_LAYER,
  type VectorTileManifest,
} from "@/lib/map/vector-tile-manifest";

export const PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID =
  PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER;
export const PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID = PARCEL_ANCHOR_VECTOR_TILE_LAYER;
export const PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_CIRCLE_LAYER_ID =
  "parcel-anchor-aggregate-markers-circle";
export const PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID = "parcel-anchor-markers-circle";
export const LISTING_MARKER_TILE_SOURCE_ID = LISTING_MARKER_TILE_LAYER;
export const LISTING_MARKER_TILE_CIRCLE_LAYER_ID = "listing-markers-circle";
export const LISTING_MARKER_DELTA_TILE_SOURCE_ID = LISTING_MARKER_DELTA_TILE_LAYER;
export const LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID = "listing-marker-deltas-circle";

type MarkerCircleLayer<
  LayerId extends string,
  SourceId extends string,
  SourceLayer extends string,
> = {
  id: LayerId;
  type: "circle";
  source: SourceId;
  "source-layer": SourceLayer;
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

type ParcelAnchorAggregateMarkerLayer = MarkerCircleLayer<
  typeof PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_CIRCLE_LAYER_ID,
  typeof PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
  string
>;

type ParcelAnchorMarkerLayer = MarkerCircleLayer<
  typeof PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
  typeof PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
  string
>;

export type ParcelAnchorAggregateMarkerLayerRegistration = {
  sourceId: typeof PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ParcelAnchorAggregateMarkerLayer];
};

export type ParcelAnchorMarkerLayerRegistration = {
  sourceId: typeof PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ParcelAnchorMarkerLayer];
};

type ListingMarkerLayer = MarkerCircleLayer<
  typeof LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
  typeof LISTING_MARKER_TILE_SOURCE_ID,
  typeof LISTING_MARKER_TILE_LAYER
>;

type ListingMarkerDeltaLayer = MarkerCircleLayer<
  typeof LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID,
  typeof LISTING_MARKER_DELTA_TILE_SOURCE_ID,
  typeof LISTING_MARKER_DELTA_TILE_LAYER
>;

export type ListingMarkerLayerRegistration = {
  sourceId: typeof LISTING_MARKER_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ListingMarkerLayer];
};

export type ListingMarkerDeltaLayerRegistration = {
  sourceId: typeof LISTING_MARKER_DELTA_TILE_SOURCE_ID;
  source: MarkerTileSource;
  layers: [ListingMarkerDeltaLayer];
};

export type BuildParcelAnchorMarkerLayerRegistrationInput = {
  manifest: VectorTileManifest;
};

export type BuildListingMarkerLayerRegistrationInput = {
  filterHash: string;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export type BuildListingMarkerDeltaLayerRegistrationInput = {
  baseVersion: number | null;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export function buildParcelAnchorMarkerLayerRegistration(
  input: BuildParcelAnchorMarkerLayerRegistrationInput,
): ParcelAnchorMarkerLayerRegistration {
  return buildParcelAnchorMarkerLayerRegistrations(input)[1];
}

export function buildParcelAnchorMarkerLayerRegistrations(
  input: BuildParcelAnchorMarkerLayerRegistrationInput,
): [ParcelAnchorAggregateMarkerLayerRegistration, ParcelAnchorMarkerLayerRegistration] {
  const aggregateArtifact = getVectorTileArtifact(
    input.manifest,
    PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER,
  );
  const exactArtifact = getVectorTileArtifact(input.manifest, PARCEL_ANCHOR_VECTOR_TILE_LAYER);
  if (!aggregateArtifact) {
    throw new Error(`platform-core manifest missing ${PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER}`);
  }
  if (!exactArtifact) {
    throw new Error(`platform-core manifest missing ${PARCEL_ANCHOR_VECTOR_TILE_LAYER}`);
  }

  return [
    {
      sourceId: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
      source: buildVectorTileSource(input.manifest, PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER),
      layers: [
        {
          id: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_CIRCLE_LAYER_ID,
          type: "circle",
          source: PARCEL_ANCHOR_AGGREGATE_MARKER_TILE_SOURCE_ID,
          "source-layer": aggregateArtifact.source_layer,
          minzoom: aggregateArtifact.render_min_zoom,
          maxzoom: aggregateArtifact.render_max_zoom,
          paint: {
            "circle-color": MAP_LAYER_COLORS.parcel.fill,
            "circle-opacity": 0.42,
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["coalesce", ["get", "count"], 1],
              1,
              2.5,
              1000,
              6,
              50000,
              12,
            ],
            "circle-stroke-color": "#ffffff",
            "circle-stroke-opacity": 0.72,
            "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 0, 0.5, 8, 0.85, 11, 1],
          },
        },
      ],
    },
    {
      sourceId: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
      source: buildVectorTileSource(input.manifest, PARCEL_ANCHOR_VECTOR_TILE_LAYER),
      layers: [
        {
          id: PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
          type: "circle",
          source: PARCEL_ANCHOR_MARKER_TILE_SOURCE_ID,
          "source-layer": exactArtifact.source_layer,
          minzoom: exactArtifact.render_min_zoom,
          maxzoom: exactArtifact.render_max_zoom,
          paint: {
            "circle-color": MAP_LAYER_COLORS.parcel.fill,
            "circle-opacity": 0.92,
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 12, 3, 14, 5, 18, 7],
            "circle-stroke-color": "#ffffff",
            "circle-stroke-opacity": 0.95,
            "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 12, 0.75, 14, 1, 18, 1.5],
          },
        },
      ],
    },
  ];
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

export function buildListingMarkerDeltaLayerRegistration(
  input: BuildListingMarkerDeltaLayerRegistrationInput,
): ListingMarkerDeltaLayerRegistration {
  return {
    sourceId: LISTING_MARKER_DELTA_TILE_SOURCE_ID,
    source: buildListingMarkerDeltaTileSource(input),
    layers: [
      {
        id: LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID,
        type: "circle",
        source: LISTING_MARKER_DELTA_TILE_SOURCE_ID,
        "source-layer": LISTING_MARKER_DELTA_TILE_LAYER,
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
