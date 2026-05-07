# ADR 0021 — PMTiles 분해 → 정적 `{z}/{x}/{y}.pbf` 호스팅 (mapbox-gl 표준 100%)

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | Accepted (SSS-grade — *진짜 SSS*) |
| 선행 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md), [ADR 0019](./0019-pmtiles-source-via-addsourcetype.md) |
| Supersedes | [ADR 0019](./0019-pmtiles-source-via-addsourcetype.md) (PMTilesSource subclass + Service Worker transport — A2+SW spike 결과 worker uncontrolled wall, A2+Blob URL 회귀 = 영구 3 trick) |

## 결정

PMTiles 단일 파일 분해 → flat `{z}/{x}/{y}.pbf` 정적 호스팅 (R2). 클라이언트는 mapbox-gl v2 의 **`type: "vector"` + `tiles: [URL_TEMPLATE]`** 표준 source 그대로 사용. **trick 0, internal API 0, monkey-patch 0**.

```ts
// 부팅 시 1회 — addSourceType 불필요 (표준 vector source).
mb.addSource("parcels", {
  type: "vector",  // mapbox-gl 의 가장 표준 source type
  tiles: ["https://r2.gongzzang.dev/gold/v3/parcels/{z}/{x}/{y}.pbf"],
  minzoom: 14,
  maxzoom: 17,
  promoteId: "PNU",
});
mb.addLayer({ id: "parcels-fill", source: "parcels", "source-layer": "parcels", ... });
```

## 컨텍스트 — ADR 0019 의 wall

ADR 0019 가 *전수 검토* 라고 박제한 대안 (C / A1 / A2-blob / A3-pure / D-only / B / E) 모두:

- C (BFF proxy) → Rust 정책 위반
- A1 (자체 Evented) → mapbox-gl SourceCache wire 안 됨
- A2-blob → 3 trick 영구 부채
- A3-pure → Naver fork worker 의 *fetch closure capture* wall
- A2+SW (ADR 0019 채택) → spike (`28d7eb2`) 결과 *Service Worker 가 mapbox-gl worker thread fetch 미통제* — web platform spec 의 worker isolation. **wall**.
- D-only / B / E → mapbox-gl 표준 비준수

ADR 0019 의 결론 = "Naver SDK 폐기 안 하면 SSS 불가능, A2+Blob URL 의 3 trick 이 *최대 SSS*". 단 본 결론은 *2개 path 누락* 으로 인한 *premature negative*:

1. **X10** — `params.data.rawData` ArrayBuffer transfer (am2222/mapbox-pmtiles 패턴, mapbox-gl v2 community 표준, 1.4k stars). 3 trick 중 2개 즉시 제거. 단 `params.data.rawData` 라는 *internal field 1개* 남음 — community 표준이지만 영구 부채.
2. **X9 (본 ADR 채택)** — PMTiles 분해 → flat .pbf 정적 호스팅. **trick 0**. mapbox-gl 의 가장 표준 source type. R2 의 single-file 이점 (atomic deploy) 만 포기.

## 결정 — X9 채택 (가장 SSS, 가장 표준, 가장 근본)

### 작동 메커니즘

```
[ETL Gold pipeline]
  R2 raw SHP (Bronze)
    ↓ ogr2ogr → GeoJSON
    ↓ tippecanoe → parcels.pmtiles (단일 파일)
    ↓ pmtiles extract / tile-join --output-to-directory  ← X9 신규 step
    ↓ flat directory: gold/v<N>/parcels/{z}/{x}/{y}.pbf
    ↓ aws-sdk-s3 batch upload to R2 (concurrent 100)
    ↓ manifest.json 의 current_version 업데이트 (atomic pointer flip)

[브라우저 — mapbox-gl 표준 path]
  mb.addSource({ type: "vector", tiles: [...{z}/{x}/{y}.pbf] })
    ↓ mapbox-gl worker 의 standard VectorTileWorkerSource
    ↓ standard fetch (Naver fork worker 의 ajax — 우리 가로채기 0)
    ↓ Cloudflare CDN edge cache (한국 POP)
    ↓ R2 origin (cache miss 시)
    ↓ raw .pbf bytes
    ↓ standard parsing → buckets → main thread paint
```

**모든 단계 = mapbox-gl spec 또는 web platform spec 그대로**. internal API 의존 0, monkey-patch 0, Blob URL 0, Service Worker 0, addSourceType 0.

### SSS 7기둥 매핑

| 기둥 | A2+Blob URL (ADR 0019 회귀) | X10 (rawData) | **X9 (본 ADR)** |
|---|---|---|---|
| 일관성 | ❌ — main fetch | △ — main fetch + rawData internal | ✅ — mapbox-gl `type:"vector"` 표준 |
| 자동강제 | △ — Blob revoke 누락 시 leak | △ — rawData transfer 책임 | ✅ — ETL 이 manifest hot-swap, CI 검증 |
| 추적성 | △ — *왜* 모름 | △ — internal field 의존 | ✅ — R2 object key = explicit lineage |
| 안전성 | ❌ — `as any` 다수 | △ — rawData internal 1개 | ✅ — internal API 의존 0 |
| 가시성 | △ | △ | ✅ — Network tab 표준 vector tile request, R2 dashboard |
| SSOT | △ — main + worker 이중 | △ — internal field 의존 | ✅ — Gold pipeline = single source |
| 명확성 | △ — Blob URL trick | △ — community convention | ✅ — `type:"vector" + tiles:[...]` = mapbox-gl 의 가장 well-documented pattern |

## 비용 견적 (한국 전국 z14-17 parcels + admin Z6-12 + complex Z0-16)

| 항목 | 값 |
|---|---|
| z14 parcels | ~17,000 tiles (~5,000 non-empty) |
| z15 parcels | ~67,000 tiles (~25,000 non-empty) |
| z16 parcels | ~270,000 tiles (~120,000 non-empty) |
| z17 parcels | ~1,070,000 tiles (~600,000 non-empty) |
| admin Z6-12 | ~50,000 tiles |
| complex Z0-16 | ~80,000 tiles (사용자 요구: 산단 모든 zoom visible — z0-5 sub-pixel coalesce + z6-16 full detail) |
| **총 R2 objects/build** | **~750k–1M** |
| R2 PUT cost | $4.50/1M = ~$3-4.5/build (월 1회) |
| R2 GET cost | DAU 1000 × 50 tile/세션 × 30일 = 1.5M = $0.54/월 |
| R2 Storage | 10-50 GB × $0.015/GB = $0.15-0.75/월 |
| **총 비용** | **~$5-10/월** (DAU 무관) |
| ETL upload time | 1M PUT / 100 concurrent = ~2.7시간 (cron 12h timeout 안에 충분) |

**Trade-off vs PMTiles 단일 파일**:

- ❌ **single-file atomic deploy 이점 포기** → manifest pointer flip (`gold/manifest.json` 의 `current_version` 한 줄 변경) 으로 동등 atomic 보장
- ❌ **byte-range 1 request → tile 1 request 로 분산** → Cloudflare CDN edge 가 각 tile 캐시 (한국 POP). 사용자 입장 latency 동일 (DAU 1000+ 에서 오히려 cache hit ratio 향상)
- ✅ **모든 trick 0** → SSS 7기둥 100%

## ETL pipeline 변경 (Rust)

### 현재 (`services/etl-base-layer/src/gold/`)

```text
shp_to_geojson.rs  — ogr2ogr SHP→GeoJSON
tippecanoe.rs      — tippecanoe spawn
build.rs           — orchestration (현재 산출물 = parcels.pmtiles 단일 파일)
manifest.rs        — GoldManifest schema
spawn.rs           — Win→WSL spawn helper
```

### X9 신규 step

```text
decompose.rs       — pmtiles 분해 (pmtiles-rs Reader API + 모든 (z,x,y) 순회 → R2 PutObject)
build.rs           — orchestration: tippecanoe → pmtiles → decompose → R2 batch upload
manifest.rs        — GoldManifest 에 tiles_url_template 필드 추가 (기존 pmtiles_url 과 *둘 다* 노출)
```

**재사용 가능 lib**:
- `pmtiles-rs` ([protomaps/PMTiles GitHub](https://github.com/protomaps/PMTiles/tree/main/rust)) — Reader / Writer / Header / Directory 표준 API
- 또는 felt fork `tippecanoe`'s `tile-join --no-tile-stats --output-to-directory <dir> input.pmtiles` (외부 spawn) — pmtiles-rs Rust 의존성 회피
- `aws-sdk-s3` (T3b.1 이미 통합) — `PutObject` batch concurrent

**Cache headers** (R2 객체 metadata):
- `Cache-Control: max-age=31536000, immutable` — flat tile 은 immutable (URL versioning 으로 무효화)
- `Content-Type: application/vnd.mapbox-vector-tile` (또는 `application/x-protobuf`)
- `Content-Encoding: gzip` (tippecanoe 출력은 기본 gzip)

## 프론트 변경 (`apps/web`)

### 폐기 (ADR 0019 SW spike 부산물)

- `apps/web/lib/pmtiles-source.ts` — VectorTileSource subclass
- `apps/web/lib/pmtiles.ts` — registerPmtilesSourceType / waitForMapbox
- `apps/web/lib/sw-register.ts` — Service Worker register
- `apps/web/lib/workers/sw-pmtiles-src.ts` — Service Worker source
- `apps/web/proxy.ts` 의 `/sw-pmtiles.js` + `/__pmtiles__` PUBLIC_PATHS
- `apps/web/proxy.ts` connectSrc 의 `blob:` (Blob URL 의존 0)

### 유지/수정

- `apps/web/components/listings/listing-map.tsx` — `addSource({ type: "pmtiles", url })` → `addSource({ type: "vector", tiles: [URL_TEMPLATE] })` 변경. addSourceType 호출 0, ensureSwActive 0. *코드 더 단순*.
- 환경변수: `NEXT_PUBLIC_PMTILES_BASE_URL=...` → `NEXT_PUBLIC_TILES_BASE_URL=https://r2.gongzzang.dev/gold/v3/` 로 변경. 우리 manifest 가 정한 prefix.

## 거부된 후속 path

- **PMTiles 도 함께 호스팅** (dual-publish) — 무의미. flat tile = mapbox-gl 표준 한 path 만 노출.
- **WebGL custom layer** (X3, ADR 0019 검토 누락 path) — 1-2주 + 우리 own shader/buffer 책임. flat tile X9 가 SSS 충족하면 over-engineering.
- **A2+rawData (X10)** — 1 trick 영구 남음 (params.data.rawData internal). X9 가 0 trick 이면 X10 채택 무가치.

## 영향

### 신규
- `services/etl-base-layer/src/gold/decompose.rs` — PMTiles 분해 + R2 batch upload
- `docs/adr/0021-static-vector-tile-decomposition.md` (본 파일)

### 수정
- `services/etl-base-layer/src/gold/build.rs` — orchestration 에 decompose step 추가
- `services/etl-base-layer/src/gold/mod.rs` — pub mod decompose
- `services/etl-base-layer/src/gold/manifest.rs` — `tiles_url_template` 필드
- `services/etl-base-layer/src/main.rs` — CLI gold subcommand 출력 에 flat tile path
- `apps/web/components/listings/listing-map.tsx` — `addSource({type:"vector",tiles:[...]})` path
- `apps/web/proxy.ts` — `sw-pmtiles` / `__pmtiles__` / `blob:` 항목 제거
- `apps/web/.env.example` — `NEXT_PUBLIC_TILES_BASE_URL`
- `docs/adr/README.md` — 인덱스 업데이트

### 폐기
- `apps/web/lib/pmtiles-source.ts`
- `apps/web/lib/pmtiles.ts`
- `apps/web/lib/sw-register.ts`
- `apps/web/lib/workers/sw-pmtiles-src.ts`
- `apps/web/public/sw-pmtiles.js` (esbuild artifact, gitignored)
- `package.json` 의 `build:sw-pmtiles` script (만약 있다면)
- ADR 0019 의 PMTilesSource subclass + Service Worker transport (Superseded)

## 후속

### Tier A — production 위생 (포함)
- HTTP `Cache-Control: max-age=31536000, immutable` (R2 object metadata 설정)
- `Content-Encoding: gzip` (tippecanoe 출력 그대로)
- URL versioning (`gold/v<N>/parcels/{z}/{x}/{y}.pbf`) — manifest 기반 hot-swap

### Tier 1 — 자동 업데이트 (1일, T6)
- ADR 0016 의 manifest hot-swap — frontend 가 `gold/manifest.json` polling → 새 `current_version` 감지 → `addSource` 의 tiles URL template 의 prefix 업데이트
- 사용자 reload 없이 무중단 교체

### Tier 3 — 관측성 (SP7 wire, 1일)
- Cloudflare R2 의 분석 dashboard 에서 cache hit ratio / origin egress / popular tile 측정
- mapbox-gl 의 source data 이벤트 (`sourcedata`, `sourcedataloading`) 으로 client-side latency Sentry 전송

## 재검토 트리거

- mapbox-gl v2 → v3 업그레이드 시 (transformRequest 가 worker 에도 호출되도록 변경됨 — `params.data.rawData` 가 stable 해질 가능성. 단 Naver SDK 가 v2 fork lock-in 이라 사실상 무관)
- R2 storage 비용이 월 $50 초과 시 (object 수 폭증 → flat → PMTiles 단일 파일로 회귀 검토)
- Cloudflare CDN edge cache hit ratio < 80% 시 (latency 측면에서 단일 파일 byte-range 재고)

## 참고

- mapbox-gl v2 vector source: <https://docs.mapbox.com/style-spec/reference/sources/#vector>
- pmtiles-rs (Rust): <https://github.com/protomaps/PMTiles/tree/main/rust>
- Mapbox vector tile spec: <https://github.com/mapbox/vector-tile-spec>
- am2222/mapbox-pmtiles (X10 reference, 본 ADR 채택 X — X9 가 우월): <https://github.com/am2222/mapbox-pmtiles>
- T3b.x bundle 분석 + runtime probe: `var/sample/maps-gl.js`, `var/sample/naver-mb-surface.json`, `var/sample/naver-polygons.json` (gitignored)
