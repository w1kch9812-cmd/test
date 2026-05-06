/**
 * PMTiles 통합 — Naver Maps `gl: true` 의 내부 mapbox 인스턴스 위에 PMTiles
 * vector source 등록.
 *
 * ADR 0016 (PMTiles 100% base layer) + ADR 0017 (단일 GL 캔버스 박자) 의 직접
 * 결과. 폴리곤은 mapbox-gl 의 같은 WebGL 캔버스 안에서 GPU 가 그림 — 별도
 * Canvas/DOM 레이어 만들지 않음.
 *
 * Naver SDK 의 mapbox-gl 버전은 명시되지 않음. `addProtocol` 은 mapbox-gl-js
 * v2.0+ 에서 도입 → 실패 가능성 있음. 본 모듈은 try/catch 로 defensive,
 * 실패 시 console.warn 후 폴리곤 layer 안 그려도 지도 자체는 정상 작동.
 */

import { Protocol } from "pmtiles";

let _protocolRegistered = false;
let _protocolError: Error | null = null;

/**
 * mapbox-gl 모듈을 Naver `_mapbox` 인스턴스에서 추출 (생성자 함수 / 클래스).
 * 일부 Naver 빌드는 globalThis 에 mapboxgl 도 노출 — 둘 중 가능한 경로 시도.
 */
function getMapboxGlNamespace(mb: unknown): unknown {
  // 우선 globalThis.mapboxgl (Naver SDK 가 노출하는 경우)
  // biome-ignore lint/suspicious/noExplicitAny: Naver private API
  const maybeGlobal = (globalThis as any).mapboxgl;
  if (maybeGlobal && typeof maybeGlobal.addProtocol === "function") {
    return maybeGlobal;
  }
  // 인스턴스의 constructor 체인을 따라 가서 정적 addProtocol 을 가진 클래스 찾기
  // biome-ignore lint/suspicious/noExplicitAny: Naver private API
  const ctor = (mb as any)?.constructor;
  if (ctor && typeof ctor.addProtocol === "function") {
    return ctor;
  }
  return null;
}

/**
 * PMTiles `pmtiles://` 프로토콜을 mapbox-gl 에 등록 (전 앱 1회).
 *
 * 성공 시 이후 `addSource({ type: 'vector', url: 'pmtiles://https://r2/.../parcels.pmtiles' })`
 * 가 작동.
 *
 * 실패 (Naver mapbox-gl 이 v1 이거나 addProtocol 미지원) → false 반환,
 * 호출자가 silent fallback (폴리곤 layer 생략) 처리.
 */
export function registerPmtilesProtocol(mapboxInstance: unknown): boolean {
  if (_protocolRegistered) return true;
  if (_protocolError) return false;

  try {
    const ns = getMapboxGlNamespace(mapboxInstance);
    if (!ns) {
      throw new Error(
        "mapboxgl namespace not found — Naver SDK 의 mapbox-gl 이 addProtocol 을 노출하지 않음 (v1?)",
      );
    }
    const protocol = new Protocol();
    // biome-ignore lint/suspicious/noExplicitAny: addProtocol 시그니처가 mapbox-gl 버전마다 다름
    (ns as any).addProtocol("pmtiles", protocol.tile);
    _protocolRegistered = true;
    return true;
  } catch (err) {
    _protocolError = err instanceof Error ? err : new Error(String(err));
    console.warn(
      "[pmtiles] registerProtocol failed — 폴리곤 layer 가 그려지지 않습니다. T5 후속에서 mapbox-gl 버전 확인 필요.",
      _protocolError.message,
    );
    return false;
  }
}

/**
 * PMTiles base URL 환경변수 — 미설정 시 폴리곤 layer 비활성 (T3 ETL 완료 전 정상 상태).
 *
 * 형식 예: `https://static.gongzzang.com/v1/`
 * 그 아래에 `parcels.pmtiles`, `admin.pmtiles`, `complex.pmtiles` 가 호스팅됨.
 */
export function getPmtilesBaseUrl(): string | undefined {
  const url = process.env.NEXT_PUBLIC_PMTILES_BASE_URL;
  if (!url || url.trim() === "") return undefined;
  return url.endsWith("/") ? url : `${url}/`;
}

/**
 * PMTiles vector source URL 빌드. `pmtiles://` 접두사로 protocol 디스패치 트리거.
 */
export function pmtilesSourceUrl(filename: string): string | undefined {
  const base = getPmtilesBaseUrl();
  if (!base) return undefined;
  return `pmtiles://${base}${filename}`;
}
