import { env } from "@/lib/env";

let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK script lazy load. 한 번만 로드.
 *
 * `naver` 글로벌이 ready 되면 resolve. 두 번째 호출 시 캐시된 Promise 반환.
 *
 * - `ncpKeyId` (NCP 새 API; 구 ncpClientId 는 deprecated)
 * - `submodules=gl,clustering` — WebGL 가속 (3D 지도) + 마커 클러스터링
 */
export function loadNaverMaps(): Promise<typeof naver> {
  if (_readyPromise) return _readyPromise;
  if (typeof window === "undefined") {
    return Promise.reject(new Error("loadNaverMaps must run in browser"));
  }
  _readyPromise = new Promise((resolve, reject) => {
    if (typeof naver !== "undefined" && naver.maps) {
      resolve(naver);
      return;
    }
    const script = document.createElement("script");
    script.src = `https://oapi.map.naver.com/openapi/v3/maps.js?ncpKeyId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=gl,clustering`;
    script.async = true;
    script.onload = () => resolve(naver);
    script.onerror = () => reject(new Error("Naver Maps SDK failed to load"));
    document.head.appendChild(script);
  });
  return _readyPromise;
}
