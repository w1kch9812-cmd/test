import { env } from "@/lib/env";

let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK script lazy load. 한 번만 로드.
 *
 * `naver` 글로벌이 ready 되면 resolve. 두 번째 호출 시 캐시된 Promise 반환.
 * `submodules=clustering` 으로 marker clustering 도 사용 가능.
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
    script.src = `https://oapi.map.naver.com/openapi/v3/maps.js?ncpClientId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=clustering`;
    script.async = true;
    script.onload = () => resolve(naver);
    script.onerror = () => reject(new Error("Naver Maps SDK failed to load"));
    document.head.appendChild(script);
  });
  return _readyPromise;
}
