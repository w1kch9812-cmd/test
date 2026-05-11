"use client";
import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { pinIconHtml } from "@/components/listings/listing-pin";
import { useListingsQuery } from "@/lib/listings/use-listings-query";
import { loadNaverMaps } from "@/lib/naver-maps";
import { usePanelStack } from "@/lib/panel/use-panel-stack";
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
 * Vector tile 폴리곤 source/layer 등록 — ADR 0021 X9 + Mapbox `TileJSON` SSOT.
 *
 * 클라이언트는 `addSource({ type: "vector", url: ".../<layer>.json" })` 한 줄.
 * mapbox-gl 이 자동으로 TileJSON fetch → minzoom/maxzoom/tiles[] 적용. 우리 fetch 코드 0.
 *
 * env:
 * - `NEXT_PUBLIC_TILES_BASE_URL` (e.g. `https://r2/.../gold/v1/`) — 미설정 시 폴리곤 비활성
 *
 * `source-layer` 이름은 ETL `LayerKind::layer_name()` 와 1:1 — `LAYER_IDS` SSOT 참조.
 * minzoom/maxzoom 은 ETL `LayerKind::zoom_range()` 가 TileJSON 에 박제 — *프론트 hardcode 0*.
 * 단 *render zoom* (`addLayer({ minzoom })`) 은 도메인 정책 — 본 컴포넌트에 명시:
 * - `parcels-fill`: zoom 14+ (TileJSON minzoom 부터, 오버줌은 mapbox-gl 자동)
 * - `admin-fill` / `complex-fill`: ETL 빌드 후 활성
 */
const TILES_BASE_URL = process.env.NEXT_PUBLIC_TILES_BASE_URL ?? "";

function tileJsonUrl(layerName: string): string {
  const base = TILES_BASE_URL.endsWith("/") ? TILES_BASE_URL : `${TILES_BASE_URL}/`;
  return `${base}${layerName}.json`;
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

  // ===== 필지 (parcels) — TileJSON 자동 소비 =====
  try {
    if (!mb.getSource?.("parcels")) {
      mb.addSource("parcels", {
        type: "vector",
        url: tileJsonUrl("parcels"),
        // PNU attribute → mapbox-gl feature.id (setFeatureState).
        promoteId: "PNU",
      });
      mb.addLayer({
        id: "parcels-fill",
        type: "fill",
        source: "parcels",
        "source-layer": "parcels",
        minzoom: 14,
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
    // audit 2026-05-08: silent swallow → structured log (operator visibility).
    // *parcels* 는 핵심 layer — 실패 시 사용자 시각 영향 큼. error level.
    logMapLayerFailure("parcels-fill", err, { kind: "core", source: "parcels" });
  }

  // ===== 행정구역 (admin) — ETL 빌드 후 활성 (TileJSON 200 OK 시) =====
  try {
    if (!mb.getSource?.("admin")) {
      mb.addSource("admin", { type: "vector", url: tileJsonUrl("admin") });
      mb.addLayer({
        id: "admin-fill",
        type: "fill",
        source: "admin",
        "source-layer": "admin",
        maxzoom: 14, // 시군구 zoom 14 미만에서만 보임 (필지가 zoom 14+ 에서 바로 이어짐)
        paint: {
          "fill-color": "#9CA3AF",
          "fill-opacity": 0.05,
          "fill-outline-color": "#6B7280",
        },
      });
    }
  } catch (err) {
    // audit 2026-05-08: silent swallow → structured log. admin 은 *부가 layer* (gracefully
    // optional) — info level. ETL 미빌드 / TileJSON 404 = 예상 (degraded 정상).
    logMapLayerFailure("admin-fill", err, { kind: "optional", source: "admin" });
  }

  // ===== 산업단지 (complex) — ETL 빌드 후 활성 =====
  try {
    if (!mb.getSource?.("complex")) {
      mb.addSource("complex", { type: "vector", url: tileJsonUrl("complex") });
      mb.addLayer({
        id: "complex-fill",
        type: "fill",
        source: "complex",
        "source-layer": "complex",
        minzoom: 10,
        maxzoom: 16,
        paint: {
          "fill-color": "#3B82F6",
          "fill-opacity": 0.15,
          "fill-outline-color": "#1D4ED8",
        },
      });
    }
  } catch (err) {
    // audit 2026-05-08: silent swallow → structured log.
    logMapLayerFailure("complex-fill", err, { kind: "optional", source: "complex" });
  }
}

/**
 * Map layer 등록 실패 시 *구조화* log emit (audit 2026-05-08 fix).
 *
 * 이전 silent skip 의 문제: production 에서 *왜 폴리곤 안 보이는지* 불가시. 본 helper 는
 * - kind=core: error level (사용자 시각 영향 큰 layer 의 실패 — Sentry / Grafana alert 트리거)
 * - kind=optional: info level (ETL 미빌드 같은 *예상 degraded* 시나리오)
 * 둘 다 message structure 일관 — log aggregation 에서 grep 가능.
 *
 * Naver SDK 의 internal API (`_mapbox`) 의존 + style.load timing race 가 본 layer 의
 * 가장 흔한 실패 원인. ADR 0019 의 박제된 trade-off — 우회 불가능, observability 로 mitigation.
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
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const boundsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [mapReady, setMapReady] = useState(false);
  const setBounds = useListingsStore((s) => s.setBounds);
  const { push: pushPanel, stack } = usePanelStack();
  const top = stack.entries.at(-1);
  const selectedListingId = top?.kind === "listing" ? top.id : undefined;
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
        .then(async (mb) => {
          if (cancelled) return;
          if (process.env.NODE_ENV !== "production") {
            (window as unknown as Record<string, unknown>).__listingMb = mb;
          }
          // addSource 는 style.load 후에만 작동. polling — `once("style.load")` 는
          // 이벤트가 이미 fire 됐으면 callback 안 호출 (Naver fork edge case).
          for (let i = 0; i < 60 && !mb.isStyleLoaded?.(); i++) {
            await new Promise((r) => setTimeout(r, 100));
            if (cancelled) return;
          }
          if (cancelled) return;
          setupPolygonLayers(mb, (pnu) => pushPanel({ kind: "parcel", id: pnu, view: "summary" }));
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
  }, [setBounds, pushPanel]);

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
          content: pinIconHtml(listing.listing_type, {
            selected: listing.id === selectedListingId,
          }),
          anchor: new naver.maps.Point(14, 28),
        },
      });
      naver.maps.Event.addListener(marker, "click", () => {
        pushPanel({ kind: "listing", id: listing.id, view: "summary" });
      });
      markersRef.current.push(marker);
    }
  }, [mapReady, listings, selectedListingId, pushPanel]);

  return (
    <div className="relative h-full w-full">
      <div ref={containerRef} className="h-full w-full" />
      <MapAttribution />
    </div>
  );
}

/**
 * 공공누리 제1유형 출처표시 — Round 5 P2 compliance 실코드.
 *
 * 박제 source: V-World dtmk dsId=30563 (연속지적도). ADR 0027 / runbook § 7.2 의
 * 클라이언트 의무. Rust 측 `crates/sp9-base-layer-config::DTMK_LICENSE` /
 * `dtmk_source_url()` 와 *내용상* 동일 SSOT — 향후 manifest lineage fetch 로 동적
 * 렌더 가능 (별도 sprint).
 *
 * 위치: 지도 우하단. mapbox-gl 의 표준 attribution slot 과 시각적 분리 — Naver SDK
 * 의 자체 attribution 과 겹침 회피.
 */
function MapAttribution() {
  // Round 5 (final stop-hook): user-facing string 은 typed i18n only (AGENTS.md §10.1.5).
  // 이전 hardcoded "필지 polygon: V-World (...)" → next-intl `map.attribution.*` namespace.
  // 향후 lineage manifest 의 `source_license` / `source_url` 도 typed key 로 swap 가능.
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
