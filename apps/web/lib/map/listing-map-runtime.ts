import { MAP_LAYER_COLORS } from "@gongzzang/ui/tokens.js";
import {
  LISTING_MARKER_RENDER_MAX_ZOOM,
  LISTING_MARKER_RENDER_MIN_ZOOM,
} from "@/lib/map/map-zoom-policy";
import { ALL_ACTIVE_MARKER_FILTER_HASH } from "@/lib/map/marker-tile-contract";
import {
  buildListingMarkerDeltaLayerRegistration,
  buildListingMarkerLayerRegistration,
  buildParcelAnchorMarkerLayerRegistrations,
  LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID,
  LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
  PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
} from "@/lib/map/marker-tile-style";
import {
  buildVectorTileSource,
  fetchVectorTileManifest,
  getVectorTileArtifact,
  type VectorTileManifest,
} from "@/lib/map/vector-tile-manifest";

export type MapboxGLLike = {
  addSource?: (id: string, src: Record<string, unknown>) => void;
  addLayer?: (layer: Record<string, unknown>, beforeId?: string) => void;
  getSource?: (id: string) => unknown;
  getLayer?: (id: string) => unknown;
  getCanvas?: () => HTMLCanvasElement | null;
  on?: (event: string, layer: string, handler: (e: unknown) => void) => void;
  isStyleLoaded?: () => boolean;
  once?: (event: string, handler: () => void) => void;
  setFilter?: (layerId: string, filter: unknown[]) => void;
};

const MAPBOX_POLL_INTERVAL_MS = 100;
const MAPBOX_POLL_TIMEOUT_MS = 6_000;
const MAPBOX_MAX_ATTEMPTS = MAPBOX_POLL_TIMEOUT_MS / MAPBOX_POLL_INTERVAL_MS;

export async function setupMapboxRuntime(
  map: naver.maps.Map,
  cleanups: Array<() => void>,
  isCancelled: () => boolean,
  onParcelClick: (pnu: string) => void,
  onListingClick: (listingId: string) => void,
  onMapboxReady: (mb: MapboxGLLike) => void,
): Promise<void> {
  const mb = await waitForMapbox(map);
  if (isCancelled()) return;
  if (process.env.NODE_ENV !== "production") {
    (window as unknown as Record<string, unknown>).__listingMb = mb;
  }

  await waitForMapboxStyle(mb, isCancelled);
  if (isCancelled()) return;

  const manifest = await loadPlatformCoreVectorTileManifest();
  await setupPolygonLayers(mb, onParcelClick, manifest);
  await setupMarkerTileLayers(mb, onParcelClick, manifest);
  await setupListingMarkerTileLayers(mb, onListingClick);
  if (isCancelled()) return;
  onMapboxReady(mb);
  const recovery = setupWebGlRecovery(mb);
  if (recovery) cleanups.push(recovery);
}

async function waitForMapbox(naverMap: naver.maps.Map): Promise<MapboxGLLike> {
  for (let i = 0; i < MAPBOX_MAX_ATTEMPTS; i++) {
    const mb = (naverMap as unknown as { _mapbox?: MapboxGLLike })._mapbox;
    if (mb && typeof mb.addSource === "function") return mb;
    await new Promise((r) => setTimeout(r, MAPBOX_POLL_INTERVAL_MS));
  }
  throw new Error(`mapbox-gl instance polling timeout (${MAPBOX_POLL_TIMEOUT_MS / 1000}s)`);
}

async function waitForMapboxStyle(mb: MapboxGLLike, isCancelled: () => boolean): Promise<void> {
  for (let i = 0; i < MAPBOX_MAX_ATTEMPTS && !mb.isStyleLoaded?.(); i++) {
    await new Promise((r) => setTimeout(r, MAPBOX_POLL_INTERVAL_MS));
    if (isCancelled()) return;
  }
}

async function setupPolygonLayers(
  mb: MapboxGLLike,
  onParcelClick: (pnu: string) => void,
  manifest: VectorTileManifest | null,
): Promise<void> {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer unavailable; polygon layer setup skipped");
    return;
  }
  if (!manifest) return;

  try {
    const artifact = getVectorTileArtifact(manifest, "parcels");
    if (artifact && !mb.getSource?.("parcels")) {
      mb.addSource("parcels", buildVectorTileSource(manifest, "parcels", { promoteId: "PNU" }));
      mb.addLayer({
        id: "parcels-fill",
        type: "fill",
        source: "parcels",
        "source-layer": artifact.source_layer,
        minzoom: artifact.render_min_zoom,
        maxzoom: artifact.render_max_zoom,
        paint: {
          "fill-color": MAP_LAYER_COLORS.parcel.fill,
          "fill-opacity": 0.1,
          "fill-outline-color": MAP_LAYER_COLORS.parcel.outline,
        },
      });
      if (typeof mb.on === "function") {
        mb.on("click", "parcels-fill", (e: unknown) => {
          const evt = e as { features?: Array<{ properties?: { PNU?: string; pnu?: string } }> };
          const props = evt.features?.[0]?.properties;
          const pnu = props?.PNU ?? props?.pnu;
          if (typeof pnu === "string" && pnu.length > 0) {
            onParcelClick(pnu);
          }
        });
      }
    }
  } catch (err) {
    logMapLayerFailure("parcels-fill", err, { kind: "core", source: "parcels" });
  }

  try {
    const artifact = getVectorTileArtifact(manifest, "admin");
    if (artifact && !mb.getSource?.("admin")) {
      mb.addSource("admin", buildVectorTileSource(manifest, "admin"));
      mb.addLayer({
        id: "admin-fill",
        type: "fill",
        source: "admin",
        "source-layer": artifact.source_layer,
        minzoom: artifact.render_min_zoom,
        maxzoom: artifact.render_max_zoom,
        paint: {
          "fill-color": MAP_LAYER_COLORS.admin.fill,
          "fill-opacity": 0.05,
          "fill-outline-color": MAP_LAYER_COLORS.admin.outline,
        },
      });
    }
  } catch (err) {
    logMapLayerFailure("admin-fill", err, { kind: "optional", source: "admin" });
  }

  try {
    const artifact = getVectorTileArtifact(manifest, "complex");
    if (artifact && !mb.getSource?.("complex")) {
      mb.addSource("complex", buildVectorTileSource(manifest, "complex"));
      mb.addLayer({
        id: "complex-fill",
        type: "fill",
        source: "complex",
        "source-layer": artifact.source_layer,
        minzoom: artifact.render_min_zoom,
        maxzoom: artifact.render_max_zoom,
        paint: {
          "fill-color": MAP_LAYER_COLORS.complex.fill,
          "fill-opacity": 0.15,
          "fill-outline-color": MAP_LAYER_COLORS.complex.outline,
        },
      });
    }
  } catch (err) {
    logMapLayerFailure("complex-fill", err, { kind: "optional", source: "complex" });
  }
}

async function setupMarkerTileLayers(
  mb: MapboxGLLike,
  onParcelClick: (pnu: string) => void,
  manifest: VectorTileManifest | null,
): Promise<void> {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer unavailable; marker tile setup skipped");
    return;
  }
  if (!manifest) return;

  try {
    const registrations = buildParcelAnchorMarkerLayerRegistrations({ manifest });

    for (const registration of registrations) {
      if (!mb.getSource?.(registration.sourceId)) {
        mb.addSource(registration.sourceId, registration.source);
      }
      for (const layer of registration.layers) {
        if (!mb.getLayer?.(layer.id)) {
          mb.addLayer(layer);
        }
      }
    }
    if (typeof mb.on === "function") {
      mb.on("click", PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID, (e: unknown) => {
        const evt = e as {
          features?: Array<{ properties?: { pnu?: string; detail_ref?: string } }>;
        };
        const props = evt.features?.[0]?.properties;
        const pnu = props?.pnu ?? props?.detail_ref;
        if (typeof pnu === "string" && pnu.length > 0) {
          onParcelClick(pnu);
        }
      });
    }
  } catch (err) {
    logMapLayerFailure("parcel-anchor-markers", err, {
      kind: "core",
      source: "platform-core-vector-tile-manifest",
    });
  }
}

async function setupListingMarkerTileLayers(
  mb: MapboxGLLike,
  onListingClick: (listingId: string) => void,
): Promise<void> {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn(
      "[ListingMap] mapbox addSource/addLayer unavailable; listing marker setup skipped",
    );
    return;
  }

  try {
    const registration = buildListingMarkerLayerRegistration({
      filterHash: ALL_ACTIVE_MARKER_FILTER_HASH,
      minzoom: LISTING_MARKER_RENDER_MIN_ZOOM,
      maxzoom: LISTING_MARKER_RENDER_MAX_ZOOM,
    });
    const deltaRegistration = buildListingMarkerDeltaLayerRegistration({
      baseVersion: null,
      minzoom: 0,
      maxzoom: LISTING_MARKER_RENDER_MAX_ZOOM,
    });

    for (const markerRegistration of [registration, deltaRegistration]) {
      if (!mb.getSource?.(markerRegistration.sourceId)) {
        mb.addSource(markerRegistration.sourceId, markerRegistration.source);
      }
      for (const layer of markerRegistration.layers) {
        if (!mb.getLayer?.(layer.id)) {
          mb.addLayer(layer);
        }
      }
    }
    if (typeof mb.on === "function") {
      const onListingMarkerClick = (e: unknown) => {
        const evt = e as {
          features?: Array<{ properties?: { id?: string; detail_ref?: string } }>;
        };
        const props = evt.features?.[0]?.properties;
        const listingId = props?.detail_ref ?? props?.id;
        if (typeof listingId === "string" && listingId.length > 0) {
          onListingClick(listingId);
        }
      };
      mb.on("click", LISTING_MARKER_TILE_CIRCLE_LAYER_ID, onListingMarkerClick);
      mb.on("click", LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID, onListingMarkerClick);
    }
  } catch (err) {
    logMapLayerFailure("listing-markers", err, {
      kind: "core",
      source: "listing",
    });
  }
}

function setupWebGlRecovery(mb: MapboxGLLike): (() => void) | undefined {
  if (typeof mb.getCanvas !== "function") return undefined;
  const glCanvas = mb.getCanvas();
  if (!glCanvas) return undefined;
  const onLost = (e: Event) => {
    e.preventDefault();
    console.warn("[ListingMap] WebGL context lost");
  };
  const onRestored = () => {
    try {
      const mbExt = mb as MapboxGLLike & { resize?: () => void; triggerRepaint?: () => void };
      mbExt.resize?.();
      mbExt.triggerRepaint?.();
    } catch {
      /* ignore */
    }
  };
  glCanvas.addEventListener("webglcontextlost", onLost);
  glCanvas.addEventListener("webglcontextrestored", onRestored);
  return () => {
    glCanvas.removeEventListener("webglcontextlost", onLost);
    glCanvas.removeEventListener("webglcontextrestored", onRestored);
  };
}

async function loadPlatformCoreVectorTileManifest(): Promise<VectorTileManifest | null> {
  return fetchVectorTileManifest().catch((err: unknown) => {
    logMapLayerFailure("vector-tile-manifest", err, {
      kind: "core",
      source: "platform-core",
    });
    return null;
  });
}

function logMapLayerFailure(
  layerId: string,
  err: unknown,
  ctx: { kind: "core" | "optional"; source: string },
): void {
  const message = err instanceof Error ? err.message : String(err);
  const payload = { event: "map_layer_register_failed", layerId, ...ctx, message };
  if (ctx.kind === "core") {
    console.error("[ListingMap]", JSON.stringify(payload));
  } else {
    console.info("[ListingMap]", JSON.stringify(payload));
  }
}
