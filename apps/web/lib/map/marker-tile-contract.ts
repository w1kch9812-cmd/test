import { z } from "zod";

const MARKER_TILE_CONTRACT_PATH = "/map/v1/marker-tiles/contract";
const MARKER_TILE_ENDPOINT_TEMPLATE =
  "/map/v1/marker-tiles/{layer}/{z}/{x}/{y}.pbf?filter_hash={hash}";
const LISTING_MARKER_TILE_ENDPOINT_TEMPLATE =
  "/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash={hash}";
export const PARCEL_ANCHOR_MARKER_TILE_LAYER = "parcel_anchor";
export const LISTING_MARKER_TILE_LAYER = "listing";
export const ALL_ACTIVE_MARKER_FILTER_HASH = "all-active-v1";

type EnvLike = Record<string, string | undefined>;

const MarkerTileContractSchema = z.object({
  response_format: z.literal("mvt_pbf"),
  position_source: z.literal("pnu_anchor"),
  bbox_marker_runtime_forbidden: z.literal(true),
  dropped_marker_success_forbidden: z.literal(true),
  endpoint_template: z.literal(MARKER_TILE_ENDPOINT_TEMPLATE),
  supported_layers: z
    .array(z.string().regex(/^[a-z][a-z0-9_]*$/))
    .refine((layers) => layers.includes(PARCEL_ANCHOR_MARKER_TILE_LAYER), {
      message: `supported_layers must include ${PARCEL_ANCHOR_MARKER_TILE_LAYER}`,
    }),
  default_filter_hash: z.literal(ALL_ACTIVE_MARKER_FILTER_HASH),
});

export type MarkerTileContract = z.infer<typeof MarkerTileContractSchema>;

export type MarkerTileSource = {
  type: "vector";
  tiles: [string];
  minzoom: number;
  maxzoom: number;
};

export type BuildMarkerTileSourceInput = {
  contract: MarkerTileContract;
  platformCoreBaseUrl: string;
  layer: string;
  filterHash: string;
  minzoom: number;
  maxzoom: number;
};

export type BuildListingMarkerTileSourceInput = {
  filterHash: string;
  minzoom: number;
  maxzoom: number;
  origin?: string;
};

export function parseMarkerTileContract(input: unknown): MarkerTileContract {
  return MarkerTileContractSchema.parse(input);
}

export function resolveMarkerTileContractUrl(
  env: EnvLike = resolveMarkerTileRuntimeEnv(),
): string | undefined {
  const platformCoreBase = nonempty(env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL);
  if (!platformCoreBase) return undefined;

  return `${stripTrailingSlash(platformCoreBase)}${MARKER_TILE_CONTRACT_PATH}`;
}

export function resolveMarkerTileAllowedOrigins(
  env: EnvLike = resolveMarkerTileRuntimeEnv(),
): string[] {
  const urls = [
    resolveMarkerTileContractUrl(env),
    nonempty(env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL),
  ];
  const origins = new Set<string>();
  for (const url of urls) {
    if (!url) continue;
    try {
      origins.add(new URL(url).origin);
    } catch {
      // Environment validation reports bad URLs elsewhere; CSP assembly should stay resilient.
    }
  }
  return [...origins].sort();
}

export async function fetchMarkerTileContract(
  fetcher: typeof fetch = fetch,
  env: EnvLike = resolveMarkerTileRuntimeEnv(),
): Promise<MarkerTileContract> {
  const contractUrl = resolveMarkerTileContractUrl(env);
  if (!contractUrl) {
    throw new Error("NEXT_PUBLIC_PLATFORM_CORE_BASE_URL is required for marker tiles");
  }

  const response = await fetcher(contractUrl, {
    headers: { accept: "application/json" },
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`marker tile contract fetch failed: ${response.status}`);
  }

  return parseMarkerTileContract(await response.json());
}

export function buildMarkerTileSource(input: BuildMarkerTileSourceInput): MarkerTileSource {
  assertLayer(input.layer);
  assertFilterHash(input.filterHash);
  assertSupportedLayer(input.contract, input.layer);
  assertSupportedFilterHash(input.contract, input.filterHash);
  assertZoomRange(input.minzoom, input.maxzoom);

  const baseUrl = stripTrailingSlash(input.platformCoreBaseUrl);
  const tilePath = input.contract.endpoint_template
    .replaceAll("{layer}", encodeURIComponent(input.layer))
    .replaceAll("{hash}", encodeURIComponent(input.filterHash));

  return {
    type: "vector",
    tiles: [`${baseUrl}${tilePath}`],
    minzoom: input.minzoom,
    maxzoom: input.maxzoom,
  };
}

export function buildDefaultMarkerTileSource(
  input: Omit<BuildMarkerTileSourceInput, "layer" | "filterHash">,
): MarkerTileSource {
  return buildMarkerTileSource({
    ...input,
    layer: PARCEL_ANCHOR_MARKER_TILE_LAYER,
    filterHash: input.contract.default_filter_hash,
  });
}

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

export function resolveMarkerTileRuntimeEnv(): EnvLike {
  return {
    NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL,
  };
}

function assertLayer(layer: string): void {
  if (/^[a-z][a-z0-9_]*$/.test(layer)) return;
  throw new Error("marker tile layer must be snake_case");
}

function assertFilterHash(filterHash: string): void {
  if (/^[A-Za-z0-9._:-]+$/.test(filterHash)) return;
  throw new Error("marker tile filterHash must be a non-empty stable identifier");
}

function assertSupportedLayer(contract: MarkerTileContract, layer: string): void {
  if (contract.supported_layers.includes(layer)) return;
  throw new Error(`unsupported marker tile layer: ${layer}`);
}

function assertSupportedFilterHash(contract: MarkerTileContract, filterHash: string): void {
  if (filterHash === contract.default_filter_hash) return;
  throw new Error(`unsupported marker tile filterHash: ${filterHash}`);
}

function assertSupportedListingFilterHash(filterHash: string): void {
  if (filterHash === ALL_ACTIVE_MARKER_FILTER_HASH) return;
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

function nonempty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
