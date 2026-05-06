# ADR 0019 — `PMTiles` 통합: built-in `VectorTileSource` subclass + `addSourceType` (mapbox-gl 표준 plugin)

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | Accepted (SSS-grade) |
| 선행 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md), [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md) |
| 폐기 | T3b.2 의 `/api/tiles/[...path]/route.ts` BFF proxy (commit `ecc52cc`) |

## 결정

`PMTiles` 통합을 **mapbox-gl 의 built-in `VectorTileSource` 클래스 subclass** + `addSourceType("pmtiles", PMTilesSource)` 로 한다. 표준 mapbox-gl plugin pattern.

```ts
// 부팅 시 1회 (mb 로딩 후)
const VectorTileSource = mb.style.constructor.getSourceType("vector");

class PMTilesSource extends VectorTileSource {
  constructor(id, options, dispatcher, eventedParent) {
    super(id, { ...options, type: "vector", tiles: [] }, dispatcher, eventedParent);
    this._pmtiles = new PMTiles(options.url);
  }
  load() { /* PMTiles header → fire 'data' metadata */ }
  loadTile(tile, callback) { /* PMTiles.getZxy → tile.actor.send('loadTile', { type:'vector', rawData }) */ }
}

mb.addSourceType("pmtiles", PMTilesSource, () => {});
mb.addSource("parcels", { type: "pmtiles", url: "/pmtiles/parcels.pmtiles" });
```

클라이언트가 PMTiles 단일 파일에 *직접* HTTP byte-range request. mapbox-gl 의 worker-side vector tile parser (Naver bundle 안의 default `vector` worker source) 가 raw .pbf bytes 를 그대로 parsing.

## 검토한 대안 (전수)

### C — Next.js BFF proxy (`/api/tiles`)
T3b.2 commit `ecc52cc` 검증. 거부:
- Rust backend 정책 (services/api) 와 일관성 위반
- mapbox-gl single-file + range request 이점이 server 안쪽만 발휘
- DAU 1000 시 server CPU 부하

### A1 — `addSourceType` + 우리 자체 Evented + main-only
T3b.x spike. 거부:
- 자체 Evented impl 이 mapbox-gl SourceCache 의 wrap 과 wire 안 됨
- 결과: load() / loadTile() 영원히 호출 안 됨

### A2 — built-in VectorTileSource subclass + addSourceType (본 결정)
*표준 mapbox-gl plugin pattern*. 작동 원리:
- Naver bundle 의 `mb.style.constructor.getSourceType("vector")` 가 built-in VectorTileSource 클래스 반환 (전수조사 확인)
- subclass 하면 mapbox-gl 의 `Evented` machinery + lifecycle 자동 inherit
- `load()` 만 override 해서 TileJSON fetch 대신 PMTiles header fetch
- `loadTile()` 만 override 해서 URL fetch 대신 PMTiles.getZxy + `params.rawData`
- worker-side parsing 은 default VectorWorkerSource 가 그대로 처리 (Naver bundle 안에 이미 있음)

### A3 — `addSourceType` + workerSourceURL (우리 자체 worker bundle)
*pure mapbox-gl plugin*. 거부:
- 우리 worker 가 vector tile parsing 까지 reimplement (`@mapbox/vector-tile` + `pbf` 번들 + WorkerTile.parse() 의 ~500 LOC 재작성)
- worker context 디버깅 어려움
- 작업량 days~weeks
- A2 와 결과 동일하지만 훨씬 복잡

### D — Service Worker intercept
Web platform 표준 transport layer. 거부:
- mapbox-gl 표준 plugin pattern 안에 있지 않음 — 한 단계 추상화 멀어짐
- TypeScript type safety 낮음 (URL string convention)
- 첫 페이지 로드 race
- dev hot reload + sw scope/lifetime gotcha
- *유일한 강점* (mapbox-gl 변경 영향 0) — Naver fork 의 mapbox-gl v2 표준 보존을 가정하면 무관

### E / B — monkey-patch
fetch / `_requestManager._transformRequestFn` patch. 비표준 / private API 의존. 거부.

## SSS 7기둥 매핑 (A2 채택 근거)

| 기둥 | 매핑 |
|---|---|
| 일관성 | mapbox-gl 표준 plugin pattern. SourceClass 가 다른 source (vector / geojson / raster) 와 동일 lifecycle 따름. |
| 자동강제 | mapbox-gl SourceCache 가 source.load() / source.loadTile() 자동 호출. 우회 불가. |
| 추적성 | TypeScript class — 코드 읽으면 PMTiles plugin 자명. Source 이름 ("pmtiles") grep 가능. |
| 안전성 | mapbox-gl Evented/SourceCache lifecycle inherit — 우리 자체 reimpl 0. type-safe params. |
| 가시성 | main thread + 표준 worker 양쪽 console.log + Network panel. service worker 별도 tab 불필요. |
| SSOT | mapbox-gl source registry (`De.setSourceType`) 가 단일 진실. dev/production 동일 path. |
| 명확성 | "PMTiles 처리는 PMTilesSource class 가" — 1줄 규칙. 추측 0. |

## 장기 strategic value

1. **mapbox-gl ecosystem 호환** — `mapbox-gl-pmtiles` (npm) 가 addProtocol 사용. addProtocol 이 다시 들어오면 (Naver SDK 업그레이드) 우리 코드도 그 패턴으로 1줄 변경.
2. **다른 custom source 추가 시 동일 pattern** (예: 미래 `mvt-zip`, `cog-tiff`).
3. **Type safety** — class fields/methods IDE 자동완성.
4. **Production debugging** — main thread / worker 양쪽 표준 DevTools.

## 영향

### 신규
- `apps/web/lib/pmtiles-source.ts` — factory `createPMTilesSourceClass(mb)` 가 mb 의 VectorTileSource 를 base 로 PMTilesSource 반환.

### 수정
- `apps/web/lib/pmtiles.ts` — `registerPmtilesSourceType(mb)` 가 factory 호출 + addSourceType 등록.
- `apps/web/components/listings/listing-map.tsx` — 표준 `addSource({type:"pmtiles", url})` pattern.

### 폐기
- `apps/web/app/api/tiles/[...path]/route.ts` — BFF proxy
- `apps/web/proxy.ts` 의 `/api/tiles` allowlist
- `apps/web/lib/pmtiles.ts` 의 `buildTileUrl()` (legacy)

## 후속 (T3b.x)

- **fallback 유지** — A2 가 어떤 이유로든 실패 시 C path (`/api/tiles` BFF) 환경변수 토글로 부활 가능. archive 보관.
- **promoteId attribute name align** — 우리 ETL 출력 (`pnu` 소문자) vs design-lab cached (`PNU` 대문자). T3b.3 합의 후 결정.
- **observability** — `loadTile` 안에서 latency / cache hit 측정. SP7 wiring.

## 참고

- mapbox-gl SourceInterface: <https://docs.mapbox.com/mapbox-gl-js/api/sources/#sourceinterface>
- pmtiles JS lib: <https://github.com/protomaps/PMTiles>
- T3b.x bundle 분석 + runtime probe: `var/sample/maps-gl.js`, `var/sample/naver-mb-surface.json`
- Naver fork worker pipeline contract (mapbox-gl v2 표준 그대로):
  ```js
  self.registerWorkerSource = (name, cls) => { ... }
  loadWorkerSource(t, e, i) { this.self.importScripts(e.url); i(); }
  ```
- Naver fork built-in source registry (`De.setSourceType` / `De.getSourceType`) — mapbox-gl v2 표준.
