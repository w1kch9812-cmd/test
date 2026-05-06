"use client";
import { useEffect, useRef, useState } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import { useListingsQuery } from "@/lib/listings/use-listings-query";
import { loadNaverMaps } from "@/lib/naver-maps";
import { pmtilesSourceUrl, registerPmtilesProtocol } from "@/lib/pmtiles";
import { useListingsStore } from "@/stores/listings";

interface MapboxLike {
  getCanvas?: () => HTMLCanvasElement;
  resize?: () => void;
  triggerRepaint?: () => void;
  // biome-ignore lint/suspicious/noExplicitAny: mapbox-gl source/layer config 타입은 라이브러리 dependent
  addSource?: (id: string, source: any) => void;
  // biome-ignore lint/suspicious/noExplicitAny: mapbox-gl layer config
  addLayer?: (layer: any) => void;
  // biome-ignore lint/suspicious/noExplicitAny: mapbox-gl event handler
  on?: (event: string, layer: string, handler: (e: any) => void) => void;
  queryRenderedFeatures?: (
    point: [number, number],
    options?: { layers?: string[] },
    // biome-ignore lint/suspicious/noExplicitAny: feature 타입은 source-layer 별 다름
  ) => any[];
}

/**
 * WebGL context lost/restored 핸들러 등록. 모바일 백그라운딩/GPU 메모리 부족 대응.
 * 실패 시 cleanup 미반환 (등록 자체가 안 됨).
 */
function setupWebGlRecovery(mb: MapboxLike): (() => void) | undefined {
  if (typeof mb.getCanvas !== "function") return undefined;
  const glCanvas = mb.getCanvas();
  if (!glCanvas) return undefined;
  const onLost = (e: Event) => {
    e.preventDefault();
    console.warn("[ListingMap] WebGL context lost");
  };
  const onRestored = () => {
    try {
      mb.resize?.();
      mb.triggerRepaint?.();
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
 * PMTiles 폴리곤 source/layer 등록 — ADR 0016. 실패 (URL 미설정 / 프로토콜 미등록 /
 * 소스 추가 실패) 시 silent fallback — 지도 자체는 정상 동작, 폴리곤만 안 그려짐.
 *
 * 클릭 시 properties.pnu 추출 → patchFilters({ pnu }) 트리거 (ADR 0018 폴리곤 클릭 모델).
 */
/**
 * PMTiles 의 *내부 layer 이름* — tippecanoe `-l <name>` 결과. 우리 ETL (SP9 T3b.2)
 * 은 `parcels` 로 빌드. 단, 일시적으로 외부 cached PMTiles (예: 형제 repo 의 `lots`
 * layer) 를 borrow 해 쓸 때만 env 로 override.
 *
 * - `NEXT_PUBLIC_PMTILES_PARCELS_LAYER` (default `parcels`)
 * - `NEXT_PUBLIC_PMTILES_ADMIN_LAYER`   (default `admin`)
 * - `NEXT_PUBLIC_PMTILES_COMPLEX_LAYER` (default `complex`)
 */
const PARCELS_LAYER = process.env.NEXT_PUBLIC_PMTILES_PARCELS_LAYER || "parcels";
const ADMIN_LAYER = process.env.NEXT_PUBLIC_PMTILES_ADMIN_LAYER || "admin";
const COMPLEX_LAYER = process.env.NEXT_PUBLIC_PMTILES_COMPLEX_LAYER || "complex";

function setupPolygonLayers(mb: MapboxLike, onParcelClick: (pnu: string) => void): void {
  const parcelsUrl = pmtilesSourceUrl("parcels.pmtiles");
  const adminUrl = pmtilesSourceUrl("admin.pmtiles");
  const complexUrl = pmtilesSourceUrl("complex.pmtiles");

  if (!parcelsUrl && !adminUrl && !complexUrl) {
    console.info(
      "[ListingMap] NEXT_PUBLIC_PMTILES_BASE_URL 미설정 — 폴리곤 layer 비활성. SP9 T3 ETL 완료 후 활성화.",
    );
    return;
  }

  if (typeof mb.addSource !== "function" || typeof mb.addLayer !== "function") {
    console.warn("[ListingMap] mapbox addSource/addLayer 미지원 — 폴리곤 skip");
    return;
  }

  try {
    if (adminUrl) {
      mb.addSource("admin", { type: "vector", url: adminUrl });
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
    if (complexUrl) {
      mb.addSource("complex", { type: "vector", url: complexUrl });
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
    if (parcelsUrl) {
      mb.addSource("parcels", { type: "vector", url: parcelsUrl });
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
      // 클릭 핸들러 — parcels-fill 레이어만. ADR 0018 의 폴리곤 클릭 모델.
      if (typeof mb.on === "function") {
        mb.on(
          "click",
          "parcels-fill",
          (e: { features?: Array<{ properties?: { pnu?: string } }> }) => {
            const pnu = e.features?.[0]?.properties?.pnu;
            if (typeof pnu === "string" && pnu.length > 0) {
              onParcelClick(pnu);
            }
          },
        );
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

      // mb 인스턴스 attach 후 PMTiles 폴리곤 + WebGL 복구
      setTimeout(() => {
        const mb = (map as unknown as { _mapbox?: MapboxLike })._mapbox;
        if (!mb) return;
        if (registerPmtilesProtocol(mb)) {
          setupPolygonLayers(mb, (pnu) => patchFilters({ pnu }));
        }
        const recovery = setupWebGlRecovery(mb);
        if (recovery) cleanups.push(recovery);
      }, 500);

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
