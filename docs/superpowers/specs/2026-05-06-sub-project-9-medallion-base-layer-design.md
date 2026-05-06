# SP9: 지도 Base Layer — PMTiles 100% 설계

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 결정 ADR | [0016](../../adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (PMTiles 100% 채택) / [0014](../../adr/0014-base-layer-defer-pmtiles.md) supersede |
| 목적 | 전국 필지 (1.4억) + 행정구역 (시도/시군구/읍면동/리) polygon 을 지도에 표현 |
| 추정 | 5 task, 1주 |

> **변경 이력**: 1차 Draft 는 Medallion (Bronze/Silver/Gold + PostGIS) 채택. 비용/yagni
> 재검토로 **PMTiles 100% (Bronze raw + Gold PMTiles, PostGIS 미도입)** 로 재설계.
> PostGIS polygon 테이블은 Phase 3+ 분석 needs 발생 시 별도 ADR/SP 로 추가.

## 1. 목표

사용자가 매물 검색 지도에서:
1. 줌 0~15 → 행정구역 경계 polygon 자동 표시 (시도→시군구→읍면동→리)
2. 줌 16+ → 필지 polygon + 산업단지 polygon 표시
3. 클라이언트 필터 (지목/면적/공시지가/행정구역) — mapbox-gl filter expression
4. 클릭 → PNU/행정구역 코드 → `/listings?pnu=...` 또는 `/admin/{code}/listings` API → 매물 panel
5. 매물 마커는 기존대로 위에 표시 (호환)

## 2. 비목표 (Phase 3+로 미룸)

- 분석 dashboard ("산단별 빈 공장용지 통계" 등) — 별도 ADR
- 실시간 polygon 색상 변동 (매물 등록에 따라) — 별도 ADR
- 건물 footprint 표시 — FU 40 별도
- PostGIS polygon 테이블

## 3. 아키텍처 — Bronze + Gold 다중 artifact (갱신 주기 분리)

### 핵심 원칙

**갱신 주기가 다른 데이터는 같은 파일에 묶지 않음**:
- polygon **모양** (geometry) — 분기/년 (지적도 거의 안 변함)
- polygon **속성** (jiga, gosi, land_use_type) — 매년 (공시지가 고시)
- **통계 aggregates** — 매일/매시간 (precompute)
- **매물** — 실시간 (PostgreSQL)

→ PMTiles 에 모든 걸 넣지 않음. Geometry 만 PMTiles, attributes/stats/listings 는 별도 R2 JSON.

```
[공공데이터포털 SHP]   [V-World 산업단지 WFS — 분기 batch]
        │                       │
        ▼ 월 1회 GitHub Actions cron
┌─────────────────────────────────────────┐
│ 🥉 Bronze (R2)                           │
│   gongzzang-bronze/<YYYY-MM>/            │
│     ├── parcel.shp.zip                   │
│     ├── admin.shp.zip                    │
│     └── industrial-complex.geojson       │
│   12개월 archive (감사 / 재현)            │
│   sha256 checksum 기록                   │
└──────────────┬──────────────────────────┘
               ▼ ETL Rust binary
               │ (shapefile crate + tippecanoe spawn)
┌─────────────────────────────────────────┐
│ 🥇 Gold (R2 정적)                        │
│   gongzzang-static/v<N>/                 │
│     ├── parcels.pmtiles                  │
│     ├── admin.pmtiles                    │
│     ├── industrial-complex.pmtiles       │
│     └── manifest.json                    │
│   pmtiles 별 layer minzoom/maxzoom 정의   │
└──────────────┬──────────────────────────┘
               ▼ HTTP Range (Cloudflare CDN cache)
┌─────────────────────────────────────────┐
│ 프론트 — Naver Maps gl + pmtiles JS      │
│   addProtocol('pmtiles', ...)            │
│   addSource + addLayer per 종류          │
│   feature.properties 클라이언트 필터     │
│   클릭 → properties.pnu → API           │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 백엔드 — listing 검색 (기존 RDS)          │
│   listing.parcel_pnu (denormalize 컬럼)  │
│   listing.admin_code                     │
│   listing.land_use_type                  │
│   GET /listings?pnu=&admin_code=&...     │
└─────────────────────────────────────────┘
```

PostGIS polygon 테이블 **없음**. Phase 3+ 분석 needs 시 별도 도입.

## 4. Bronze — raw SHP 보존

### 4.1 R2 bucket 구조

```
gongzzang-bronze/
├── 2026-05/
│   ├── parcel.shp.zip         (~500 MB-1 GB)
│   ├── admin.shp.zip          (~50 MB)
│   ├── industrial-complex.geojson (~10 MB, V-World 응답 그대로)
│   └── manifest.json          { downloaded_at, sha256, source_urls }
├── 2026-06/
└── ...
```

12개월 보관 후 cold archive (R2 archive class 또는 삭제 정책). 감사 요구 시 복원.

### 4.2 다운로드 정책

- 공공데이터포털 SHP: 월 1회 GitHub Actions cron
- V-World 산업단지: 분기 1회 (응답 캡처 = 기존 `parcel_external_data` 패턴)
- sha256 checksum 기록 → 외부 데이터 변경 감지

## 5. Gold — 다중 R2 artifact

### 5.0 Artifact 카테고리 정리

| 카테고리 | 파일 | 갱신 주기 | 크기 | 캐시 정책 |
|---|---|---|---|---|
| Geometry | `v1/parcels.pmtiles` | 분기 1회 | ~10 GB | 1년 (immutable, version prefix) |
| Geometry | `v1/admin.pmtiles` | 분기 1회 | ~50 MB | 1년 |
| Geometry | `v1/complex.pmtiles` | 분기 1회 | ~5 MB | 1년 |
| Attributes | `v1/parcel-attrs/<sigungu>.json` | 매년 (공시지가 고시) | ~50 MB / 시군구 | 1개월 |
| Attributes | `v1/admin-attrs.json` | 분기 | ~5 MB | 1개월 |
| Aggregates | `v1/complex-stats.json` | 매일 cron | ~1 MB | 1시간 |
| Aggregates | `v1/density-by-sigungu.json` | 매일 cron | ~1 MB | 1시간 |
| Aggregates | `v1/zoning-jiga-summary.json` | 매년 (공시지가 갱신 시) | ~500 KB | 1개월 |
| Listings | `v1/listings-by-pnu/<sigungu>.json` | 매시간 cron | ~10 MB / 시군구 | 5분 |
| Listings | `v1/listings-by-complex.json` | 매시간 cron | ~1 MB | 5분 |
| Manifest | `v1/manifest.json` | 매 빌드 | <10 KB | 1분 |

각 artifact 별도 갱신 → 변경 없는 데이터는 CDN 영구 캐시 (PMTiles 가 매월 rebuild 되지 않음).

### 5.1 빌드 도구

[tippecanoe](https://github.com/felt/tippecanoe) — Felt 의 fork (active maintenance)

```bash
# 필지 (1.4억)
tippecanoe -o parcels.pmtiles \
  --minimum-zoom=16 --maximum-zoom=18 \
  --layer=parcels \
  --include=pnu,jibun,addr,jiga,gosi_year,gosi_month,land_use_type,area_m2,sido_code,sigungu_code,eupmyeondong_code \
  --no-feature-limit --no-tile-size-limit \
  parcels.geojson

# 행정구역 (시도/시군구/읍면동/리)
tippecanoe -o admin.pmtiles \
  --minimum-zoom=0 --maximum-zoom=15 \
  --coalesce-densest-as-needed \
  --layer-by-zoom-level \
  --layer=admin \
  --include=code,level,name,parent_code \
  admin.geojson

# 산업단지
tippecanoe -o industrial-complex.pmtiles \
  --minimum-zoom=8 --maximum-zoom=18 \
  --layer=complex \
  --include=code,name,type \
  industrial-complex.geojson
```

### 5.2 줌 레벨 LOD

| Zoom | 표시 | PMTiles |
|---|---|---|
| 0-7 | 시도 (17) | admin.pmtiles minzoom 0 |
| 8-10 | 시군구 (~250) + 산단 | admin minzoom 8 + complex minzoom 8 |
| 11-13 | 읍면동 (~3,500) | admin minzoom 11 |
| 14-15 | 리 (~17,000) | admin minzoom 14 |
| 16+ | 필지 (1.4억) + 산단 | parcels minzoom 16 + complex |

zoom < 16 에서 필지 안 그림. gongzzang-design-lab 동일 정책.

### 5.3 R2 호스팅

- bucket: `gongzzang-static`
- key prefix: `v1/` (versioning — 신규 빌드 시 `v2/` 로 hot-swap)
- public URL: `https://static.gongzzang.com/v1/parcels.pmtiles`
- TTL: 30일 (월 갱신과 동기)
- Cloudflare CDN cache 자동

### 5.4 Build manifest

각 Gold 빌드 후 `manifest.json` R2 업로드:

```json
{
  "version": "v1",
  "build_id": "20260501-abc123",
  "bronze_versions": {
    "parcel": "2026-05/parcel.shp.zip@sha256:...",
    "admin": "2026-05/admin.shp.zip@sha256:...",
    "industrial-complex": "2026-05/industrial-complex.geojson@sha256:..."
  },
  "built_at": "2026-05-01T03:14:25Z",
  "artifacts": {
    "parcels.pmtiles":           { "size": 9_421_337_000, "sha256": "..." },
    "admin.pmtiles":             { "size":   142_337_000, "sha256": "..." },
    "industrial-complex.pmtiles":{ "size":     5_337_000, "sha256": "..." }
  },
  "row_counts": {
    "parcel": 142_345_678,
    "admin_division": 20_834,
    "industrial_complex": 1_247
  }
}
```

클라이언트가 manifest 조회 → stale 검출 가능.

## 6. ETL — Rust binary (`services/etl-base-layer/`)

```
services/etl-base-layer/
├── Cargo.toml
└── src/
    ├── main.rs           # CLI entry (수동/cron 양쪽)
    ├── config.rs         # 환경변수 (R2 creds, V-World key)
    ├── bronze/
    │   ├── shp_download.rs   # 공공데이터포털 SHP 다운로드 + R2 업로드
    │   ├── vworld_fetch.rs   # V-World 산업단지 BBOX WFS → GeoJSON
    │   └── manifest.rs       # bronze manifest 생성
    ├── shp_to_geojson.rs # SHP → GeoJSON stream (tippecanoe input)
    ├── srid.rs           # EPSG:5179 → EPSG:4326 변환 (proj 또는 ogr2ogr 위임)
    ├── tippecanoe.rs     # binary spawn + 빌드 옵션
    ├── r2_upload.rs      # Gold artifact + manifest R2 업로드
    └── verify/
        └── smoke.rs      # 빌드 결과 검증
```

### 6.1 ETL 흐름

```rust
// 의사코드
async fn etl_polygon_layers() -> Result<EtlReport> {
    // 1. Bronze 다운로드 + R2 archive
    let parcel_shp = bronze::shp_download::fetch_parcel_shp().await?;
    let admin_shp = bronze::shp_download::fetch_admin_shp().await?;
    let complex_geojson = bronze::vworld_fetch::fetch_industrial_complex().await?;
    let bronze_manifest = bronze::manifest::write(&[parcel_shp, admin_shp, complex_geojson]).await?;

    // 2. SHP → GeoJSON stream (좌표계 변환 포함)
    let parcel_geojson = shp_to_geojson::convert(parcel_shp.path()).await?;
    let admin_geojson = shp_to_geojson::convert(admin_shp.path()).await?;

    // 3. tippecanoe 빌드
    tippecanoe::build("parcels.pmtiles", &parcel_geojson, ZoomConfig::parcels()).await?;
    tippecanoe::build("admin.pmtiles", &admin_geojson, ZoomConfig::admin()).await?;
    tippecanoe::build("industrial-complex.pmtiles", &complex_geojson, ZoomConfig::complex()).await?;

    // 4. 검증
    verify::smoke::known_pnu_in_pmtiles("1168010100107370000", "parcels.pmtiles").await?;
    verify::smoke::row_count_within(&bronze_manifest, &gold_artifacts, 0.05).await?; // 5% 이내 변동

    // 5. R2 업로드 (새 v_N prefix → manifest 갱신 → public URL 활성화)
    let new_version = next_version();
    r2_upload::upload_artifacts(new_version, &artifacts).await?;
    r2_upload::upload_manifest(new_version, &gold_manifest).await?;
    r2_upload::activate_version(new_version).await?;

    Ok(EtlReport { ... })
}
```

### 6.2 SHP 라이브러리

- [`shapefile`](https://crates.io/crates/shapefile) — 순수 Rust SHP reader
- 좌표계 변환: 1차 = `ogr2ogr` (GDAL CLI) spawn — 검증된 path. 2차 = `proj` Rust binding 검토
- 또는 PostGIS 없이도 가능: SHP의 `.prj` 읽고 proj4 string → `proj` crate 변환

### 6.3 운영

- GitHub Actions: 매월 1일 03:00 KST cron
- 실패 → Sentry 알림 + Gold version 갱신 안 함 (이전 v_N 유지)
- 성공 → Slack 알림 + 신규 version 활성화
- 수동 트리거 가능 (`workflow_dispatch`)

## 7. 프론트 통합 — Naver Maps gl + PMTiles

[listing-map.tsx](../../../apps/web/components/listings/listing-map.tsx) 의 `_mapbox` 인스턴스 활용:

### 7.1 PMTiles protocol 등록

```typescript
import { PMTiles, Protocol } from 'pmtiles';
const protocol = new Protocol();
mapboxgl.addProtocol('pmtiles', protocol.tile);  // 또는 maplibregl
```

### 7.2 Source + Layer 등록

```typescript
const PMTILES_BASE = 'pmtiles://https://static.gongzzang.com/v1';

mb.addSource('admin', { type: 'vector', url: `${PMTILES_BASE}/admin.pmtiles` });
mb.addSource('parcels', { type: 'vector', url: `${PMTILES_BASE}/parcels.pmtiles`, promoteId: 'pnu' });
mb.addSource('complex', { type: 'vector', url: `${PMTILES_BASE}/industrial-complex.pmtiles` });

// 행정구역 outline (모든 zoom)
mb.addLayer({
    id: 'admin-line', source: 'admin', 'source-layer': 'admin',
    type: 'line',
    paint: {
        'line-color': '#636e72',
        'line-width': ['interpolate', ['linear'], ['zoom'], 8, 0.5, 16, 2],
    },
});

// 필지 fill (zoom 16+)
mb.addLayer({
    id: 'parcel-fill', source: 'parcels', 'source-layer': 'parcels',
    type: 'fill', minzoom: 16,
    paint: {
        'fill-color': ['match', ['get', 'land_use_type'],
            'FactorySite', '#ff6b6b',
            'WarehouseSite', '#feca57',
            '#dfe6e9'  // default
        ],
        'fill-opacity': 0.3,
        'fill-outline-color': '#2d3436',
    },
});

// 산업단지
mb.addLayer({
    id: 'complex-fill', source: 'complex', 'source-layer': 'complex',
    type: 'fill', minzoom: 8,
    paint: {
        'fill-color': '#0984e3',
        'fill-opacity': 0.2,
    },
});
```

### 7.3 클라이언트 필터링

```typescript
// 검색바에서 사용자가 "공장용지 + 1000m² 이상 + 역삼동" 필터
mb.setFilter('parcel-fill', [
    'all',
    ['==', ['get', 'land_use_type'], 'FactorySite'],
    ['>=', ['to-number', ['get', 'area_m2']], 1000],
    ['==', ['get', 'eupmyeondong_code'], '1168010100'],
]);
```

백엔드 호출 0회. 즉시 반응.

### 7.4 클릭 핸들러

```typescript
mb.on('click', 'parcel-fill', async (e) => {
    const props = e.features[0].properties;
    setSelectedParcel({
        pnu: props.pnu,
        jibun_address: props.addr,
        jiga: props.jiga,
        land_use_type: props.land_use_type,
    });
    // 매물 fetch (denormalize 컬럼 활용)
    const listings = await fetch(`/listings?pnu=${props.pnu}`).then(r => r.json());
    setParcelListings(listings);
});
```

## 8. listing 검색 — denormalize 컬럼

### 8.1 마이그레이션 (필요 시)

```sql
-- 기존 listing 테이블에 컬럼 추가 (이미 있으면 skip)
ALTER TABLE listing
    ADD COLUMN parcel_pnu CHAR(19),
    ADD COLUMN admin_code VARCHAR(10),
    ADD COLUMN parcel_land_use_type VARCHAR(20),
    ADD COLUMN parcel_zoning VARCHAR(20);

CREATE INDEX listing_parcel_pnu_idx ON listing(parcel_pnu);
CREATE INDEX listing_admin_code_idx ON listing(admin_code);
CREATE INDEX listing_land_use_type_idx ON listing(parcel_land_use_type);
```

### 8.2 등록 시 lookup

```rust
// services/api/src/routes/listings.rs (등록 핸들러)
async fn create_listing(req: ListingCreate) -> Result<Listing> {
    // 좌표 → PNU lookup (1회)
    let parcel_info = parcel_lookup::at_point(req.lng, req.lat).await?;

    sqlx::query!(
        r#"INSERT INTO listing (
            ..., parcel_pnu, admin_code, parcel_land_use_type, parcel_zoning, ...
        ) VALUES (..., $1, $2, $3, $4, ...)"#,
        parcel_info.pnu.as_str(),
        parcel_info.admin_code,
        parcel_info.land_use_type as _,
        parcel_info.zoning as _,
    ).execute(&pool).await?;
}
```

`parcel_lookup::at_point` 1차 구현:
- V-World API 직접 호출 (`fetch_by_point`) — 등록은 자주 안 일어나니 quota 부담 적음
- 또는 백엔드 메모리 R-tree (Bronze SHP 시작 시 로드)

### 8.3 검색 endpoint

```
GET /listings?pnu=1168010100107370000        → 특정 필지 매물
GET /listings?admin_code=1168010100          → 역삼동 매물
GET /listings?land_use_type=FactorySite&...  → 공장용지 매물
```

기존 listing 테이블 쿼리 — polygon 테이블 join 없음.

### 8.4 polygon 갱신 시 listing 재매핑 cron

```
매월 cron (PMTiles rebuild 후):
  - 모든 listing 의 좌표를 새 PMTiles 또는 SHP 로 재 lookup
  - parcel_pnu / admin_code 가 변경된 행 UPDATE
  - 변경 < 0.1% 라 부담 적음 (대부분 폴리곤 ID 동일)
```

## 9. SSS 7 기둥 매핑

| 기둥 | SP9 적용 |
|---|---|
| 일관성 | Bronze → Gold deterministic ETL |
| 자동강제 | smoke test (PNU 존재, sha256, row count 변동), Sentry, version hot-swap |
| 추적성 | Bronze 12개월 archive, build manifest, version prefix history |
| 안전성 | 정적 PMTiles (런타임 에러 0), 등록 시 PNU lookup 검증 |
| 가시성 | manifest 비교, GitHub Actions UI |
| SSOT | Bronze = 외부 진실. Gold = derived. PostGIS 사본 안 만듦 |
| 명확성 | ADR 0016 + 본 spec |

## 10. 트레이드오프 인정

- **분석 dashboard 어려움** — Phase 3+ 에서 PostGIS polygon 테이블 점진 도입 (별도 ADR)
- **실시간 polygon 색상 변동 불가** — PMTiles 정적이라 매물 등록 즉시 반영 안 됨. 클릭 시 panel 에서 보이면 충분
- **listing parcel_pnu denormalize stale 가능성** — 월 1회 재매핑 cron 으로 mitigate. 1% 미만 변동
- **PostGIS GIST/JOIN 분석 power 포기** — Phase 1-2 에선 needs 없음

## 11. 인프라 비용

| 자원 | 크기 | R2 비용/월 |
|---|---|---|
| Bronze (12개월 archive) | ~6-12 GB | $0.10 |
| Gold (현재 + 1 backup version) | ~20 GB | $0.30 |
| Class B reads (사용자 트래픽) | (Range request) | < $0.10 |
| **합계 polygon 시스템** | | **~$0.50/월** |

DAU 무관. Cloudflare R2 egress 무료.

## 12. 후속 (Phase 3+ 시 별도 ADR)

- PostGIS polygon 테이블 + spatial JOIN — 분석 dashboard needs 발생 시
- 백엔드 동적 MVT endpoint — 실시간 색상 변동 needs 발생 시
- 건물 footprint (`LT_C_SPBD`) — FU 40
- PMTiles 갱신 주기 단축 (월 1회 → 주 1회) — 데이터 freshness 요구 변경 시

## 13. 참조 — 형제 프로젝트

| 프로젝트 | 패턴 | 데이터 형식 | SSOT |
|---|---|---|---|
| `gongzzang/apps/gongzzang-design-lab` | **PMTiles 정적 100%** (본 SP9 와 동일) | PMTiles + R2 + Cloudflare Workers | PMTiles + 원본 SHP |
| `gongzzang-develop/.../platform-web` | 백엔드 MVT 100% | `ST_AsMVT` 동적 PBF endpoint | PostGIS |

본 SP9 는 **gongzzang-design-lab 패턴 직접 차용** + 우리 도메인 (Rust ETL, listing denormalize) 통합.

활용 가능한 reference 자산:
- [`scripts/pipeline/steps/build-pmtiles.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\scripts\pipeline\steps\build-pmtiles.ts) — tippecanoe 옵션
- [`docs/PMTILES_GUIDE.md`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\docs\PMTILES_GUIDE.md) — 빌드/배포 가이드
- [`UnifiedPolygonGLLayer.tsx`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\UnifiedPolygonGLLayer.tsx) — orchestrator
- [`usePolygonSources.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\hooks\usePolygonSources.ts) — source/layer 등록 패턴
- [`useMapboxGLInit.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\hooks\useMapboxGLInit.ts) — Naver `_mapbox` 추출

## 14. 참고

- ADR [0016](../../adr/0016-medallion-base-layer-postgis-silver-pmtiles-gold.md), [0014 (superseded)](../../adr/0014-base-layer-defer-pmtiles.md), [0015](../../adr/0015-v-world-acl-rearchitecture.md)
- 도구: [tippecanoe](https://github.com/felt/tippecanoe), [PMTiles spec](https://github.com/protomaps/PMTiles), [shapefile crate](https://crates.io/crates/shapefile), [pmtiles JS lib](https://www.npmjs.com/package/pmtiles)
- 데이터: [공공데이터포털 연속지적도](https://www.data.go.kr/data/15004245/fileData.do), [공공데이터포털 행정구역](https://www.data.go.kr/data/15054956/fileData.do)
