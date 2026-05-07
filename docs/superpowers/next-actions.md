# 다음 작업 (Next Actions)

> **갱신일**: 2026-05-07 EOD (T3b.4 finale — ADR 0021 채택, X9 = PMTiles 분해 → flat .pbf, mapbox-gl 표준 100%)
> **목적**: 다음 세션이 컨텍스트 없이도 즉시 시작 가능하도록 우선순위 + 진입점 명시.
> **SSOT**: 본 문서 = 단기 작업큐. 장기 = [`roadmap.md`](./roadmap.md). 진행 현황 = [`memory/project_progress.md`](../../memory/project_progress.md).

---

## 🆕 1순위 — SP9: 지도 base layer — **ADR 0021 (X9) 채택, ETL R2 upload + 빌드 검증 잔여**

### 현재 상태 (2026-05-07 EOD)

T1/T2/T3a/T3b.1/T3b.2/T4/T5 완료. **T3b.4 finale = ADR 0021** ([0021-static-vector-tile-decomposition.md](../adr/0021-static-vector-tile-decomposition.md)) — A2+SW spike 결과 worker uncontrolled wall, A2+Blob URL 영구 3 trick. **X9 (PMTiles 분해 → flat `{z}/{x}/{y}.pbf` 정적 호스팅)** 으로 채택 — mapbox-gl 표준 100%, trick 0, internal API 0.

### Path 5개 spike 결과 (ADR 0019 + ADR 0021 합본)

| path | 결과 | 결정 |
|---|---|---|
| C — BFF proxy `/api/tiles` | ✅ 작동 | 비표준 (Rust backend 정책 위반) |
| A2 + Blob URL | ✅ 작동 (`59e5785`) | trick 3개 영구 부채 |
| A3 — workerSourceURL + monkey-patch | ❌ wall (worker fetch closure capture) | 폐기 |
| A2 + Service Worker | ❌ wall (worker uncontrolled by SW, web spec) | 폐기 |
| **X9 — PMTiles 분해 → flat .pbf** | ✅ **mapbox-gl 표준 `type:"vector" + tiles:[URL]`** | **채택 (ADR 0021)** |
| X10 — `params.data.rawData` ArrayBuffer transfer | ✅ 작동 (am2222 패턴) | trick 1개 (internal field) — X9 가 우월 |

### ADR 0019 의 결론 reject

ADR 0019 의 *전수 검토* 가 X9 / X10 를 누락. "Naver SDK 폐기 안 하면 SSS 불가" = **틀림**. Naver SDK 안에서 X9 가 SSS 7기둥 100% 충족.

### 변경된 파일 (이번 commit 직전 working dir)

**프론트** — A2+SW spike 코드 모두 폐기, X9 wire:
- `apps/web/lib/pmtiles-source.ts` ❌ 삭제 (PMTilesSource subclass)
- `apps/web/lib/pmtiles.ts` ❌ 삭제 (registerPmtilesSourceType)
- `apps/web/lib/sw-register.ts` ❌ 삭제 (Service Worker register)
- `apps/web/lib/workers/sw-pmtiles-src.ts` ❌ 삭제 (Service Worker source)
- `apps/web/tests/e2e/pmtiles-debug.spec.ts` ❌ 삭제 (ADR 0019 spike 박제 — git history 보존)
- `apps/web/components/listings/listing-map.tsx` 🔧 `type:"vector" + tiles:[URL_TEMPLATE]` path
- `apps/web/proxy.ts` 🔧 PMTiles/SW 항목 PUBLIC_PATHS 폐기, R2 origin connectSrc 동적 추가
- `apps/web/package.json` 🔧 build:sw-pmtiles + pmtiles dep 폐기
- `apps/web/.env.local.example` 🔧 `NEXT_PUBLIC_TILES_BASE_URL` 추가

**ETL** — ADR 0021 decompose step:
- `services/etl-base-layer/src/gold/decompose.rs` ✨ 신규 (`tile-join --output-to-directory` spawn)
- `services/etl-base-layer/src/gold/build.rs` 🔧 orchestration: tippecanoe → decompose
- `services/etl-base-layer/src/gold/manifest.rs` 🔧 `tiles_url_template` + `flat_tile_count` 필드
- `services/etl-base-layer/src/gold/mod.rs` 🔧 pub mod decompose
- `services/etl-base-layer/src/main.rs` 🔧 CLI 출력 갱신
- `services/etl-base-layer/Cargo.toml` 🔧 walkdir dep 추가

**ADR**:
- `docs/adr/0021-static-vector-tile-decomposition.md` ✨ 신규 (X9 채택)
- `docs/adr/0019-pmtiles-source-via-addsourcetype.md` 🔧 Superseded marker
- `docs/adr/0020-naver-vector-interaction-model.md` 🔧 probe scope = polygon-only 명시 + symbol layer 후속
- `docs/adr/README.md` 🔧 인덱스 (0019/0020/0021)

**Probe (ADR 0020 사각지대 보강)**:
- `apps/web/tests/e2e/naver-all-features-probe.spec.ts` ✨ 신규 — symbol/line/circle/raster + multi-viewport + CadastralLayer 비교

### 검증 (이번 commit 직전)

- `cargo clippy -p etl-base-layer --all-targets -- -D warnings` ✅ 그린
- `pnpm typecheck` (apps/web) ✅ 그린
- `pnpm lint` (apps/web) ✅ 그린 (15 warnings 모두 e2e probe console.log — 의도)

### 다음 세션 진입점 (concrete)

1. **commit + push** — 본 ADR 0021 + 프론트/ETL/probe 갈아끼우기 한 commit
2. **ETL T3b.5 — R2 upload integration** (ADR 0021 § "ETL pipeline 변경"):
   - `services/etl-base-layer/src/r2_upload.rs` 가 *flat tile directory* walk + batch PutObject (concurrent 100). 현재 R2Uploader 가 PMTiles 단일 파일 PUT 만 함.
   - `Cache-Control: max-age=31536000, immutable` metadata 설정
   - `Content-Type: application/vnd.mapbox-vector-tile` (또는 `application/x-protobuf`)
   - `Content-Encoding: gzip` (tippecanoe 출력은 기본 gzip)
3. **ETL T3b.3 — V-World fetch Rust 모듈** (Node 스크립트 prototype 폐기, 자동화 완성)
4. **Local 빌드 + R2 검증** — 강남구 표본 SHP 으로 `cargo run -p etl-base-layer -- gold --layer parcels ...` → flat tile dir → R2 dev bucket upload → `curl https://r2/.../parcels/16/56500/26000.pbf` 검증
5. **manifest.json 첫 publish** — `current_version=v1`, `tiles_url_template`. 프론트 `NEXT_PUBLIC_TILES_BASE_URL=https://r2-dev/gold/v1/`
6. **e2e probe 확장 spec 돌려보기** — `pnpm exec playwright test naver-all-features-probe.spec.ts`. 결과: `var/sample/naver-all-features-{catalog,gangnam,bupyeong,seoul-station,cadastral}.json`. 결과 박제 → ADR 0020 § 후속 + (선택) 새 ADR (Naver POI runtime 활용 model)
7. **T6 — GitHub Actions cron + manifest hot-swap + Sentry**

### 발견 (사용자 needs 박제)

- **건물 식별 needs 명시** (2026-05-07) → SP9 종료 후 **FU 40 (`buildings.pmtiles`) 우선순위 escalate** 검토. V-World `LT_C_SPBD` ETL → 별도 layer.
- **probe scope 사각지대** — ADR 0020 의 polygon-only filter. 새 spec `naver-all-features-probe.spec.ts` 가 채워줌.
- **Naver `CadastralLayer`** — Naver SDK 가 별도 옵션 cadastral overlay 제공. raster 추정. 우리 PMTiles 와 비교 필요 (probe spec 에 포함).

**진입점**:

- ADR: [`0016 PMTiles 100%`](../adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) / [`0017 마커 렌더`](../adr/0017-listing-marker-render-canvas-bitmap-stamp.md) / [`0018 PNU-First`](../adr/0018-pnu-first-identity-no-coordinates.md)
- Spec: [`2026-05-06-sub-project-9`](./specs/2026-05-06-sub-project-9-medallion-base-layer-design.md)
- Plan: [`2026-05-06-sub-project-9`](./plans/2026-05-06-sub-project-9-medallion-base-layer.md)
- Reference: `C:\Users\User\Desktop\gongzzang\apps\gongzzang-design-lab\` — `scripts/pipeline/steps/build-pmtiles.ts`, `components/map/naver/UnifiedPolygonGLLayer.tsx`, `docs/PMTILES_GUIDE.md`

**Task 분해 (plan T1~T6)**:

| T | 작업 | 상태 | commit |
|---|---|---|---|
| T1 | docs commit | ✅ | `e7a3fd8` |
| T2 | listing denormalize 컬럼 마이그레이션 | ✅ | `0864270` |
| T3a | etl-base-layer crate + Bronze SHP 다운 + sha256 + manifest | ✅ | `3dcf027` |
| T3b.1 | R2 업로드 (`aws-sdk-s3`) + Bronze archive + GoldManifest skeleton | ✅ | `4302ff4` |
| T3b.2 | Gold pipeline — ogr2ogr + tippecanoe spawn (Win→WSL) + CLI gold subcommand | ✅ | `a12becd` |
| T3b.3 | **V-World fetch Rust 모듈** (Node 스크립트 prototype 폐기, 자동화 완성) | ⏳ | — |
| T3b.4 | **Frontend X9 wire** — ADR 0021 (PMTiles 분해 → flat .pbf) + lib/pmtiles*.ts 폐기 + decompose.rs 신규 + manifest 확장 | ✅ | (이번 commit) |
| T3b.5 | **R2 upload of flat tile directory** — concurrent batch PutObject + Cache-Control + Content-Type/Encoding metadata | ⏳ | — |
| T4 | parcel-lookup crate + listing 등록 hooks + 검색 필터 | ✅ | `e87d7d6` |
| T5 | 프론트 PMTiles 통합 (T3b.4 가 *재구현*) | ✅ | `ae48c54` |
| T6 | GitHub Actions cron + manifest hot-swap + Sentry 알림 + Tier 1 (자동 manifest poll) + Tier 3 (관측성 SP7) | ⏳ | — |

**T3b.4 시작 진입점 (다음 단계)**:

- ADR 0019 채택 = **`VectorTileSource` subclass + Service Worker transport**.
- 신규 파일:
  - `apps/web/lib/workers/sw-pmtiles-src.ts` (Service Worker source — pmtiles JS lib + `/__pmtiles__/` fetch handler)
  - `apps/web/public/sw-pmtiles.js` (esbuild bundle 결과, gitignore)
  - `apps/web/lib/sw-register.ts` (등록 + skipWaiting + controllerchange 대기)
  - `apps/web/lib/pmtiles-source.ts` (`createPMTilesSourceClass(mb)` factory — VectorTileSource subclass)
  - `apps/web/lib/pmtiles.ts` (`registerPmtilesSourceType(mb)` + `waitForMapbox(map)`)
- 수정:
  - `apps/web/components/listings/listing-map.tsx` — `await ensureSwActive()` → `registerPmtilesSourceType` → `addSource({type:"pmtiles"})`
  - `apps/web/proxy.ts` — `/sw-pmtiles.js` + `/__pmtiles__` PUBLIC_PATHS, `/api/tiles` 제거
- 폐기:
  - `apps/web/app/api/tiles/[...path]/route.ts` (BFF proxy, ADR 0019 폐기)
  - A2 + Blob URL 로직 (T3b.2 commit `59e5785` 의 lib/pmtiles-source.ts) → SW path 로 교체

**T3b.3 시작 진입점**:

- 기존 crate 확장: `services/etl-base-layer/`
- 신규 모듈 (제안): `gold/{shp_to_geojson.rs, tippecanoe.rs, build.rs, activate.rs}` + `verify.rs`
- CI 의존: tippecanoe binary 빌드 (`make -j` from felt/tippecanoe github), GDAL (`apt install gdal-bin`)
- 환경변수: `GOLD_VERSION`, `GOLD_PARCEL_LAYER_NAME` 등 (T3b.1 의 `Config::gold_version` field 가 이미 capture)
- R2 KEY 레이아웃 (T3b.1 README 참조): `<gold_prefix>/<version>/{parcels,admin,complex}.pmtiles` + `<gold_prefix>/manifest.json`
- 첫 30분: 로컬에서 강남구 시군구 SHP 작은 표본으로 tippecanoe 한 번 돌려보기 (CI 의 큰 빌드 디버깅 비용 회피)
- verify smoke: pmtiles-rs 또는 wrangler r2 object get 후 `tippecanoe-decode` 로 강남 PNU 1168010100107370000 존재 확인

**T6 시작 진입점**:

- `.github/workflows/sp9-base-layer-etl.yml`
- 매월 1일 03:00 KST cron + workflow_dispatch
- ubuntu-22.04-large (32GB RAM) — 1.4억 polygon 빌드용
- timeout 720분 (12시간), 정 안 되면 region 별 batch 옵션
- manifest hot-swap: `gongzzang-static/v<N+1>/` 빌드 후 `manifest.json` 의 current_version 업데이트

**T5 후속 — 프론트 폴리곤 활성화**:

- T3b.2 완료 후 `NEXT_PUBLIC_PMTILES_BASE_URL=https://r2-static/v1/` 환경변수 한 줄 설정 → 폴리곤 layer 자동 활성화 (코드 변경 0)

**핵심 architecture (ADR 0016)**:

```text
🥉 Bronze (R2): 공공데이터포털 raw SHP 12개월 archive
🥇 Gold (R2 정적, 갱신 주기 분리):
    parcels.pmtiles      geometry + PNU만        (분기)
    admin.pmtiles        행정구역                 (분기)
    complex.pmtiles      산단                    (분기)
    parcel-attrs/<sigungu>.json   jiga/gosi/land_use_type (매년)
    complex-stats.json            산단별 통계 precompute  (매일)
    listings-by-pnu/<sigungu>.json  매물 list             (매시간)
클라이언트 turf.js: viewport spatial 계산 (반경/contains)
listing 테이블 denormalize: parcel_pnu, admin_code
```

**비용**: polygon 시스템 ~$0.5/월 (DAU 무관, R2 egress 무료).

**미리 알아둘 lessons**:

- gongzzang-design-lab 의 `_mapbox` private API 추출 패턴 그대로 차용 가능 — 검증됨
- tippecanoe 빌드 시 `ubuntu-22.04-large` (32GB RAM) 필요 — 1.4억 polygon
- 좌표계: SHP 가 EPSG:5179 → tippecanoe 입력 GeoJSON 은 EPSG:4326 (ogr2ogr 변환)
- listing.parcel_pnu denormalize stale: 월 1회 재매핑 cron 으로 mitigation
- ad-hoc 분석 / temporal audit 은 의도적 미지원 — Phase 3+ 별도 ADR

**리스크 → mitigate**:

- PostGIS spatial JOIN 부재 → 클라 turf.js + 매월 precomputed JSON
- DB CHECK constraint 부재 → ETL smoke test (강남 PNU 존재 + sha256 + row count 변동 5%)
- PMTiles 빌드 12시간 timeout → GitHub Actions large runner + region 별 batch 옵션

---

## 2순위 — SP4-iii-b: data.go.kr 실거래가 + RealTransactionReader (1-2일)

**왜 1순위**: SP4-iii-a 가 만든 `DataGoKrClient` + `Policy::data_go_kr_default` + `pnu_split` + `PgRawCapture` 인프라를 *재사용*. 같은 패턴 답습이라 빠름. `RealTransaction` Aggregate 는 SP2c 에서 이미 구현 (`crates/domain/market/real-transaction`).

**진입점**:

- 도메인: [`crates/domain/market/real-transaction/src/`](../../crates/domain/market/real-transaction/src/) — Aggregate + Reader trait 이미 존재
- 신규 파일: `crates/data-clients/data-go-kr/src/real_transaction/{client.rs,parser.rs,reader.rs}` — `building_register/` 와 같은 모듈 구조
- API endpoint (참고): `data.go.kr` 부동산 실거래가 API
  - 아파트: `getRTMSDataSvcAptTrade`
  - 오피스텔: `getRTMSDataSvcOffiTrade`
  - 단독/다가구: `getRTMSDataSvcSHTrade`
  - 비주거 (산업용): `getRTMSDataSvcNrgTrade` ← **1차 타겟**

**작업 골격**:

1. spec + plan (`docs/superpowers/specs/2026-05-04-sub-project-4-iii-b-real-transaction-design.md` + plan)
2. `RealTransactionRegisterClient::fetch_by_jibun_period(parts, year_month)` — 5분해 파라미터 + `LAWD_CD` (PNU[0..5]) + `DEAL_YMD` (YYYYMM)
3. `parser::parse_real_transactions` — 응답 → `Vec<RealTransaction>` ACL
4. `DataGoKrRealTransactionReader` impl `RealTransactionReader::find_by_pnu_period` (또는 trait 이 정의한 메서드 — 코드 확인 필수)
5. `raw_capture(source = "data_go_kr_tx")` — `parcel_external_data.source` CHECK 에 이미 포함
6. wiremock 6 시나리오 (happy / multi-month / empty / 5xx / malformed / circuit)

**미리 알아둘 것 (SP4-iii-a 발견 lessons 적용)**:

- 한글 라벨 → enum 매핑은 `Other` fallback (외부 스키마 확장에 견고)
- `items.item` 단일/배열/빈 문자열 다형 처리 (`serde_json::Value` match)
- 빈 응답 분기는 V-World 등 secondary fetch 회피로 비용 절감
- `PolygonSrid` required 필드가 도메인에 있으면 V-World 합성 (FU 40 까지)
- `clippy::needless_pass_by_value` 가 헬퍼 fn 의 `Value` 인자에 자주 발동 → `&Value` 받기

---

## 3순위 — SP4-iii-c: 법제처 도시계획 텍스트 (1-2일)

**왜 다음**: `Parcel.zoning` 이 V-World 의 한글 분류만 사용 — 법제처 실제 조례/시행령 텍스트가 정확. ZoningReader port 신규.

**진입점**:

- API: 법제처 Open API (`open.law.go.kr`)
- 신규 crate: `crates/data-clients/korean-law/`
- 도메인: 신규 `ZoningRegulationReader` port — 또는 `Parcel` 의 추가 필드. ADR 필요할 수 있음 (zoning 텍스트가 Aggregate 인지 ValueObject 인지)
- raw_capture source: `"lawmaking"` (이미 CHECK 포함)

**리스크**: 법제처 응답이 HTML/XML 다중 — JSON 파서가 안 듬. 별도 파서 패턴 필요.

---

## ~~3순위 — SP4-iii-e: R2 PMTiles Reader 6 + FU 40 (2-3일)~~ → SP9 로 통합

> ⚠️ **본 항목은 SP9 (위 1순위) 가 supersede**. ADR 0016 으로 R2 PMTiles base layer 가
> 1순위로 격상 + 다중 artifact (parcels/admin/complex) 로 확장. 옛 R2 Reader 6 design
> 의 stub 은 SP9 의 ETL/PMTiles 빌드 input 으로 흡수. FU 40 (`Building.geom` 정확
> footprint) 는 SP9 종료 후 별도 sub-project (`LT_C_SPBD` 또는 PMTiles building 레이어).

**왜 통합됐는지**: 1차 SP4-iii-e design (commit `9d8a513`) 은 PMTiles fetch 를 *Reader port* 로 추상화하려 했음 → ADR 0014 가 보류 → ADR 0016 이 *PMTiles 정적 호스팅 + 클라 직접 fetch* 패턴으로 재설계.

**진입점**:

- 신규 crate: `crates/data-clients/r2-public-data/`
- ETL 파이프라인 분리: `services/etl-pmtiles-builder` 가 V-World/data.go.kr → PMTiles 빌드 후 R2 upload (별도 서비스)
- 6 Reader: `Parcel::fetch_markers_in_bbox` (현재 honest failure), `Building::fetch_by_id` (FU 42 도 같이), `IndustrialComplex`, `Manufacturer`, `RealTransaction::fetch_markers_in_bbox`, `CourtAuction::fetch_markers_in_bbox`
- FU 40: `Building.geom` 을 V-World `AL_D194_*` (건물 footprint) 또는 PMTiles 에서 가져옴. SP4-iii-a 의 합성 코드 (`reader.rs::fetch_polygon`) 가 polymorphic 으로 분기하도록 변경

**리스크**: PMTiles 파서 (`pmtiles-rs` crate) 가 alpha. 검증 필요. 정적 빌드 경로 결정 필요.

---

## 4순위 — Production 잔여 부채 일괄 정리 (FU 미해소 9건+)

[`roadmap.md` § Spec FU 누적](./roadmap.md) 참조.

특히 production 직전 필수:

- **FU 4 / 6**: BusinessNumber NTS 체크섬 외부 검증 + 사업자유형 코드
- **FU 8**: KsicCode 대분류 letter 강제
- **FU 13**: AuditLog spec § 4.3 ↔ 실제 schema 정렬
- **FU 14**: BVQ/LRQ entity `updated_at` ↔ DB 컬럼 미존재 정합
- **FU 18**: AuthCrate clippy 빚 — `crates/auth/src/verifier.rs` panic + manual_let_else (SP3 잔재)
- **FU 26**: `clippy::disallowed_types` 로 reqwest::Client 직접 호출 차단

---

## 그 다음 단계 (SP4-iii 완전 종료 후)

| SP | 영역 | 추정 |
|---|---|---|
| **SP6** | Frontend (Next.js + React 19, 4-7일) — SP6-i 인증 / SP6-ii 매물 검색 / SP6-iii 북마크 / SP6-iv 알림 | 분해 필요 |
| **SP7** | 관측성 (Grafana + Prometheus + Loki + Tempo + Sentry) — Outbox publisher metrics + Breaker open alert | 2-3일 |
| **SP8** | IaC (Pulumi RDS / R2 / ECS / ALB) | 3-4일 |
| **SP9-12** | 데이터 파이프라인 / AI 어시스턴트 / 검색 / 결제 | TBD |

---

## 환경 체크 (다음 세션 시작 전)

- `cargo --version` → 1.88.0 가 path 에 있는지 (`$env:USERPROFILE\.cargo\bin`)
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린 (SP4-iii-a 종료 시점 검증됨)
- `git log --oneline -5` 마지막 commit `2aaf7d9` 확인
- push 권한: `git push origin main` 이 sandbox policy 로 차단될 수 있음 — 사용자 승인 필요
- markdownlint pre-commit hook 활성 — `+`/`*` 로 시작하는 indented 텍스트 금지 (MD004)

---

## SP4-iii-a 가 발견한 명시적 follow-up

| FU | 내용 | 우선순위 |
|---|---|---|
| 40 | `Building.geom` 정확한 footprint (V-World AL_D194 또는 R2 PMTiles) | SP4-iii-e 와 묶음 |
| 41 | `mainPurpsCdNm` / `strctCdNm` 한글 매핑표 28+ 케이스 확장 | low (Other fallback 작동 중) |
| 42 | `BuildingReader::fetch_by_id` (mgmBldrgstPk endpoint) | medium |
| 43 | 캐시 정책 (`expires_at = fetched_at + 30 days`) | medium (SP4-iii 종료 후) |
| 44 | 토지대장 endpoint | SP4-iii-b 와 묶음 검토 |
