# SP9: 지도 Base Layer — PMTiles 100% 구현 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 spec | [2026-05-06-sub-project-9-medallion-base-layer-design.md](../specs/2026-05-06-sub-project-9-medallion-base-layer-design.md) |
| 선행 ADR | [0016](../../adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (PMTiles 100%) |
| 추정 | 6 task, 1주 |

> 1차 plan 은 PostGIS Silver 포함 8 task 2-3주. ADR 0016 변경 (PMTiles 100%) 으로
> Silver/PostGIS 작업 제거 + ETL 단순화 → 6 task 1주.

---

## T1 — ADR 0016 + spec/plan 커밋

이미 작성됨 (본 plan + ADR 0016 + spec). T1 = 한 commit.

**commit**: `docs(sp9): ADR 0016 + spec + plan — PMTiles 100% (Bronze + Gold) base layer`

---

## T2 — listing denormalize 컬럼 마이그레이션

**대상**: `db/migration/V<N>__sp9_listing_polygon_denormalize.sql` (신규)

```sql
-- listing 에 polygon 정보 denormalize (PostGIS polygon 테이블 안 만듦)
ALTER TABLE listing
    ADD COLUMN parcel_pnu CHAR(19),
    ADD COLUMN admin_code VARCHAR(10),
    ADD COLUMN parcel_land_use_type VARCHAR(20),
    ADD COLUMN parcel_zoning VARCHAR(20),
    ADD COLUMN parcel_lookup_at TIMESTAMPTZ;

CREATE INDEX listing_parcel_pnu_idx ON listing(parcel_pnu);
CREATE INDEX listing_admin_code_idx ON listing(admin_code);
CREATE INDEX listing_land_use_type_idx ON listing(parcel_land_use_type);

-- CHECK constraint: PNU format
ALTER TABLE listing
    ADD CONSTRAINT listing_parcel_pnu_format
    CHECK (parcel_pnu IS NULL OR parcel_pnu ~ '^[0-9]{19}$');
```

`parcel_lookup_at` — 마지막 lookup 시점. polygon 갱신 cron 이 stale 검출.

**검증**: 마이그레이션 적용 + sqlx migrate run 통과.

**commit**: `feat(sp9-t2): listing denormalize columns (parcel_pnu, admin_code, etc.) for polygon-aware search`

---

## T3 — ETL Rust binary (Bronze 다운 + tippecanoe + R2 업로드)

> **분해 (2026-05-07 갱신)**: T3 가 무거워 5 task 로 분해됨. 진행 현황:
>
> | | 내용 | 상태 | commit |
> |---|---|---|---|
> | T3a | etl-base-layer crate + Bronze SHP 다운 + sha256 + manifest | ✅ | `3dcf027` |
> | T3b.1 | R2 업로드 모듈 (`aws-sdk-s3`) + Bronze archive PUT + GoldManifest skeleton | ✅ | `4302ff4` |
> | T3b.2 | Gold pipeline — ogr2ogr + tippecanoe spawn (Win→WSL auto) + CLI gold subcommand | ✅ | `a12becd` |
> | T3b.3 | V-World fetch Rust 모듈 (Node 스크립트 prototype 폐기) | ⏳ | - |
> | T3b.4 | Frontend PMTiles 통합 — ADR 0019 (PMTilesSource subclass + Service Worker) | ⏳ | - |
>
> T3b.4 우선 (사용자 폴리곤 시각 확인). T3b.3 (자동화) 는 V-World API 복구 시점에 진행.

**대상**: `services/etl-base-layer/` (신규 crate)

```
services/etl-base-layer/
├── Cargo.toml
└── src/
    ├── main.rs           # CLI entry
    ├── config.rs         # 환경변수
    ├── bronze/
    │   ├── shp_download.rs   # 공공데이터포털 SHP fetch + R2 archive
    │   ├── vworld_fetch.rs   # 산업단지 GeoJSON
    │   └── manifest.rs       # bronze manifest 생성
    ├── shp_to_geojson.rs # SHP → GeoJSON stream + 좌표계 변환 (ogr2ogr spawn)
    ├── tippecanoe.rs     # binary spawn + zoom config
    ├── r2_upload.rs      # Gold artifact + manifest 업로드
    └── verify/
        └── smoke.rs      # PNU 존재, sha256, row count 변동 검증
```

핵심 기능:
- 매월 1회 실행 (cron 또는 수동)
- Bronze SHP/GeoJSON 다운로드 → R2 archive (sha256)
- SHP → GeoJSON 변환 (ogr2ogr 또는 shapefile crate + proj)
- tippecanoe spawn (필지 / 행정 / 산단 3종)
- Gold artifact + manifest R2 업로드
- 신규 version prefix 활성화

**의존성** (CI runner):
- tippecanoe binary (`apt install` 또는 build from source)
- ogr2ogr (GDAL — `apt install gdal-bin`)
- Rust 1.88

**테스트**:
- 단위 테스트 (mock R2 + 작은 SHP 일부 — 강남구만)
- smoke test (실 SHP 일부 → 강남 PNU 존재 확인)

**commit**: `feat(sp9-t3): etl-base-layer — bronze fetch + tippecanoe + R2 upload + manifest`

---

## T4 — parcel_lookup 백엔드 service (listing 등록 hooks)

**대상**: `crates/parcel-lookup/` (신규) + `services/api/src/routes/listings.rs` (수정)

`parcel-lookup` crate:
- 1차: V-World API 직접 호출 (`fetch_by_point` — V-World POINT spatial filter, 이미 검증됨)
- 2차: 백엔드 메모리 R-tree (Bronze SHP 시작 시 로드, 메모리 ~500MB) — V-World quota 부담 시
- 책임: `(lng, lat) → ParcelInfo { pnu, admin_code, land_use_type, zoning, gosi_year_month }`

`listings.rs` 수정:
```rust
async fn create_listing(req: ListingCreate) -> Result<Listing> {
    let parcel = parcel_lookup::at_point(req.lng, req.lat).await?;
    sqlx::query!(
        "INSERT INTO listing (..., parcel_pnu, admin_code, ...) VALUES (..., $1, $2, ...)",
        parcel.pnu.as_str(), parcel.admin_code, ...
    ).execute(&pool).await?;
}

async fn search_listings(q: ListingsQuery) -> Result<Vec<Listing>> {
    sqlx::query!(
        "SELECT * FROM listing WHERE
           ($1::text IS NULL OR parcel_pnu = $1)
           AND ($2::text IS NULL OR admin_code = $2)
           AND ($3::text IS NULL OR parcel_land_use_type = $3)
           AND ...",
        q.pnu, q.admin_code, q.land_use_type
    ).fetch_all(&pool).await?
}
```

**테스트**:
- parcel_lookup unit test (V-World wiremock fixture 재사용 — 이미 ADR 0015 에서 작성됨)
- listing create integration test (lookup 호출 후 DB 행에 컬럼 채워짐 확인)
- search test (필터 조합 5+ 케이스)

**commit**: `feat(sp9-t4): parcel-lookup crate + listing create hooks + filtered search`

---

## T5 — 프론트 PMTiles 통합

**대상**: `apps/web/components/listings/listing-map.tsx` (수정) + 신규 hook 5개 + `parcel-info-card.tsx`

작업:
1. `pmtiles` JS lib 설치 (pnpm)
2. `loadNaverMaps` 후 `_mapbox` 인스턴스에 `addProtocol('pmtiles', ...)` 등록
3. Source/Layer 등록 hook (`usePolygonSources`)
4. 클라이언트 필터 hook (`usePolygonFilters` — admin_code/land_use_type/area/jiga 검색바와 동기)
5. 클릭 핸들러 hook (`useParcelClick` — `properties.pnu` → `/listings?pnu=...` fetch + panel 열기)
6. `<ParcelInfoCard>` 컴포넌트 — 필지 정보 + 매물 list

**Reference**: gongzzang-design-lab 의 hook 패턴 참고 (
[`usePolygonSources.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\hooks\usePolygonSources.ts),
[`useMapboxGLInit.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\hooks\useMapboxGLInit.ts)).

**테스트**:
- Vitest 컴포넌트 test (mock map)
- Playwright E2E — 줌 16+ 진입 시 polygon 보임 확인 + 클릭 → panel 열림

**commit**: `feat(sp9-t5): map PMTiles integration — polygon layers + client filter + click panel`

---

## T6 — GitHub Actions cron + manifest hot-swap + Sentry 알림

**대상**: `.github/workflows/sp9-base-layer-etl.yml` (신규)

```yaml
on:
  schedule:
    - cron: '0 18 1 * *'  # 매월 1일 03:00 KST = 18:00 UTC 전월 마지막날
  workflow_dispatch: {}

jobs:
  etl:
    runs-on: ubuntu-22.04-large  # 디스크 100GB+ 필요
    timeout-minutes: 720  # 12시간
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust + GDAL + tippecanoe
        run: |
          sudo apt-get update
          sudo apt-get install -y gdal-bin libsqlite3-dev
          git clone https://github.com/felt/tippecanoe && cd tippecanoe && make -j && sudo make install
      - name: Run ETL
        env:
          R2_ACCOUNT_ID: ${{ secrets.R2_ACCOUNT_ID }}
          R2_ACCESS_KEY: ${{ secrets.R2_ACCESS_KEY }}
          R2_SECRET_KEY: ${{ secrets.R2_SECRET_KEY }}
          VWORLD_API_KEY: ${{ secrets.VWORLD_API_KEY }}
          VWORLD_DOMAIN: ${{ secrets.VWORLD_DOMAIN }}
          DATA_GO_KR_KEY: ${{ secrets.DATA_GO_KR_KEY }}
          SENTRY_DSN: ${{ secrets.SENTRY_DSN }}
        run: cargo run --release -p etl-base-layer
      - name: Activate new version (R2 manifest update)
        if: success()
        run: cargo run --release -p etl-base-layer -- activate
      - name: Sentry on failure
        if: failure()
        run: curl -X POST $SENTRY_INGEST -d '{"message":"sp9 etl failed","level":"error"}'
```

manifest hot-swap:
- 빌드 완료 후 `gongzzang-static/v<N+1>/` 에 새 artifacts
- 검증 통과 후 `gongzzang-static/manifest.json` 의 `current_version` = `v<N+1>`
- 클라이언트는 manifest 조회 → URL 결정
- 실패 시 manifest 변경 안 함 → 클라가 이전 v_N 그대로 fetch

**commit**: `feat(sp9-t6): github actions cron — monthly ETL + version hot-swap + sentry alert`

---

## 검증 (전체 통과 기준)

- T2-T6 각 commit 후 `cargo clippy --all -- -D warnings` 통과
- `cargo test --workspace` 통과
- 실 SHP 일부 (강남구만) 로 ETL 통과 → PMTiles 빌드 → 강남 PNU 존재 확인
- 프론트 E2E (Playwright) — 줌 16 진입 + 클릭 → panel
- 한 번 운영키/공공데이터포털 키 발급 후 nightly cron 1회 성공 확인

---

## 위험 요소

| 위험 | 완화 |
|---|---|
| tippecanoe 빌드 시 메모리 폭증 (1.4억 polygon) | `--no-tile-size-limit` + GitHub Actions `ubuntu-22.04-large` (32GB RAM) + 정 안 되면 self-hosted runner |
| ETL 12시간 timeout 초과 | 720분 timeout. 정 안 되면 region별 batch (시도별 분할 빌드) |
| 좌표계 변환 정확도 | ogr2ogr 사용 (검증된 GDAL path). spot check 으로 강남 PNU 좌표 비교 |
| Naver Maps gl 의 `_mapbox` private API 변경 | 1차 = `_mapbox` 직접. 깨지면 MapLibre 로 base map 교체 (별도 spec) |
| V-World quota — listing 등록 시 lookup 폭증 | parcel-lookup crate 가 V-World 호출 + Redis cache 1주 TTL → 같은 좌표 재조회 시 cache hit |
| listing parcel_pnu stale (polygon 갱신 후) | 월간 ETL 후 listing 재매핑 cron — 1% 미만 변동이라 부담 적음 |

---

## 일정 추정 (PMTiles 100% 단순화)

- T1 (커밋만): 0.5일
- T2 (마이그레이션): 0.5일
- T3 (ETL crate): 2일
- T4 (parcel-lookup + listing hooks): 1.5일
- T5 (프론트 통합): 2일
- T6 (cron): 0.5일

**총 7일** = 1주.

(이전 plan 의 PostGIS Silver 작업 제거로 50% 단축)

---

## 다음

본 plan 검토 후 T1 (커밋) → T2 (마이그레이션) 부터 순차. T3-T5 는 의존 (T3 결과물 = T5 입력).
