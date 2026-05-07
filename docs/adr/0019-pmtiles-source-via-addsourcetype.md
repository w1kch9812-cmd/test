# ADR 0019 — `PMTiles` 통합: `VectorTileSource` subclass + Service Worker transport

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | **Superseded by [ADR 0021](./0021-static-vector-tile-decomposition.md)** (2026-05-07 EOD) — A2+SW spike 결과 worker uncontrolled wall (commit `28d7eb2`); 본 ADR 의 "전수 검토" 가 X9 (PMTiles 분해 → flat .pbf) / X10 (rawData transfer) 두 path 누락 — ADR 0021 이 X9 채택 (trick 0, mapbox-gl 표준 100%) |
| 선행 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md), [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md) |
| 폐기 | T3b.2 의 `/api/tiles/[...path]/route.ts` BFF proxy (commit `ecc52cc`) |

## 후속 정정 (2026-05-07 EOD)

본 ADR 의 채택 (A2-plugin + D-transport via Service Worker) 은 spike (commit `28d7eb2`) 결과 **wall**:

> Service Worker 가 main thread fetch 만 가로챔 — mapbox-gl 의 vector tile worker 가 보내는 fetch 는 *worker scope 안의 별도 ServiceWorkerContainer*. SW 가 *worker thread 의 fetch 를 통제 못 함* (web platform spec 의 worker isolation).

또한 본 ADR 의 "검토한 대안 — 전수" (§ 아래) 는 **2개 path 누락**:

1. **X9** — PMTiles 분해 → flat `{z}/{x}/{y}.pbf` 정적 호스팅 (mapbox-gl 의 가장 표준 source `type:"vector" + tiles:[URL]`). trick 0, internal API 0.
2. **X10** — `params.data.rawData` ArrayBuffer transfer (am2222/mapbox-pmtiles 패턴). 3 trick 중 2개 제거.

**현재 채택** = [ADR 0021](./0021-static-vector-tile-decomposition.md) (X9). 본 ADR 의 결론 ("Naver SDK 폐기 안 하면 SSS 불가능") 은 **reject**. Naver SDK 안에서 X9 가 SSS 7기둥 100% 충족.

**본 ADR 은 historical record 로 보존** — *X9 / X10 검토 누락 + SW worker uncontrolled wall* 의 spike 결과 박제.

---

## (Original) 결정

`PMTiles` 통합 = **mapbox-gl plugin layer + Service Worker transport layer 의 분리** :

1. **Plugin layer (표준)**: `mb.style.constructor.getSourceType("vector")` 로 built-in `VectorTileSource` 클래스 추출 → factory 로 동적 subclass → `mb.addSourceType("pmtiles", PMTilesSource, cb)` 표준 등록.
2. **Transport layer (표준)**: 클라가 `tiles=["/__pmtiles__/<encoded>/{z}/{x}/{y}.pbf"]` 표준 URL pattern 으로 worker 한테 dispatch. **Service Worker** 가 `/__pmtiles__/` URL 가로채 PMTiles JS lib 으로 raw .pbf bytes 반환. mapbox-gl 의 standard VectorTileWorkerSource 가 그 bytes parsing.

```ts
// 부팅 시 1회 (mb 로딩 후, sw 활성 후):
const VectorTileSource = mb.style.constructor.getSourceType("vector");
class PMTilesSource extends VectorTileSource { ... }
mb.addSourceType("pmtiles", PMTilesSource, () => {});

// 사용:
mb.addSource("parcels", { type: "pmtiles", url: "/pmtiles/parcels.pmtiles" });
//                        ↑ 우리 spec
mb.addLayer({ id: "parcels-fill", source: "parcels", "source-layer": "parcels", ... });
```

## 검토한 대안 — 전수

### C — Next.js BFF proxy (`/api/tiles`)
T3b.2 commit `ecc52cc` 검증. 거부:
- Rust backend 정책 (`services/api`) 와 일관성 위반
- mapbox-gl single-file + range 이점이 *server 안쪽만* 발휘
- DAU 1000 시 server CPU 부하

### A1 — `addSourceType` + 우리 자체 Evented + main-only
T3b.x spike. 거부:
- 자체 Evented impl 이 mapbox-gl SourceCache 의 wrap 과 wire 안 됨
- load() / loadTile() 영원히 호출 안 됨

### A2-blob — `addSourceType` + Blob URL trick
T3b.x spike (commit `59e5785`). 작동은 함 (parcelsFill: 71). 거부:
- main thread 가 PMTiles fetch (UI thread 부담)
- 매 tile 마다 Blob alloc/free (메모리 churn)
- mapbox-gl internal API (`dispatcher`, `painter`, `actor`) `as any` 다수
- 영구 architectural debt

### A3-pure — `addSourceType` + workerSourceURL + own worker bundle
T3b.x spike. 거부 (구체적 wall):
- worker bundle importScripts 작동 ✅
- `self.fetch` monkey-patch 작동 — Naver tile 들 거쳐감 ✅
- 단 *PMTiles URL 만* worker fetch 까지 안 도착
- 진단: Naver fork 의 worker side ajax wrapper 가 *fetch reference 를 module load 시 closure 로 capture*. 우리 patch 가 *후* 적용되어 bypass.
- 또는 *진짜 standard A3* = 우리 own `PMTilesWorkerSource` class + vector tile parsing 전체 reimplementation (`@mapbox/vector-tile` + `pbf` + `WorkerTile.parse()` 의 bucket / glyph atlas / icon atlas 빌드 + style layer evaluation). 수천 LOC, 수주, 사실상 mapbox-gl fork 작성.
- ROI 마이너스.

### D-only — Service Worker intercept (mapbox plugin 없이)
거부 (이번 ADR 한정):
- mapbox-gl plugin 표준 안 따름 (URL 약속만)
- Discoverability 낮음 (string convention)

### B — `_requestManager._transformRequestFn` private API patch
거부 — Naver SDK 업그레이드 risk.

### E — `globalThis.fetch` + XHR monkey-patch (main thread)
거부 — globalThis side-effect, 다른 모듈 영향.

### A2 + Blob URL 깊은 진단 — 꼼수 3개
1. *main thread fetch* — 표준은 worker fetch
2. *Blob URL allocation per tile* — 메모리 churn, revoke 누락 시 leak
3. *internal API (dispatcher / painter / actor) `as any`* — mapbox-gl 패치 시 깨짐

이 셋이 *영구 architectural debt*. SSS 의 적.

## 채택 — A2 plugin + D transport (본 결정)

**plugin layer (mapbox-gl spec)** + **transport layer (web platform spec)** = 표준 두 개 조합.

```
[브라우저 main thread]
  PMTilesSource extends VectorTileSource
  ↓ super.loadTile() — *override 0*
  ↓ tile.actor.send("loadTile", { request: { url: "/__pmtiles__/..." }, ... })

[mapbox-gl worker thread]
  ↓ standard VectorTileWorkerSource.loadVectorData(params, callback)
  ↓ worker 의 ajax 가 "/__pmtiles__/..." fetch 시도

[브라우저의 transport layer]
  ↓ Service Worker 가 fetch 가로챔 (/__pmtiles__/ URL)
  ↓ PMTiles JS lib 으로 byte-range fetch + tile 추출
  ↓ Response (raw .pbf bytes) 반환

[mapbox-gl worker]
  ↓ 평범한 vector tile 으로 parsing → buckets
  ↓ main 에 send

[main thread]
  ↓ tile.loadVectorData (mapbox-gl orchestration 자동)
  ↓ painter 가 폴리곤 그림
```

**main thread fetch / Blob URL / internal API 의존 모두 0**.

## SSS 7기둥 매핑

| 기둥 | A2 + Blob | A2 + SW (본 결정) |
|---|---|---|
| 일관성 | ❌ — main 이 fetch | ✅ — worker fetch (표준 Mapbox plugin pattern) |
| 자동강제 | △ — Blob revoke 누락 시 leak | ✅ — sw scope 가 모든 ajax 자동 캡처 |
| 추적성 | △ — 코드 *왜* 모름 | ✅ — sw 의 fetch handler = single point |
| 안전성 | ❌ — `as any` 다수 | ✅ — internal API 의존 0 |
| 가시성 | △ | ✅ — DevTools Application > SW |
| SSOT | △ — main + worker 이중 path | ✅ — sw 가 단일 transport |
| 명확성 | △ — Blob URL trick | ✅ — `/__pmtiles__/` URL 규칙 |

## 첫 로드 race (인정 + 해결)

SW 첫 등록 시 `register → install → activate → claim → controller 됨` 까지 ~200-500ms. 그 사이 mapbox-gl 가 tile fetch 하면 *SW 거치지 않음* → 404 → 빈 polygon.

**해결 패턴 (표준)**:
- SW 측: `self.skipWaiting()` + `self.clients.claim()` — 첫 로드 페이지 즉시 통제.
- 앱 측: `await ensureSwActive()` (controllerchange 이벤트 대기) → `mb.addSource(...)`.

**비용**: 첫 페이지 로드 시 *폴리곤 layer 등록* 만 ~300ms 지연 (지도 본체 즉시). 이후 모든 로드 = 0. 1회성.

**A2 + Blob URL 의 *영구 꼼수* vs SW 의 *1회성 0.3초* — SSS 는 후자**.

## 거부된 후속 path (over-engineering)

| | 왜 |
|---|---|
| Offline cache (PWA) | 한국 LTE 99% — enterprise edge case 가치 낮음 |
| Predictive prefetch | vector tile viewport 변동성 높음 → 예측 정확도 ↓ → bandwidth 낭비 |
| WASM MVT decoder | 측정 데이터 없음, over-engineering |
| GPU compute decoding | "" |
| Predictive ML | "" |

**SSS = 추측 0, 측정 후만**.

## 채택 후속 (Tier A + 1 + 3)

### Tier A — production 위생 (0.5일)
- HTTP `Cache-Control: max-age=31536000, immutable` (PMTiles immutable assets)
- R2 한국 edge POP (production 자동)
- URL versioning (`parcels-v3.pmtiles` 또는 `?v=3` query) — manifest 기반 hot-swap
- (선택) Brotli compression — 측정 후

### Tier 1 — 자동 업데이트 (1일)
- ADR 0016 의 manifest hot-swap 의 SW path 활성화
- SW 가 `gold/manifest.json` polling → 새 `current_version` 감지 → cache invalidate + 신 버전 fetch
- 사용자 reload 없이 무중단 교체

### Tier 3 — 관측성 (SP7 wire, 1일)
- SW 가 매 fetch 의 latency / cache hit / origin / size 수집
- `postMessage` 로 main thread 에 metric 전달
- Sentry / Grafana 노출

## 영향

### 신규
- `apps/web/lib/workers/sw-pmtiles-src.ts` — Service Worker source (PMTiles JS lib + `/__pmtiles__/` fetch handler)
- `apps/web/public/sw-pmtiles.js` — esbuild bundle 결과
- `apps/web/lib/sw-register.ts` — 등록 + skipWaiting + controllerchange 대기
- `apps/web/lib/pmtiles-source.ts` — `createPMTilesSourceClass(mb)` factory (subclass VectorTileSource, `tiles: ["/__pmtiles__/..."]` 패턴)
- `apps/web/lib/pmtiles.ts` — `registerPmtilesSourceType(mb)` + `waitForMapbox(map)`
- `package.json` — `build:sw-pmtiles` esbuild 스크립트

### 수정
- `apps/web/components/listings/listing-map.tsx` — `await ensureSwActive()` → registerPmtilesSourceType → addSource
- `apps/web/proxy.ts` — `/sw-pmtiles.js` + `/__pmtiles__` PUBLIC_PATHS

### 폐기
- `apps/web/app/api/tiles/[...path]/route.ts` (BFF proxy)
- `apps/web/proxy.ts` 의 `/api/tiles` allowlist
- A2 + Blob URL trick (commit `59e5785`)
- A3 spike artifacts (`apps/web/lib/workers/pmtiles-worker-src.ts` 의 fetch hook 부분 — sw 로 이관)

## 참고

- mapbox-gl SourceInterface: <https://docs.mapbox.com/mapbox-gl-js/api/sources/#sourceinterface>
- Service Worker spec: <https://w3c.github.io/ServiceWorker/>
- pmtiles JS lib: <https://github.com/protomaps/PMTiles>
- T3b.x bundle 분석 + runtime probe: `var/sample/maps-gl.js`, `var/sample/naver-mb-surface.json`, `var/sample/naver-polygons.json`
- Naver fork worker pipeline contract (mapbox-gl v2 표준 그대로):

  ```js
  self.registerWorkerSource = (name, cls) => { ... }
  loadWorkerSource(t, e, i) { this.self.importScripts(e.url); i(); }
  ```
