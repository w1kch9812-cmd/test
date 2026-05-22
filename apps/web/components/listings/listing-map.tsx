"use client";
import { MAP_LAYER_COLORS } from "@gongzzang/ui/tokens.js";
import { useTranslations } from "next-intl";
import { useEffect, useRef } from "react";
import {
  ALL_ACTIVE_MARKER_FILTER_HASH,
  fetchMarkerTileContract,
  resolveMarkerTileRuntimeEnv,
} from "@/lib/map/marker-tile-contract";
import {
  buildListingMarkerLayerRegistration,
  buildParcelAnchorMarkerLayerRegistration,
  LISTING_MARKER_TILE_CIRCLE_LAYER_ID,
  PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID,
} from "@/lib/map/marker-tile-style";
import {
  buildVectorTileSource,
  fetchVectorTileManifest,
  getVectorTileArtifact,
} from "@/lib/map/vector-tile-manifest";
import { loadNaverMaps } from "@/lib/naver-maps";
import { usePanelStack } from "@/lib/panel/use-panel-stack";

async function setupMarkerTileLayers(
  mb: MapboxGLLike,
  onParcelClick: (pnu: string) => void,
): Promise<void> {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer unavailable; marker tile setup skipped");
    return;
  }

  const contract = await fetchMarkerTileContract().catch((err: unknown) => {
    logMapLayerFailure("parcel-anchor-markers", err, {
      kind: "core",
      source: "platform-core-marker-contract",
    });
    return null;
  });
  if (!contract) {
    return;
  }

  const platformCoreBaseUrl = resolveMarkerTileRuntimeEnv().NEXT_PUBLIC_PLATFORM_CORE_BASE_URL;
  if (!platformCoreBaseUrl) {
    logMapLayerFailure(
      "parcel-anchor-markers",
      new Error("NEXT_PUBLIC_PLATFORM_CORE_BASE_URL is required for marker tiles"),
      { kind: "core", source: "platform-core-marker-contract" },
    );
    return;
  }

  try {
    const registration = buildParcelAnchorMarkerLayerRegistration({
      contract,
      platformCoreBaseUrl,
      minzoom: 8,
      maxzoom: 18,
    });

    if (!mb.getSource?.(registration.sourceId)) {
      mb.addSource(registration.sourceId, registration.source);
    }
    for (const layer of registration.layers) {
      if (!mb.getLayer?.(layer.id)) {
        mb.addLayer(layer);
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
      source: "parcel_anchor",
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
      minzoom: 8,
      maxzoom: 18,
    });

    if (!mb.getSource?.(registration.sourceId)) {
      mb.addSource(registration.sourceId, registration.source);
    }
    for (const layer of registration.layers) {
      if (!mb.getLayer?.(layer.id)) {
        mb.addLayer(layer);
      }
    }
    if (typeof mb.on === "function") {
      mb.on("click", LISTING_MARKER_TILE_CIRCLE_LAYER_ID, (e: unknown) => {
        const evt = e as {
          features?: Array<{ properties?: { id?: string; detail_ref?: string } }>;
        };
        const props = evt.features?.[0]?.properties;
        const listingId = props?.detail_ref ?? props?.id;
        if (typeof listingId === "string" && listingId.length > 0) {
          onListingClick(listingId);
        }
      });
    }
  } catch (err) {
    logMapLayerFailure("listing-markers", err, {
      kind: "core",
      source: "listing",
    });
  }
}

/**
 * Naver Maps GL keeps the underlying mapbox-gl instance on a private `_mapbox` field.
 * There is no public SDK accessor, so ADR 0019 records this as a bounded runtime tradeoff.
 */
type MapboxGLLike = {
  addSource?: (id: string, src: Record<string, unknown>) => void;
  addLayer?: (layer: Record<string, unknown>, beforeId?: string) => void;
  getSource?: (id: string) => unknown;
  getLayer?: (id: string) => unknown;
  getCanvas?: () => HTMLCanvasElement | null;
  on?: (event: string, layer: string, handler: (e: unknown) => void) => void;
  isStyleLoaded?: () => boolean;
  once?: (event: string, handler: () => void) => void;
};

// Polling constants for the private mapbox-gl bridge.
const MAPBOX_POLL_INTERVAL_MS = 100;
const MAPBOX_POLL_TIMEOUT_MS = 6_000;
const MAPBOX_MAX_ATTEMPTS = MAPBOX_POLL_TIMEOUT_MS / MAPBOX_POLL_INTERVAL_MS;

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

async function setupMapboxRuntime(
  map: naver.maps.Map,
  cleanups: Array<() => void>,
  isCancelled: () => boolean,
  onParcelClick: (pnu: string) => void,
  onListingClick: (listingId: string) => void,
): Promise<void> {
  const mb = await waitForMapbox(map);
  if (isCancelled()) return;
  if (process.env.NODE_ENV !== "production") {
    (window as unknown as Record<string, unknown>).__listingMb = mb;
  }

  // addSource/addLayer is reliable only after the mapbox style is loaded. Polling also covers
  // Naver fork edge cases where a one-shot style.load listener can miss an already-fired event.
  await waitForMapboxStyle(mb, isCancelled);
  if (isCancelled()) return;

  await setupPolygonLayers(mb, onParcelClick);
  await setupMarkerTileLayers(mb, onParcelClick);
  await setupListingMarkerTileLayers(mb, onListingClick);
  const recovery = setupWebGlRecovery(mb);
  if (recovery) cleanups.push(recovery);
}

/**
 * Register WebGL context lost/restored handlers for mobile backgrounding and GPU memory pressure.
 */
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

/**
 * Register vector tile sources and layers.
 *
 * ADR 0036 / platform-core ADR 0004:
 * Gongzzang is a static vector tile manifest consumer. Platform Core owns active versions,
 * source layers, render zooms, lineage, and file asset links.
 *
 * env:
 * - `NEXT_PUBLIC_TILES_MANIFEST_URL`: public R2/CDN manifest pointer.
 * - `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL`: Catalog API manifest endpoint base.
 */
async function setupPolygonLayers(
  mb: MapboxGLLike,
  onParcelClick: (pnu: string) => void,
): Promise<void> {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer unavailable; polygon layer setup skipped");
    return;
  }

  const manifest = await fetchVectorTileManifest().catch((err: unknown) => {
    logMapLayerFailure("vector-tile-manifest", err, {
      kind: "core",
      source: "platform-core",
    });
    return null;
  });
  if (!manifest) {
    return;
  }

  // Parcels are a core layer from the Platform Core manifest.
  try {
    if (!mb.getSource?.("parcels")) {
      const artifact = getVectorTileArtifact(manifest, "parcels");
      if (!artifact) {
        throw new Error("platform-core manifest missing parcels artifact");
      }
      // Promote PNU to the mapbox-gl feature id for feature-state operations.
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

  // Admin boundaries are optional and registered only when present in the manifest.
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

  // Industrial complex boundaries are optional and registered only when present.
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

/**
 * Emit structured logs when map layer registration fails.
 *
 * Core layer failures are user-visible and logged as errors. Optional layer failures are degraded
 * states and logged at info level. The message shape is stable for log aggregation.
 *
 * The private Naver SDK `_mapbox` bridge and style-load timing are known risk points from
 * ADR 0019, so every failure path must stay observable.
 */
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

export function ListingMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const { push: pushPanel } = usePanelStack();

  // Initialize Naver GL runtime plus Platform Core polygon and marker PBF layers.
  useEffect(() => {
    let cancelled = false;
    const cleanups: Array<() => void> = [];

    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      // gl: true lets Naver Maps render through its mapbox-gl backed WebGL path.
      // The underlying mapbox instance is accessed through the bounded `_mapbox` bridge above.
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978), // Seoul City Hall
        zoom: 8,
        minZoom: 7,
        maxZoom: 21,
        gl: true,
        zoomControl: false,
        mapTypeControl: false,
        disableKineticPan: false,
      } as naver.maps.MapOptions);
      mapRef.current = map;

      // Dev-only handles for debugging and Playwright probes; omitted from production builds.
      if (process.env.NODE_ENV !== "production") {
        (window as unknown as Record<string, unknown>).__listingMap = map;
      }

      // ADR 0021: flat .pbf vector tiles use the standard mapbox-gl vector source path.
      // No addSourceType, Service Worker, or Blob URL transport is used here.
      setupMapboxRuntime(
        map,
        cleanups,
        () => cancelled,
        (pnu) => pushPanel({ kind: "parcel", id: pnu, view: "summary" }),
        (listingId) => pushPanel({ kind: "listing", id: listingId, view: "summary" }),
      ).catch((e: unknown) => {
        if (!cancelled) {
          console.warn(
            "[ListingMap] mapbox-gl bridge unavailable; vector layers disabled",
            e instanceof Error ? e.message : String(e),
          );
        }
      });
    });
    return () => {
      cancelled = true;
      for (const fn of cleanups) fn();
    };
  }, [pushPanel]);

  return (
    <div className="relative h-full w-full">
      <div ref={containerRef} className="h-full w-full" />
      <MapAttribution />
    </div>
  );
}

/**
 * Public-data attribution for parcel base layers.
 *
 * Source: V-World DTMK dsId=30563. ADR 0027 and the base-layer runbook require this
 * attribution while the runtime still renders parcel tiles from that lineage.
 *
 * This stays separate from mapbox-gl's attribution control to avoid collisions with Naver SDK
 * attribution.
 */
function MapAttribution() {
  // User-facing strings must come from typed i18n.
  const t = useTranslations("map.attribution");
  return (
    <aside
      className="pointer-events-auto absolute bottom-1 right-1 z-10 rounded bg-white/85 px-2 py-0.5 text-[10px] leading-tight text-gray-600 shadow-sm backdrop-blur-sm dark:bg-black/70 dark:text-gray-300"
      aria-label={t("ariaDataSource")}
    >
      <span>{t("parcelSourceLabel")}: </span>
      <a
        href="https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"
        target="_blank"
        rel="noopener noreferrer"
        className="underline hover:text-gray-900 dark:hover:text-white"
      >
        {t("vWorldLink")}
      </a>
      <span className="ml-1">{t("license")}</span>
    </aside>
  );
}
