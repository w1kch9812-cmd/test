"use client";
import { useEffect, useRef, useState } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import { useListingsQuery } from "@/lib/listings/use-listings-query";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";

export function ListingMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const boundsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [mapReady, setMapReady] = useState(false);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);
  const query = useListingsQuery();
  const listings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  // 1. 지도 초기화 (1회) — gl WebGL 모드 + context recovery
  useEffect(() => {
    let cancelled = false;
    const cleanups: Array<() => void> = [];

    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      // gl: true → Naver Maps 가 mapbox-gl 백엔드로 WebGL 가속 렌더링.
      // 내부 mapbox 인스턴스는 (map as any)._mapbox 로 접근 (향후 deck.gl/custom layer 자리).
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

      // WebGL context lost/restored 핸들링 (모바일 백그라운딩, GPU 메모리 부족)
      // mapbox 내부 인스턴스는 init 후 약간 지연되어 attach 됨.
      setTimeout(() => {
        try {
          const mb = (
            map as unknown as {
              _mapbox?: {
                getCanvas: () => HTMLCanvasElement;
                resize: () => void;
                triggerRepaint?: () => void;
              };
            }
          )._mapbox;
          if (!mb || typeof mb.getCanvas !== "function") return;
          const glCanvas = mb.getCanvas();
          if (!glCanvas) return;
          const onLost = (e: Event) => {
            e.preventDefault();
            console.warn("[ListingMap] WebGL context lost");
          };
          const onRestored = () => {
            try {
              mb.resize();
              mb.triggerRepaint?.();
            } catch {
              /* ignore */
            }
          };
          glCanvas.addEventListener("webglcontextlost", onLost);
          glCanvas.addEventListener("webglcontextrestored", onRestored);
          cleanups.push(() => {
            glCanvas.removeEventListener("webglcontextlost", onLost);
            glCanvas.removeEventListener("webglcontextrestored", onRestored);
          });
        } catch {
          /* WebGL recovery 등록 실패는 치명적이지 않음 */
        }
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
  }, [setBounds]);

  // 2. 매물 변경 → marker 재생성 (mapReady 가 true 일 때만)
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
