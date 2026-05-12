let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK 는 app/layout.tsx 의 <head> 에서 동기 로드된다 (gl 서브모듈이
 * WebGL 백엔드를 등록하려면 첫 Map 생성 시점 이전에 이미 로드되어야 함).
 *
 * 이 함수는 SDK 가 ready 될 때까지 polling 한 뒤 `naver` 글로벌을 resolve 한다.
 * 이미 ready 면 즉시 resolve. 두 번째 호출 시 캐시된 Promise 재사용.
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
    // Naver Maps SDK polling tuning constants — 로컬 SSOT.
    const POLL_INTERVAL_MS = 100;
    const POLL_TIMEOUT_MS = 10_000;
    const MAX_ATTEMPTS = POLL_TIMEOUT_MS / POLL_INTERVAL_MS;
    let attempts = 0;
    const tick = () => {
      attempts += 1;
      if (typeof naver !== "undefined" && naver.maps) {
        resolve(naver);
        return;
      }
      if (attempts >= MAX_ATTEMPTS) {
        reject(
          new Error(
            `Naver Maps SDK 로드 타임아웃 (${POLL_TIMEOUT_MS / 1000}s). app/layout.tsx 의 <head> script 확인 필요.`,
          ),
        );
        return;
      }
      setTimeout(tick, POLL_INTERVAL_MS);
    };
    tick();
  });
  return _readyPromise;
}
