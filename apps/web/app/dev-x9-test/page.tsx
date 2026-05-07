"use client";
import { useEffect, useRef, useState } from "react";
import { loadNaverMaps } from "@/lib/naver-maps";

/**
 * SP9 ADR 0021 X9 path 시각 검증 페이지 (dev only).
 *
 * Naver Maps gl SDK 안에서 우리 정적 vector tile (`/dev-tiles/parcels/...`) 이
 * mapbox-gl 표준 `type:"vector"` source 로 렌더되는지 검증.
 *
 * 단독 page — auth/store/backend 의존성 0. listings 페이지의 production 코드 와
 * 동일한 wire pattern (proxy.ts 의 /dev-x9-test PUBLIC_PATHS).
 */

type MapboxGLLike = {
  addSource?: (id: string, src: Record<string, unknown>) => void;
  addLayer?: (layer: Record<string, unknown>, beforeId?: string) => void;
  getSource?: (id: string) => unknown;
  on?: (event: string, layer: string, handler: (e: unknown) => void) => void;
  isStyleLoaded?: () => boolean;
  once?: (event: string, handler: () => void) => void;
};

async function waitForMapbox(naverMap: naver.maps.Map): Promise<MapboxGLLike> {
  for (let i = 0; i < 60; i++) {
    const mb = (naverMap as unknown as { _mapbox?: MapboxGLLike })._mapbox;
    if (mb && typeof mb.addSource === "function") return mb;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("mapbox-gl polling timeout");
}

const TILES_BASE_URL = process.env.NEXT_PUBLIC_TILES_BASE_URL ?? "";
const PARCELS_LAYER = process.env.NEXT_PUBLIC_TILES_PARCELS_LAYER || "parcels";

export default function DevX9TestPage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<string>("loading…");
  const [clickedPnu, setClickedPnu] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setStatus("Naver SDK 로드 중…");

    loadNaverMaps()
      .then(async (naverNs) => {
        if (cancelled || !containerRef.current) return;
        setStatus("지도 init…");

        const map = new naverNs.maps.Map(containerRef.current, {
          // V-World fetch 한 데이터 bounds 의 center (metadata.json 기준)
          center: new naverNs.maps.LatLng(37.471588, 127.118683),
          zoom: 17,
          minZoom: 14,
          maxZoom: 19,
          gl: true,
          zoomControl: true,
          mapTypeControl: false,
        } as naver.maps.MapOptions);

        setStatus("mapbox-gl 인스턴스 polling…");
        const mb = await waitForMapbox(map);
        if (cancelled) return;
        if (process.env.NODE_ENV !== "production") {
          (window as unknown as Record<string, unknown>).__devMb = mb;
          (window as unknown as Record<string, unknown>).__devMap = map;
        }

        if (!TILES_BASE_URL) {
          setStatus("❌ NEXT_PUBLIC_TILES_BASE_URL 미설정");
          return;
        }

        // addSource 는 style.load 후에만 작동. polling — `once("style.load")` 는
        // *이벤트가 이미 fire 됐으면* callback 안 호출 (Naver fork edge case).
        for (let i = 0; i < 60 && !mb.isStyleLoaded?.(); i++) {
          await new Promise((r) => setTimeout(r, 100));
          if (cancelled) return;
        }

        const base = TILES_BASE_URL.endsWith("/") ? TILES_BASE_URL : `${TILES_BASE_URL}/`;
        const tileJsonUrl = `${base}parcels.json`;
        setStatus(`TileJSON source 등록 중… ${tileJsonUrl}`);

        if (!mb.getSource?.("parcels")) {
          // ADR 0021 SSS — Mapbox TileJSON spec. mapbox-gl 자동 fetch + zoom 적용.
          mb.addSource?.("parcels", {
            type: "vector",
            url: tileJsonUrl,
            promoteId: "PNU",
          });
          mb.addLayer?.({
            id: "parcels-fill",
            type: "fill",
            source: "parcels",
            "source-layer": PARCELS_LAYER,
            minzoom: 14,
            paint: {
              "fill-color": "#10B981",
              "fill-opacity": 0.45,
              "fill-outline-color": "#047857",
            },
          });
          // 추가 outline line layer — 시각 명확화 (시 검증용 강조)
          mb.addLayer?.({
            id: "parcels-outline",
            type: "line",
            source: "parcels",
            "source-layer": PARCELS_LAYER,
            minzoom: 14,
            paint: {
              "line-color": "#064E3B",
              "line-width": 1.5,
            },
          });
          mb.on?.("click", "parcels-fill", (e: unknown) => {
            const evt = e as {
              features?: Array<{ properties?: { PNU?: string; pnu?: string } }>;
            };
            const props = evt.features?.[0]?.properties;
            const pnu = props?.PNU ?? props?.pnu ?? null;
            setClickedPnu(typeof pnu === "string" ? pnu : "(no PNU)");
          });
        }
        setStatus(`✅ TileJSON wire 활성 (강남 z17, click 시 PNU 표시) — ${tileJsonUrl}`);
      })
      .catch((e: unknown) => {
        setStatus(`❌ 실패: ${e instanceof Error ? e.message : String(e)}`);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col">
      <header className="flex items-center justify-between bg-emerald-600 px-4 py-2 text-white">
        <div className="font-mono text-sm">SP9 ADR 0021 — X9 시각 검증 (dev only)</div>
        <div className="font-mono text-xs">{status}</div>
      </header>
      <div ref={containerRef} className="flex-1" />
      {clickedPnu && (
        <div className="bg-black/80 px-4 py-2 font-mono text-sm text-white">
          클릭 PNU: <span className="font-bold text-emerald-400">{clickedPnu}</span>
        </div>
      )}
    </div>
  );
}
