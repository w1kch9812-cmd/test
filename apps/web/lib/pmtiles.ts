/**
 * SP9 ADR 0019 — PMTiles 통합 helper. addSourceType + VectorTileSource subclass path.
 *
 * 본 모듈:
 * - `getMapboxFromNaver` — Naver Map 인스턴스에서 mapbox-gl Map 추출
 * - `waitForMapbox` — gl init polling
 * - `registerPmtilesSourceType` — PMTilesSource 클래스를 mb 의 source registry 에 등록
 *
 * 등록 후 사용자는 표준 패턴:
 * ```ts
 * mb.addSource("parcels", { type: "pmtiles", url: "/pmtiles/parcels.pmtiles" });
 * mb.addLayer({ id: "parcels-fill", source: "parcels", "source-layer": "parcels", ... });
 * ```
 */

import { createPMTilesSourceClass } from "./pmtiles-source";

/** Naver SDK 의 `getMapbox()` 결과 타입 (mapbox-gl v2 Map 의 sub-shape). */
export interface MapboxGLLike {
  addSource: (id: string, source: unknown) => void;
  addLayer: (layer: unknown, beforeId?: string) => void;
  getSource: (id: string) => unknown;
  getLayer: (id: string) => unknown;
  // biome-ignore lint/suspicious/noExplicitAny: mapbox-gl v2 Source factory cb
  addSourceType?: (name: string, SourceClass: any, callback: (err?: Error) => void) => void;
  getStyle?: () => { sources?: Record<string, unknown>; layers?: Array<{ id: string }> };
  isStyleLoaded?: () => boolean;
  on?: (
    event: string,
    layerOrCallback: string | ((e: unknown) => void),
    callback?: (e: unknown) => void,
  ) => void;
  queryRenderedFeatures?: (point?: unknown, options?: { layers?: string[] }) => unknown[];
  getCanvas?: () => HTMLCanvasElement;
  // ADR 0019 — VectorTileSource subclass factory 사용을 위해 style 노출 필요.
  // biome-ignore lint/suspicious/noExplicitAny: minified Style class
  style?: any;
  // biome-ignore lint/suspicious/noExplicitAny: minified painter
  painter?: any;
}

/** `pmtiles` source type 이 한 mb 인스턴스에 1회만 등록되도록 추적. */
const _registeredOnMb = new WeakSet<object>();

/**
 * PMTilesSource 를 mb 인스턴스의 source type registry 에 등록.
 *
 * mb 인스턴스 별로 1회만 호출 (idempotent). factory 가 mb.style.constructor.getSourceType("vector")
 * 로 base class 받아 동적 subclass.
 *
 * 실패 (Naver fork 변경 / built-in vector source 못 찾음) 시 false → 호출자가 fallback.
 */
export async function registerPmtilesSourceType(mb: MapboxGLLike): Promise<boolean> {
  if (_registeredOnMb.has(mb as unknown as object)) return true;

  if (typeof mb.addSourceType !== "function") {
    console.warn("[pmtiles] mb.addSourceType 미지원 — Naver fork 변경 추정");
    return false;
  }

  let SourceClass: unknown;
  try {
    // biome-ignore lint/suspicious/noExplicitAny: factory needs raw mb shape
    SourceClass = createPMTilesSourceClass(mb as any);
  } catch (e) {
    console.warn("[pmtiles] PMTilesSource factory 실패:", (e as Error).message);
    return false;
  }

  return await new Promise<boolean>((resolve) => {
    mb.addSourceType?.("pmtiles", SourceClass, (err) => {
      if (err) {
        console.warn("[pmtiles] addSourceType 실패:", err.message);
        resolve(false);
        return;
      }
      _registeredOnMb.add(mb as unknown as object);
      resolve(true);
    });
  });
}

/** Naver Map 인스턴스에서 mapbox-gl 인스턴스 추출. `getMapbox()` 우선, fallback `_mapbox`. */
export function getMapboxFromNaver(naverMap: unknown): MapboxGLLike | null {
  if (!naverMap || typeof naverMap !== "object") return null;
  const m = naverMap as { getMapbox?: () => MapboxGLLike | undefined; _mapbox?: MapboxGLLike };
  const mb = m.getMapbox?.() ?? m._mapbox;
  if (mb && typeof mb.addSource === "function") return mb;
  return null;
}

/**
 * mapbox-gl 인스턴스가 준비될 때까지 polling (50회 × 100ms = 최대 5초 + style.load 10초).
 */
export async function waitForMapbox(
  naverMap: unknown,
  options?: { intervalMs?: number; maxAttempts?: number },
): Promise<MapboxGLLike> {
  const interval = options?.intervalMs ?? 100;
  const maxAttempts = options?.maxAttempts ?? 50;

  let mb: MapboxGLLike | null = null;
  for (let i = 0; i < maxAttempts; i++) {
    mb = getMapboxFromNaver(naverMap);
    if (mb) break;
    await new Promise((r) => setTimeout(r, interval));
  }
  if (!mb) throw new Error("mapbox-gl 인스턴스 polling timeout (Naver gl init 실패 추정)");

  for (let i = 0; i < 100; i++) {
    if (mb.isStyleLoaded?.()) return mb;
    await new Promise((r) => setTimeout(r, interval));
  }
  return mb;
}
