"use client";
import { useEffect, useRef, useState } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import { useListingsQuery } from "@/lib/listings/use-listings-query";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";

/**
 * Naver Map 의 내부 mapbox-gl Map 인스턴스 — `(map as any)._mapbox` private API.
 * Naver SDK 가 exposed function 0 이라 차선책. ADR 0019 의 spike 진단 박제.
 */
type MapboxGLLike = {
  addSource?: (id: string, src: Record<string, unknown>) => void;
  addLayer?: (layer: Record<string, unknown>, beforeId?: string) => void;
  getSource?: (id: string) => unknown;
  getCanvas?: () => HTMLCanvasElement | null;
  on?: (event: string, layer: string, handler: (e: unknown) => void) => void;
  isStyleLoaded?: () => boolean;
  once?: (event: string, handler: () => void) => void;
};

/** Naver Map 의 내부 mapbox 인스턴스 polling — load 직후 ~수십 ms 미존재 가능. */
async function waitForMapbox(naverMap: naver.maps.Map): Promise<MapboxGLLike> {
  for (let i = 0; i < 60; i++) {
    const mb = (naverMap as unknown as { _mapbox?: MapboxGLLike })._mapbox;
    if (mb && typeof mb.addSource === "function") return mb;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("mapbox-gl 인스턴스 polling timeout (6s)");
}

/**
 * WebGL context lost/restored 핸들러 등록. 모바일 백그라운딩/GPU 메모리 부족 대응.
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
 * Vector tile 폴리곤 source/layer 등록 — ADR 0021 (PMTiles 분해 → 정적 .pbf, mapbox-gl 표준 100%).
 *
 * 클라이언트는 `type: "vector" + tiles: [URL_TEMPLATE]` 표준 source 그대로 사용.
 * addSourceType / Service Worker / Blob URL / private API 의존 0 (ADR 0019 의 모든 trick 폐기).
 *
 * env:
 * - `NEXT_PUBLIC_TILES_BASE_URL` (e.g. `https://r2.gongzzang.dev/gold/v3/`) — 미설정 시 폴리곤 비활성
 * - `NEXT_PUBLIC_TILES_PARCELS_LAYER` (default `parcels`) — vector tile 안의 source-layer 이름
 * - `NEXT_PUBLIC_TILES_ADMIN_LAYER`   (default `admin`)
 * - `NEXT_PUBLIC_TILES_COMPLEX_LAYER` (default `complex`)
 */
const TILES_BASE_URL = process.env.NEXT_PUBLIC_TILES_BASE_URL ?? "";
const PARCELS_LAYER = process.env.NEXT_PUBLIC_TILES_PARCELS_LAYER || "parcels";
const ADMIN_LAYER = process.env.NEXT_PUBLIC_TILES_ADMIN_LAYER || "admin";
const COMPLEX_LAYER = process.env.NEXT_PUBLIC_TILES_COMPLEX_LAYER || "complex";

function tilesUrlTemplate(layerDir: string): string {
  const base = TILES_BASE_URL.endsWith("/") ? TILES_BASE_URL : `${TILES_BASE_URL}/`;
  return `${base}${layerDir}/{z}/{x}/{y}.pbf`;
}

function setupPolygonLayers(mb: MapboxGLLike, onParcelClick: (pnu: string) => void): void {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer 미지원 — 폴리곤 skip");
    return;
  }
  if (!TILES_BASE_URL) {
    console.info("[ListingMap] NEXT_PUBLIC_TILES_BASE_URL 미설정 — 폴리곤 비활성 (지도 본체 정상)");
    return;
  }

  try {
    // ===== 행정구역 (admin) =====
    if (!mb.getSource?.("admin")) {
      mb.addSource("admin", {
        type: "vector",
        tiles: [tilesUrlTemplate("admin")],
        minzoom: 6,
        maxzoom: 12,
      });
      mb.addLayer({
        id: "admin-fill",
        type: "fill",
        source: "admin",
        "source-layer": ADMIN_LAYER,
        minzoom: 0,
        maxzoom: 16,
        paint: {
          "fill-color": "#9CA3AF",
          "fill-opacity": 0.05,
          "fill-outline-color": "#6B7280",
        },
      });
    }

    // ===== 산업단지 (complex) =====
    if (!mb.getSource?.("complex")) {
      mb.addSource("complex", {
        type: "vector",
        tiles: [tilesUrlTemplate("complex")],
        minzoom: 10,
        maxzoom: 15,
      });
      mb.addLayer({
        id: "complex-fill",
        type: "fill",
        source: "complex",
        "source-layer": COMPLEX_LAYER,
        minzoom: 12,
        paint: {
          "fill-color": "#3B82F6",
          "fill-opacity": 0.15,
          "fill-outline-color": "#1D4ED8",
        },
      });
    }

    // ===== 필지 (parcels) =====
    if (!mb.getSource?.("parcels")) {
      mb.addSource("parcels", {
        type: "vector",
        tiles: [tilesUrlTemplate("parcels")],
        minzoom: 14,
        maxzoom: 17,
        // tile 안의 PNU attribute → mapbox-gl feature.id (setFeatureState 등에 사용).
        // 대문자 `PNU` (design-lab tippecanoe 출력 표준). ETL T3b.3 정렬 시 align.
        promoteId: "PNU",
      });
      mb.addLayer({
        id: "parcels-fill",
        type: "fill",
        source: "parcels",
        "source-layer": PARCELS_LAYER,
        minzoom: 16,
        paint: {
          "fill-color": "#10B981",
          "fill-opacity": 0.1,
          "fill-outline-color": "#059669",
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
    console.warn("[ListingMap] 폴리곤 layer 추가 실패 — 무시", err);
  }
}

export function ListingMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const boundsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [mapReady, setMapReady] = useState(false);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);
  const patchFilters = useListingsStore((s) => s.patchFilters);
  const query = useListingsQuery();
  const listings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  // 1. 지도 초기화 (1회) — gl WebGL 모드 + context recovery + PMTiles 폴리곤
  useEffect(() => {
    let cancelled = false;
    const cleanups: Array<() => void> = [];

    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      // gl: true → Naver Maps 가 mapbox-gl 백엔드로 WebGL 가속 렌더링.
      // 내부 mapbox 인스턴스는 (map as any)._mapbox 로 접근.
      // ⚠️ swiftshader/headless 환경에서는 Naver SDK 가 WebGL 검사 실패 → raster fallback (canvas=0).
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978), // 서울 시청
        zoom: 8,
        minZoom: 7,
        maxZoom: 21,
        gl: true,
        zoomControl: false,
        mapTypeControl: false,
        disableKineticPan: false,
      } as naver.maps.MapOptions);
      mapRef.current = map;

      // bounds 변경 이벤트 (debounce 350ms)
      const boundsListener = naverNs.maps.Event.addListener(map, "bounds_changed", () => {
        if (boundsTimerRef.current) clearTimeout(boundsTimerRef.current);
        boundsTimerRef.current = setTimeout(() => {
          const bounds = map.getBounds() as naver.maps.LatLngBounds;
          const sw = bounds.getSW();
          const ne = bounds.getNE();
          setBounds({
            south: sw.lat(),
            west: sw.lng(),
            north: ne.lat(),
            east: ne.lng(),
          });
        }, 350);
      });
      cleanups.push(() => naverNs.maps.Event.removeListener(boundsListener));

      // 초기 bounds 도 emit
      const b = map.getBounds() as naver.maps.LatLngBounds;
      setBounds({
        south: b.getSW().lat(),
        west: b.getSW().lng(),
        north: b.getNE().lat(),
        east: b.getNE().lng(),
      });

      // dev-only window 노출 — 디버깅 / Playwright E2E 용. production 빌드 미포함.
      if (process.env.NODE_ENV !== "production") {
        (window as unknown as Record<string, unknown>).__listingMap = map;
      }

      // mapbox-gl 인스턴스 polling → 표준 vector source 등록.
      // ADR 0021: PMTiles 분해 → 정적 .pbf, mapbox-gl 의 가장 표준 source type.
      // addSourceType / Service Worker / Blob URL / private API 의존 0.
      waitForMapbox(map)
        .then((mb) => {
          if (cancelled) return;
          if (process.env.NODE_ENV !== "production") {
            (window as unknown as Record<string, unknown>).__listingMb = mb;
          }
          setupPolygonLayers(mb, (pnu) => patchFilters({ pnu }));
          const recovery = setupWebGlRecovery(mb);
          if (recovery) cleanups.push(recovery);
        })
        .catch((e: unknown) => {
          // GPU 가 swiftshader fallback 인 환경 (headless 등) 에서는 GL init 실패.
          // 이 경우 폴리곤 비활성. 지도 자체는 raster 모드로 정상 작동.
          console.warn(
            "[ListingMap] mapbox-gl 인스턴스 polling 실패 — 폴리곤 비활성:",
            e instanceof Error ? e.message : String(e),
          );
        });

      setMapReady(true); // marker useEffect 가 trigger 됨
    });
    return () => {
      cancelled = true;
      if (boundsTimerRef.current) {
        clearTimeout(boundsTimerRef.current);
        boundsTimerRef.current = null;
      }
      for (const fn of cleanups) fn();
    };
  }, [setBounds, patchFilters]);

  // 2. 매물 변경 → marker 재생성 (mapReady 가 true 일 때만)
  // ADR 0017 후속에서 BitmapStampCache 패턴으로 마이그레이션 예정.
  useEffect(() => {
    if (!mapReady || !mapRef.current) return;
    const map = mapRef.current;

    for (const m of markersRef.current) m.setMap(null);
    markersRef.current = [];

    for (const listing of listings) {
      const marker = new naver.maps.Marker({
        position: new naver.maps.LatLng(listing.lat, listing.lng),
        map,
        icon: {
          content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedId }),
          anchor: new naver.maps.Point(14, 28),
        },
      });
      naver.maps.Event.addListener(marker, "click", () => {
        setSelected(listing.id);
      });
      markersRef.current.push(marker);
    }
  }, [mapReady, listings, selectedId, setSelected]);

  return <div ref={containerRef} className="h-full w-full" />;
}
