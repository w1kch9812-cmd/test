import { z } from "zod";

export const CORE_VECTOR_TILE_LAYER = "parcels" as const;
export const OPTIONAL_VECTOR_TILE_LAYERS = ["admin", "complex"] as const;
export type VectorTileLayerId =
  | typeof CORE_VECTOR_TILE_LAYER
  | (typeof OPTIONAL_VECTOR_TILE_LAYERS)[number];

type EnvLike = Record<string, string | undefined>;

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
  .url()
  .refine(
    (value) => ["{version}", "{layer}", "{z}", "{x}", "{y}"].every((p) => value.includes(p)),
    {
      message: "tiles_url_template must contain {version}, {layer}, {z}, {x}, and {y}",
    },
  );

const VectorTileManifestSchema = z
  .object({
    schema_version: z.number().int().min(1),
    current_version: z.string().min(1),
    previous_version: z.string().min(1),
    tiles_url_template: tilesTemplateSchema,
    published_at: z.string().datetime({ offset: true }),
    artifacts: z.record(z.string(), VectorTileArtifactSchema),
  })
  .superRefine((manifest, ctx) => {
    if (!manifest.artifacts[CORE_VECTOR_TILE_LAYER]) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "platform-core vector tile manifest must include parcels artifact",
        path: ["artifacts", CORE_VECTOR_TILE_LAYER],
      });
    }
  });

export type VectorTileManifest = z.infer<typeof VectorTileManifestSchema>;
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
  return parseVectorTileManifest(await response.json());
}

export function buildVectorTileSource(
  manifest: VectorTileManifest,
  layer: VectorTileLayerId,
  options: { promoteId?: string } = {},
): VectorTileSource {
  const artifact = manifest.artifacts[layer];
  if (!artifact) {
    throw new Error(`vector tile artifact is missing: ${layer}`);
  }
  const source: VectorTileSource = {
    type: "vector",
    tiles: [materializeTilesUrl(manifest.tiles_url_template, manifest.current_version, layer)],
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

function materializeTilesUrl(template: string, version: string, layer: string): string {
  return template.replaceAll("{version}", version).replaceAll("{layer}", layer);
}

function nonempty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
