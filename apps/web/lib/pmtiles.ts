/**
 * PMTiles 통합 helper — Naver Maps gl SDK 의 mapbox-gl 백엔드 위에 vector tile
 * source 등록.
 *
 * SP9 T3b.2 진단 결과:
 * - Naver SDK 는 mapbox-gl v2 인스턴스를 `getMapbox()` getter 로 노출.
 * - 단, mapbox-gl namespace 자체 (`addProtocol` 함수 보유) 는 외부에 노출 X.
 * - 따라서 `pmtiles://` URL scheme 등록 path 사용 불가.
 *
 * 대신 design-lab 의 검증된 패턴: 서버사이드 API route 가 PMTiles 파일에서 단일
 * tile 추출 → `mb.addSource(type:'vector', tiles:[<API URL>])` 표준 패턴 사용.
 *
 * 본 모듈은 그 URL 빌더 + 인스턴스 polling 헬퍼만 제공. addProtocol / Protocol
 * import 모두 제거.
 */

/** PMTiles 파일 이름 (확장자 제외) → API route URL pattern. */
export function buildTileUrl(name: string): string {
  // 외부 R2 / Cloudflare Worker 우선, 미설정 시 Next.js API route fallback.
  const workerUrl = process.env.NEXT_PUBLIC_TILE_WORKER_URL;
  if (workerUrl && workerUrl.trim() !== "") {
    return `${workerUrl.replace(/\/$/, "")}/tiles/${name}/{z}/{x}/{y}.pbf`;
  }
  if (typeof window !== "undefined") {
    return `${window.location.origin}/api/tiles/${name}/{z}/{x}/{y}.pbf`;
  }
  return `/api/tiles/${name}/{z}/{x}/{y}.pbf`;
}

/** Naver SDK 의 `getMapbox()` 결과 타입 (mapbox-gl v2 Map 의 sub-shape). */
export interface MapboxGLLike {
  addSource: (id: string, source: unknown) => void;
  addLayer: (layer: unknown, beforeId?: string) => void;
  getSource: (id: string) => unknown;
  getLayer: (id: string) => unknown;
  getStyle?: () => { sources?: Record<string, unknown>; layers?: Array<{ id: string }> };
  isStyleLoaded?: () => boolean;
  on?: (
    event: string,
    layerOrCallback: string | ((e: unknown) => void),
    callback?: (e: unknown) => void,
  ) => void;
  queryRenderedFeatures?: (point?: unknown, options?: { layers?: string[] }) => unknown[];
  getCanvas?: () => HTMLCanvasElement;
}

/** Naver Map 인스턴스에서 mapbox-gl 인스턴스 추출. `getMapbox()` 우선, fallback `_mapbox`. */
export function getMapboxFromNaver(naverMap: unknown): MapboxGLLike | null {
  if (!naverMap || typeof naverMap !== "object") return null;
  const m = naverMap as { getMapbox?: () => MapboxGLLike | undefined; _mapbox?: MapboxGLLike };
  const mb = m.getMapbox?.() ?? m._mapbox;
  // 최소 sanity — addSource 가 함수면 진짜 mapbox-gl.
  if (mb && typeof mb.addSource === "function") return mb;
  return null;
}

/**
 * mapbox-gl 인스턴스가 준비될 때까지 polling. design-lab 의 `waitForMapboxGL`
 * 패턴 — 50회 × 100ms = 최대 5초. 그 후 추가로 style.load 폴링 10초.
 *
 * Naver SDK 의 `gl: true` 옵션이 raster 모드로 fallback 하는 환경 (headless
 * Chromium swiftshader 등) 에서는 영원히 mapbox 인스턴스 안 만들어짐 → reject.
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

  // style.load 까지 대기 (최대 10초).
  for (let i = 0; i < 100; i++) {
    if (mb.isStyleLoaded?.()) return mb;
    await new Promise((r) => setTimeout(r, interval));
  }
  // style 미로드 상태로도 반환 — 일부 호출자는 style.load 이벤트로 직접 listen.
  return mb;
}
