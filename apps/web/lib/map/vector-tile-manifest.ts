import { z } from "zod";

export const CORE_VECTOR_TILE_LAYER = "parcels" as const;
export const OPTIONAL_VECTOR_TILE_LAYERS = ["admin", "complex"] as const;
export const PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER = "parcel_anchor_aggregate" as const;
export const PARCEL_ANCHOR_VECTOR_TILE_LAYER = "parcel_anchor" as const;
export type VectorTileLayerId =
  | typeof CORE_VECTOR_TILE_LAYER
  | (typeof OPTIONAL_VECTOR_TILE_LAYERS)[number]
  | typeof PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER
  | typeof PARCEL_ANCHOR_VECTOR_TILE_LAYER;

type EnvLike = Record<string, string | undefined>;
const manifestUrlSymbol: unique symbol = Symbol("gongzzang.vectorTileManifestUrl");

const uuidSchema = z.string().uuid();

const VectorTileLineageSchema = z.object({
  source_record_id: uuidSchema,
  manifest_file_asset_id: uuidSchema,
  tilejson_file_asset_id: uuidSchema,
  source_file_asset_ids: z.array(uuidSchema),
});

const zoomSchema = z.number().int().min(0).max(24);

const VectorTileArtifactSchema = z
  .object({
    source_layer: z.string().min(1),
    tile_min_zoom: zoomSchema,
    tile_max_zoom: zoomSchema,
    render_min_zoom: zoomSchema,
    render_max_zoom: zoomSchema,
    tilejson_object_key: z.string().min(1),
    object_key_prefix: z.string().min(1),
    flat_tile_count: z.number().int().min(1),
    flat_tile_total_bytes: z.number().int().min(1),
    lineage: VectorTileLineageSchema,
  })
  .superRefine((artifact, ctx) => {
    if (artifact.tile_min_zoom > artifact.tile_max_zoom) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "tile_min_zoom must be <= tile_max_zoom",
        path: ["tile_min_zoom"],
      });
    }
    if (artifact.render_min_zoom > artifact.render_max_zoom) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "render_min_zoom must be <= render_max_zoom",
        path: ["render_min_zoom"],
      });
    }
  });

const tilesTemplateSchema = z
  .string()
  .min(1)
  .refine((value) => ["{object_key_prefix}", "{z}", "{x}", "{y}"].every((p) => value.includes(p)), {
    message: "tiles_url_template must contain {object_key_prefix}, {z}, {x}, and {y}",
  });

const VectorTileManifestSchema = z.object({
  schema_version: z.number().int().min(1),
  current_version: z.string().min(1),
  previous_version: z.string().min(1),
  tiles_url_template: tilesTemplateSchema,
  published_at: z.string().datetime({ offset: true }),
  artifacts: z
    .record(z.string(), VectorTileArtifactSchema)
    .refine((artifacts) => Object.keys(artifacts).length > 0, {
      message: "platform-core vector tile manifest must include at least one artifact",
    }),
});

export type VectorTileManifest = z.infer<typeof VectorTileManifestSchema> & {
  readonly [manifestUrlSymbol]?: string;
};
export type VectorTileArtifact = z.infer<typeof VectorTileArtifactSchema>;

export type VectorTileSource = {
  type: "vector";
  tiles: [string];
  minzoom: number;
  maxzoom: number;
  promoteId?: string;
};

export function parseVectorTileManifest(input: unknown): VectorTileManifest {
  return VectorTileManifestSchema.parse(input);
}

export function resolveVectorTileRuntimeEnv(): EnvLike {
  return {
    NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL,
    NEXT_PUBLIC_TILES_MANIFEST_URL: process.env.NEXT_PUBLIC_TILES_MANIFEST_URL,
  };
}

export function resolveVectorTileManifestUrl(
  env: EnvLike = resolveVectorTileRuntimeEnv(),
): string | undefined {
  const explicit = nonempty(env.NEXT_PUBLIC_TILES_MANIFEST_URL);
  if (explicit) return explicit;

  const platformCoreBase = nonempty(env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL);
  if (platformCoreBase) {
    const base = platformCoreBase.endsWith("/") ? platformCoreBase.slice(0, -1) : platformCoreBase;
    return `${base}/catalog/v1/vector-tiles/manifest`;
  }

  return undefined;
}

export function resolveVectorTileAllowedOrigins(
  env: EnvLike = resolveVectorTileRuntimeEnv(),
): string[] {
  const urls = [
    resolveVectorTileManifestUrl(env),
    nonempty(env.NEXT_PUBLIC_TILES_MANIFEST_URL),
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

export async function fetchVectorTileManifest(
  fetcher: typeof fetch = fetch,
  env: EnvLike = resolveVectorTileRuntimeEnv(),
): Promise<VectorTileManifest> {
  const manifestUrl = resolveVectorTileManifestUrl(env);
  if (!manifestUrl) {
    throw new Error(
      "NEXT_PUBLIC_TILES_MANIFEST_URL or NEXT_PUBLIC_PLATFORM_CORE_BASE_URL is required for vector tiles",
    );
  }
  const response = await fetcher(manifestUrl, {
    headers: { accept: "application/json" },
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`vector tile manifest fetch failed: ${response.status}`);
  }
  return attachManifestUrl(parseVectorTileManifest(await response.json()), manifestUrl);
}

export function buildVectorTileSource(
  manifest: VectorTileManifest,
  layer: VectorTileLayerId,
  options: { promoteId?: string; tileUrlBaseUrl?: string } = {},
): VectorTileSource {
  const artifact = manifest.artifacts[layer];
  if (!artifact) {
    throw new Error(`vector tile artifact is missing: ${layer}`);
  }
  const source: VectorTileSource = {
    type: "vector",
    tiles: [
      materializeTilesUrl(manifest.tiles_url_template, artifact, {
        manifestUrl: manifest[manifestUrlSymbol],
        tileUrlBaseUrl: options.tileUrlBaseUrl,
      }),
    ],
    minzoom: artifact.tile_min_zoom,
    maxzoom: artifact.tile_max_zoom,
  };
  if (options.promoteId) source.promoteId = options.promoteId;
  return source;
}

export function getVectorTileArtifact(
  manifest: VectorTileManifest,
  layer: VectorTileLayerId,
): VectorTileArtifact | undefined {
  return manifest.artifacts[layer];
}

function attachManifestUrl(manifest: VectorTileManifest, manifestUrl: string): VectorTileManifest {
  return Object.defineProperty(manifest, manifestUrlSymbol, {
    value: manifestUrl,
    enumerable: false,
  });
}

function materializeTilesUrl(
  template: string,
  artifact: VectorTileArtifact,
  runtime: { manifestUrl?: string; tileUrlBaseUrl?: string },
): string {
  const materialized = template.replaceAll("{object_key_prefix}", artifact.object_key_prefix);
  if (isAbsoluteHttpUrl(materialized)) return materialized;

  const origin = resolveTileUrlOrigin(runtime);
  if (materialized.startsWith("/")) return `${origin}${materialized}`;
  return `${origin}/${materialized}`;
}

function resolveTileUrlOrigin(runtime: { manifestUrl?: string; tileUrlBaseUrl?: string }): string {
  const baseUrl = runtime.tileUrlBaseUrl ?? runtime.manifestUrl;
  if (!baseUrl) {
    throw new Error("relative tiles_url_template requires a manifest URL or tileUrlBaseUrl");
  }
  return new URL(baseUrl).origin;
}

function isAbsoluteHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

function nonempty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
