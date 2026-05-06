/**
 * SP9 ADR 0019 — Service Worker 등록 + 첫 로드 race handle.
 *
 * 패턴 (표준):
 * 1. SW 측: `install: skipWaiting()` + `activate: clients.claim()` — 즉시 통제
 * 2. 앱 측: `register()` → 이미 controller 면 OK / 아니면 `controllerchange` 이벤트 대기
 *
 * 첫 로드 시 ~200-500ms 지연 (download SW + install + activate + claim).
 * 이후 모든 로드 = controller 즉시. 1회성 비용.
 */

const SW_URL = "/sw-pmtiles.js";

let _swReady: Promise<void> | null = null;

/**
 * Service Worker 활성 보장. mb.addSource (PMTilesSource) *전* 에 호출.
 *
 * - SW 미지원 브라우저: silently skip (PMTiles 폴리곤 미작동, 지도 본체는 정상)
 * - SW 등록 실패: console.warn, skip
 * - SW 활성 완료 시 resolve
 *
 * 같은 페이지에서 여러 번 호출해도 한 번만 register (Promise cache).
 */
export function ensureSwActive(): Promise<void> {
  if (_swReady) return _swReady;

  if (typeof navigator === "undefined" || !("serviceWorker" in navigator)) {
    _swReady = Promise.resolve();
    return _swReady;
  }

  _swReady = (async () => {
    console.info("[sw-register] register 시작");
    try {
      const reg = await navigator.serviceWorker.register(SW_URL, { scope: "/" });
      console.info("[sw-register] register 성공", {
        scope: reg.scope,
        active: !!reg.active,
        installing: !!reg.installing,
        waiting: !!reg.waiting,
      });
    } catch (err) {
      console.warn("[sw-register] register 실패 — PMTiles 폴리곤 비활성:", err);
      return;
    }
    if (navigator.serviceWorker.controller) {
      console.info("[sw-register] 이미 controller — resolve 즉시");
      return;
    }
    console.info("[sw-register] controllerchange 대기 시작");

    // 첫 로드 — claim 까지 대기.
    await new Promise<void>((resolve) => {
      const onChange = () => {
        if (navigator.serviceWorker.controller) {
          console.info("[sw-register] controllerchange — controller 활성, resolve");
          navigator.serviceWorker.removeEventListener("controllerchange", onChange);
          resolve();
        }
      };
      navigator.serviceWorker.addEventListener("controllerchange", onChange);
      if (navigator.serviceWorker.controller) {
        console.info("[sw-register] race 후 controller 확인 — resolve");
        navigator.serviceWorker.removeEventListener("controllerchange", onChange);
        resolve();
      }
    });
  })();

  return _swReady;
}
