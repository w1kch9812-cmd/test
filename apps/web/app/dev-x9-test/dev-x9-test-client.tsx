"use client";

import { useEffect, useRef, useState } from "react";
import {
  buildVectorTileSource,
  fetchVectorTileManifest,
  getVectorTileArtifact,
} from "@/lib/map/vector-tile-manifest";
import { loadNaverMaps } from "@/lib/naver-maps";

type MapboxGLLike = {
  addSource?: (id: string, src: Record<string, unknown>) => void;
  addLayer?: (layer: Record<string, unknown>, beforeId?: string) => void;
  getSource?: (id: string) => unknown;
  on?: (event: string, layer: string, handler: (e: unknown) => void) => void;
  isStyleLoaded?: () => boolean;
};

async function waitForMapbox(naverMap: naver.maps.Map): Promise<MapboxGLLike> {
  for (let i = 0; i < 60; i++) {
    const mb = (naverMap as unknown as { _mapbox?: MapboxGLLike })._mapbox;
    if (mb && typeof mb.addSource === "function") return mb;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("mapbox-gl polling timeout");
}

async function waitForStyleLoaded(mb: MapboxGLLike, isCancelled: () => boolean): Promise<void> {
  for (let i = 0; i < 60 && !mb.isStyleLoaded?.(); i++) {
    await new Promise((resolve) => setTimeout(resolve, 100));
    if (isCancelled()) return;
  }
}

async function registerDevParcelLayer(
  mb: MapboxGLLike,
  setStatus: (status: string) => void,
  setClickedPnu: (pnu: string | null) => void,
  isCancelled: () => boolean,
): Promise<void> {
  setStatus("Fetching platform-core manifest...");
  const manifest = await fetchVectorTileManifest();
  if (isCancelled()) return;

  const artifact = getVectorTileArtifact(manifest, "parcels");
  if (!artifact) {
    setStatus("Platform-core manifest has no parcels artifact");
    return;
  }

  const vectorSource = buildVectorTileSource(manifest, "parcels", { promoteId: "PNU" });
  setStatus(`Registering vector tile source: ${vectorSource.tiles[0]}`);

  if (!mb.getSource?.("parcels")) {
    mb.addSource?.("parcels", vectorSource);
    mb.addLayer?.({
      id: "parcels-fill",
      type: "fill",
      source: "parcels",
      "source-layer": artifact.source_layer,
      minzoom: artifact.render_min_zoom,
      maxzoom: artifact.render_max_zoom,
      paint: {
        "fill-color": "#10B981",
        "fill-opacity": 0.45,
        "fill-outline-color": "#047857",
      },
    });
    mb.addLayer?.({
      id: "parcels-outline",
      type: "line",
      source: "parcels",
      "source-layer": artifact.source_layer,
      minzoom: artifact.render_min_zoom,
      maxzoom: artifact.render_max_zoom,
      paint: {
        "line-color": "#064E3B",
        "line-width": 1.5,
      },
    });
    mb.on?.("click", "parcels-fill", (event: unknown) => {
      const evt = event as {
        features?: Array<{ properties?: { PNU?: string; pnu?: string } }>;
      };
      const props = evt.features?.[0]?.properties;
      const pnu = props?.PNU ?? props?.pnu ?? null;
      setClickedPnu(typeof pnu === "string" ? pnu : "(no PNU)");
    });
  }

  setStatus(`Platform-core parcels layer active: ${vectorSource.tiles[0]}`);
}

async function initializeDevX9Map(
  naverNs: typeof naver,
  container: HTMLDivElement,
  setStatus: (status: string) => void,
  setClickedPnu: (pnu: string | null) => void,
  isCancelled: () => boolean,
): Promise<void> {
  setStatus("Initializing map...");

  const map = new naverNs.maps.Map(container, {
    center: new naverNs.maps.LatLng(37.471588, 127.118683),
    zoom: 17,
    minZoom: 14,
    maxZoom: 19,
    gl: true,
    zoomControl: true,
    mapTypeControl: false,
  } as naver.maps.MapOptions);

  setStatus("Waiting for mapbox-gl bridge...");
  const mb = await waitForMapbox(map);
  if (isCancelled()) return;

  if (process.env.NODE_ENV !== "production") {
    (window as unknown as Record<string, unknown>).__devMb = mb;
    (window as unknown as Record<string, unknown>).__devMap = map;
  }

  await waitForStyleLoaded(mb, isCancelled);
  if (isCancelled()) return;
  await registerDevParcelLayer(mb, setStatus, setClickedPnu, isCancelled);
}

export function DevX9TestClient() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<string>("Loading...");
  const [clickedPnu, setClickedPnu] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setStatus("Loading Naver Maps SDK...");

    loadNaverMaps()
      .then(async (naverNs) => {
        if (cancelled || !containerRef.current) return;
        await initializeDevX9Map(
          naverNs,
          containerRef.current,
          setStatus,
          setClickedPnu,
          () => cancelled,
        );
      })
      .catch((error: unknown) => {
        setStatus(`Failed: ${error instanceof Error ? error.message : String(error)}`);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col">
      <header className="flex items-center justify-between bg-emerald-600 px-4 py-2 text-white">
        <div className="font-mono text-sm">SP9 ADR 0036 vector tile manifest check</div>
        <div className="font-mono text-xs">{status}</div>
      </header>
      <div ref={containerRef} className="flex-1" />
      {clickedPnu && (
        <div className="bg-black/80 px-4 py-2 font-mono text-sm text-white">
          Clicked PNU: <span className="font-bold text-emerald-400">{clickedPnu}</span>
        </div>
      )}
    </div>
  );
}
