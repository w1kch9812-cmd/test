"use client";
import { useEffect, useRef, useState } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import { useListingsQuery } from "@/lib/listings/use-listings-query";
import { loadNaverMaps } from "@/lib/naver-maps";
import { type MapboxGLLike, registerPmtilesSourceType, waitForMapbox } from "@/lib/pmtiles";
import { useListingsStore } from "@/stores/listings";

/**
 * WebGL context lost/restored 핸들러 등록. 모바일 백그라운딩/GPU 메모리 부족 대응.
 * 실패 시 cleanup 미반환 (등록 자체가 안 됨).
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
      // resize / triggerRepaint 는 mapbox-gl Map 메서드. MapboxGLLike 의 추가 형 단언.
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
 * PMTiles 폴리곤 source/layer 등록 — ADR 0019 (addSourceType 표준 path).
 *
 * 클라이언트가 PMTiles 파일에 *직접* HTTP byte-range request → server proxy 0.
 * `mb.addSourceType("pmtiles", PMTilesSource)` 등록은 `setupPolygonLayers` 호출 *이전*
 * 에 완료되어야 함 (호출자 책임).
 *
 * env override (외부 cached PMTiles 의 source-layer 이름이 다를 때만):
 * - `NEXT_PUBLIC_PMTILES_PARCELS_LAYER` (default `parcels`)
 * - `NEXT_PUBLIC_PMTILES_ADMIN_LAYER`   (default `admin`)
 * - `NEXT_PUBLIC_PMTILES_COMPLEX_LAYER` (default `complex`)
 */
const PARCELS_LAYER = process.env.NEXT_PUBLIC_PMTILES_PARCELS_LAYER || "parcels";
const ADMIN_LAYER = process.env.NEXT_PUBLIC_PMTILES_ADMIN_LAYER || "admin";
const COMPLEX_LAYER = process.env.NEXT_PUBLIC_PMTILES_COMPLEX_LAYER || "complex";

const PMTILES_BASE = process.env.NEXT_PUBLIC_PMTILES_BASE_URL || "/pmtiles/";

function pmtilesUrl(filename: string): string {
  const base = PMTILES_BASE.endsWith("/") ? PMTILES_BASE : `${PMTILES_BASE}/`;
  return `${base}${filename}`;
}

function setupPolygonLayers(mb: MapboxGLLike, onParcelClick: (pnu: string) => void): void {
  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer 미지원 — 폴리곤 skip");
    return;
  }

  try {
    // ===== 행정구역 (admin) =====
    if (!mb.getSource("admin")) {
      mb.addSource("admin", { type: "pmtiles", url: pmtilesUrl("admin.pmtiles") });
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
    if (!mb.getSource("complex")) {
      mb.addSource("complex", { type: "pmtiles", url: pmtilesUrl("complex.pmtiles") });
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
    if (!mb.getSource("parcels")) {
      mb.addSource("parcels", {
        type: "pmtiles",
        url: pmtilesUrl("parcels.pmtiles"),
        // tile 안의 PNU attribute → mapbox-gl feature.id (setFeatureState 등에 사용).
        // 대문자 `PNU` (design-lab tippecanoe 출력 표준) — 우리 ETL 은 소문자 `pnu`.
        // T3b.3 합의 후 attribute 이름 align 필요.
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

      // mapbox-gl 인스턴스 polling → addSourceType 등록 → PMTiles source 등록.
      // ADR 0019: server proxy 폐기, 표준 mapbox-gl v2 path.
      waitForMapbox(map)
        .then(async (mb) => {
          if (cancelled) return;
          if (process.env.NODE_ENV !== "production") {
            (window as unknown as Record<string, unknown>).__listingMb = mb;
          }
          // SourceType 등록 *먼저* — 그 후 addSource(type:"pmtiles") 가능.
          const ok = await registerPmtilesSourceType(mb);
          if (cancelled) return;
          if (!ok) {
            console.warn("[ListingMap] addSourceType 실패 — PMTiles 폴리곤 비활성");
            return;
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
