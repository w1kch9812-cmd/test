# ADR-0016: 지도 base layer — PMTiles 100% (Bronze raw archive + Gold derived view), ADR 0014 supersede

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| Supersedes | [ADR 0014](./0014-base-layer-defer-pmtiles.md) (전국 base layer 보류) |
| 컨텍스트 | 사용자가 "필지 + 행정구역(시도/시군구/읍면동/리) 모든 polygon을 지도에 표시" 요구 |

> **변경 이력 (같은 날, 결정 진화)**
> - 1차안: PMTiles 단독 (ADR 0014에서 보류)
> - 2차안: Medallion (Bronze/Silver/Gold) — PostGIS Silver 추가로 SSOT 강화 (yagni 위반 의심)
> - **확정안 (본 문서)**: PMTiles 100% (Bronze raw + Gold PMTiles). PostGIS Silver는 Phase 3+ 분석 needs 발생 시 별도 ADR로 추가.

## 컨텍스트

ADR 0014 는 R2 PMTiles 단독 base layer 방향을 다음 4가지 결함을 들어 보류했음:

1. SSOT 위반 (V-World → ETL → PMTiles → R2 = 사본 3개)
2. Freshness lag (batch 빌드 시간/일 단위)
3. 자동강제 부재 (ETL cron 깨지면 침묵)
4. Scope creep 의심 (B2B 산단 사용자 needs 미검증)

본 ADR 시점(2026-05-06) 에서 사용자가 명시적으로 "필지 + 행정구역 모든 polygon 표현" 요구 → **4번(scope) 의심 해소**.

남은 1-3번은 Bronze archive + ETL 자동 검증으로 해소 (아래).

## 핵심 통찰

### 1. PMTiles 는 "사본"이 아니라 "derived view"

ADR 0014 의 "사본 3개" 비판은 *layer* 와 *copy* 를 혼동했음. 정확한 그림:

```
[공공데이터포털]
   외부 진실
      │
      ▼ 월 1회 다운로드 (cron)
🥉 Bronze (R2 archive)
   gongzzang-bronze/<YYYY-MM>/parcel.shp.zip
   = "외부 데이터의 SSOT"
   = 정부가 우리한테 준 그대로 1년 보관
      │
      ▼ tippecanoe ETL (deterministic)
🥇 Gold (R2 정적)
   gongzzang-static/v1/parcels.pmtiles
   gongzzang-static/v1/admin.pmtiles
   = derived view — Bronze에서 결정적으로 빌드
   = 사용자 지도 렌더 직결
      │
      ▼ HTTP Range
   [Naver Maps gl + mapbox-gl PMTiles protocol]
```

**SSOT 분리**:
- **외부 진실 SSOT** = Bronze (raw SHP)
- **렌더 view** = Gold PMTiles (Bronze 에서 deterministic 재빌드 가능)

PMTiles 를 사본이라 부른 건 잘못. *Bronze 가 있으면* PMTiles 는 derived view.

### 2. PostGIS Silver 는 yagni — Phase 3+ 에서 추가

Phase 1-2 검색 needs:
- 매물 검색 (위치/지목/면적 필터) → listing 테이블 + denormalize 컬럼 (`parcel_pnu`, `admin_code`)
- polygon 클릭 시 정보 → PMTiles `feature.properties` (PNU, 주소, 공시지가, 지목 미리 포함)
- 줌별 polygon 표시 → PMTiles minzoom/maxzoom

진짜 PostGIS spatial JOIN 가 필요한 시점:
- Phase 3+ 분석 dashboard ("산단별 빈 공장용지 통계" 등)
- 그때 별도 ADR 로 PostGIS polygon 테이블 점진 도입

지금 PostGIS Silver 추가는 yagni 위반 + 비용 +$300-1500/월 (Phase 3 기준).

### 3. R2 + Cloudflare Egress 무료 = polygon 시스템 사실상 0원

| 자원 | 크기 | R2 비용/월 |
|---|---|---|
| Bronze SHP (12개월 archive) | ~6-12 GB | $0.10 |
| Gold PMTiles (필지 + 행정) | ~10-15 GB | $0.20 |
| Class B reads (사용자 트래픽) | (Range request) | < $0.10 (R2 egress 무료) |
| **합계** | | **~$0.40/월** |

10K DAU 와도 100K DAU 와도 동일. Cloudflare R2의 egress $0 정책 덕분.

## 결정

1. **Bronze + Gold 2계층 채택** (Silver 미도입):
   - Bronze: 공공데이터포털 raw SHP archive (R2, 1년 보관)
   - Gold: 다중 R2 정적 artifact

2. **Gold 를 다중 artifact 로 분리 — 갱신 주기 별 데이터 분할**:

   | Gold artifact | 내용 | 갱신 주기 | 크기 |
   |---|---|---|---|
   | `parcels.pmtiles` | polygon **geometry + PNU 만** | 분기/년 (지적도 거의 안 변함) | ~10 GB |
   | `admin.pmtiles` | 행정구역 geometry + 코드 + 이름 | 분기 1회 | ~50 MB |
   | `industrial-complex.pmtiles` | 산단 geometry + 코드 + 이름 + type | 분기 1회 | ~5 MB |
   | `parcel-attrs/<sigungu>.json` | PNU 별 jiga, gosi_year, gosi_month, land_use_type, area_m2 | 매년 1회 (공시지가 고시) | ~50 MB / 시군구 |
   | `complex-stats.json` | 산단별 매물 수, 빈 공장용지 수 등 precomputed aggregates | 매일 cron | ~1 MB |
   | `density-by-sigungu.json` | 시군구별 매물 밀도 heatmap | 매일 cron | ~1 MB |
   | `listings-by-pnu/<sigungu>.json` | PNU 별 매물 list (id, price 등 마커용) | 매시간 cron | ~10 MB / 시군구 |

   각 artifact 가 별도 갱신 주기 → 변경 없는 데이터는 CDN 영구 캐시. PMTiles 가 작은 파일 수십 개 매월 rebuild 하지 않음.

3. **PostGIS Silver 는 도입 안 함** — Phase 3+ ad-hoc 분석 / temporal audit needs 발생 시 별도 ADR 로 점진 도입

4. **클라이언트 spatial 계산 = `turf.js`** — viewport 안 polygon 으로 다음 가능:
   - `turf.booleanContains` — "이 산단 안 매물", "이 필지 안 건물"
   - `turf.distance` — "반경 500m 내 다른 매물"
   - `turf.area`, `turf.intersect` 등
   - PMTiles 가 viewport 안 features 자동 제공 → turf 가 그 위에서 계산
   - PostGIS GIST index 같은 spatial index 는 없지만, viewport 단위 데이터라 부담 적음 (보통 100-1000 features)

5. **listing 테이블에 denormalize 컬럼 추가** — 매물 검색용:
   - `listing.parcel_pnu` (매물 등록 시 좌표 → PNU lookup, 1회만 호출)
   - `listing.admin_code` (동일 — 어느 행정구역 안인지)
   - `listing.land_use_type`, `listing.zoning` 등 polygon attribute 일부도 denormalize
   - Polygon 갱신 시 listing 재매핑 cron (월 1회)

6. **검증된 reference 활용** — gongzzang-design-lab 가 같은 패턴 운영 중 ([`scripts/pipeline/steps/build-pmtiles.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\scripts\pipeline\steps\build-pmtiles.ts), [`UnifiedPolygonGLLayer.tsx`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\UnifiedPolygonGLLayer.tsx), [`docs/PMTILES_GUIDE.md`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\docs\PMTILES_GUIDE.md))

## ADR 0014 비판의 해소

| ADR 0014 비판 | 본 ADR 의 해소 |
|---|---|
| **SSOT 위반** (사본 3개) | Bronze = 외부 진실 SSOT. Gold = derived view (사본이 아닌 view). PostGIS 사본 아예 안 만듦 |
| **Freshness lag** | 매물 등록은 즉시 (listing 테이블 직접 업데이트). polygon 갱신은 월 1회 (정부 데이터 자체가 분기/월 갱신이라 lag 0) |
| **자동강제 부재** | ETL smoke test (강남 PNU `1168010100107370000` Gold 에 존재 + Bronze sha256 검증) + GitHub Actions 실패 → Sentry 알림 |
| **Scope creep** | 사용자 명시 결정으로 해소 |

## 데이터 소스 (Bronze)

| 종류 | 소스 | 갱신 | 라이선스 |
|---|---|---|---|
| 필지 | [공공데이터포털 연속지적도 SHP](https://www.data.go.kr/data/15004245/fileData.do) | 월 1회 | 공공저작물 (출처 표기) |
| 시도/시군구/읍면동/리 | [공공데이터포털 행정구역 SHP](https://www.data.go.kr/data/15054956/fileData.do) | 분기 1회 | 공공저작물 |
| 산업단지 | V-World `LT_C_WGISIE*` (4종) — 분기 batch | 분기 | 공공저작물 |

## 비용 비교 (polygon 시스템만, 앱 base 제외)

| 옵션 | 100 DAU | 1K DAU | 10K DAU | 100K DAU | SSOT | 분석 |
|---|---|---|---|---|---|---|
| **PMTiles 100% (본 ADR)** | **$1** | **$2** | **$5** | **$10** | Bronze | Phase 4+ 별도 |
| Hybrid (PostGIS Silver 추가) | $20 | $80 | $400 | $1,500 | PostGIS | 즉시 |
| 100% 백엔드 MVT (gongzzang-develop) | $80 | $400 | $2,000 | $8,000 | PostGIS | 즉시 |

> 위는 polygon 시스템만의 추가 비용. 앱 base (RDS for listing/user/auth, Fargate, Redis) 는 셋 다 동일하며 별도.

## 아키텍처

```
[공공데이터포털 SHP] [V-World 산단 WFS]
        │                    │
        ▼ 월/분기 GitHub Actions cron
┌─────────────────────────────────────────────┐
│ 🥉 Bronze (R2)                               │
│   gongzzang-bronze/<YYYY-MM>/*.shp.zip       │
│   12개월 archive (감사 / 재현)                │
└──┬───────────────────────────────────────┬──┘
   │                                        │
   │ tippecanoe (geometry only)            │ aggregation (precompute)
   ▼                                        ▼
┌─────────────────────────────────────────────┐
│ 🥇 Gold (R2 정적, 다중 artifact)              │
│                                              │
│ Geometry (분기 갱신, 거의 안 변함):           │
│   v1/parcels.pmtiles    geometry + PNU       │
│   v1/admin.pmtiles      geometry + 코드      │
│   v1/complex.pmtiles    geometry + 코드      │
│                                              │
│ Attributes (매년 갱신, 시군구별 분할):        │
│   v1/parcel-attrs/<sigungu>.json             │
│     { "1168010100107370000": {               │
│         jiga, gosi_year, gosi_month,         │
│         land_use_type, area_m2 }, ... }      │
│                                              │
│ Aggregates (매일 precompute):                │
│   v1/complex-stats.json                      │
│   v1/density-by-sigungu.json                 │
│                                              │
│ Listings (매시간 precompute):                │
│   v1/listings-by-pnu/<sigungu>.json          │
│                                              │
│   v1/manifest.json   (모든 artifact 버전)    │
└──┬──────────────────────────────────────────┘
   │ HTTP Range (PMTiles) + JSON fetch
   │ Cloudflare CDN cache
   ▼
┌─────────────────────────────────────────────┐
│ 프론트                                        │
│   Naver Maps gl + pmtiles JS                 │
│   turf.js (viewport spatial 계산)            │
│   클릭 → PMTiles PNU + R2 attrs JSON 합성    │
│   "반경 500m" / "산단 안" → turf.js          │
└─────────────────────────────────────────────┘

[Backend — listing 만 PostgreSQL]
listing 등록 → PNU/admin_code lookup → listing 컬럼 저장
listing 검색 = WHERE 절 (denormalize 활용)
실시간 매물 정보는 listings-by-pnu JSON precompute (매시간 cron)
```

## 줌 레벨 LOD

| Zoom | 표시 | PMTiles |
|---|---|---|
| 0-7 | 시도 (17) | admin.pmtiles minzoom 0 |
| 8-10 | 시군구 (~250) | admin.pmtiles minzoom 8 |
| 11-13 | 읍면동 (~3,500) | admin.pmtiles minzoom 11 |
| 14-15 | 리 (~17,000) | admin.pmtiles minzoom 14 |
| 16+ | 필지 (1.4억) + 산업단지 | parcels.pmtiles minzoom 16 / complex.pmtiles |

zoom < 16 에서는 필지 안 그림 (data 양 폭증 방지). gongzzang-design-lab 도 동일 LOD 정책.

## 클라이언트 필터링 + 계산 (PostGIS 없이)

### PMTiles + properties 기반 필터

PMTiles 의 `parcels.pmtiles` 는 geometry + PNU 만. attributes 는 별도 R2 JSON. 필터하려면 viewport 안 PNU 를 추출 → 그 시군구의 `parcel-attrs/<sigungu>.json` fetch → 클라가 합성:

```typescript
// 1. viewport 안 features 추출
const features = mb.queryRenderedFeatures({ layers: ['parcel-fill'] });

// 2. 시군구별 attrs JSON 그룹 fetch (CDN 캐시)
const sigunguGroups = groupBy(features, f => f.properties.pnu.substring(0, 5));
const attrsBySigungu = await Promise.all(
    Object.keys(sigunguGroups).map(sigungu =>
        fetch(`/static/v1/parcel-attrs/${sigungu}.json`).then(r => r.json())
    )
);

// 3. PNU 별 합성 + 필터
const filtered = features.filter(f => {
    const attrs = attrsBySigungu[/* sigungu lookup */][f.properties.pnu];
    return attrs.land_use_type === 'FactorySite'
        && attrs.jiga <= 10_000_000
        && attrs.area_m2 >= 1000;
});

// 4. PMTiles 에 mb.setFilter 적용 (PNU 화이트리스트)
mb.setFilter('parcel-fill', ['in', ['get', 'pnu'], ['literal', filtered.map(f => f.properties.pnu)]]);
```

attrs JSON 은 CDN 캐시 → 같은 시군구 다시 보면 즉시.

### turf.js 클라이언트 spatial 계산

```typescript
import * as turf from '@turf/turf';

// "이 매물 반경 500m 다른 매물"
const otherListings = listings.filter(l =>
    turf.distance(turf.point([this.lng, this.lat]), turf.point([l.lng, l.lat])) < 0.5  // km
);

// "이 산단 안 매물"
const complexPolygon = mb.queryRenderedFeatures({ layers: ['complex-fill'] })[0];
const inComplex = listings.filter(l =>
    turf.booleanContains(complexPolygon, turf.point([l.lng, l.lat]))
);

// "공장용지 안 빈 매물 수"
const factoryParcels = features.filter(f => attrs[f.properties.pnu].land_use_type === 'FactorySite');
const emptyCount = factoryParcels.filter(p =>
    !listingsByPnu[p.properties.pnu] || listingsByPnu[p.properties.pnu].length === 0
).length;
```

PostGIS spatial JOIN 의 90% 가 클라이언트로 가능. viewport 단위 데이터 (보통 100-1000 features) 라 부담 적음.

### Precomputed aggregates 활용

ad-hoc 이 아닌 *고정 차원* 통계는 ETL 시점 precompute → R2 JSON:

```typescript
// "산단별 빈 공장용지 수" — DB GROUP BY 안 함, JSON 그대로 사용
const stats = await fetch('/static/v1/complex-stats.json').then(r => r.json());
// { "남동국가산업단지": { empty_factory_parcels: 47, total: 312 }, ... }
```

ETL이 매일 한 번 모든 산단별·시군구별 aggregates 빌드 → 사용자 fetch 시 즉시.

### 한계 (정직)

- ❌ **사용자 임의 차원 dashboard** ("이번주 등록 + 공장용지 + 1억 이하 + 강남구 외" 같은 ad-hoc) — precompute 못 함
- ❌ **Temporal audit** ("2026-01-15 시점 polygon") — history 저장 없음
- ❌ **DB CHECK constraint** — ETL 검증으로 mitigation
- ❌ **매물 등록 즉시 polygon 색 변경** — listings-by-pnu JSON 매시간 cron

이 4개가 정말 필요해지면 Phase 3+ 에서 PostGIS 점진 도입 (별도 ADR).

## listing 검색 (Phase 1-2)

매물 등록 시 한 번:

```rust
// services/api/src/routes/listings.rs (등록 핸들러)
async fn register(req: ListingCreate) -> Result<Listing> {
    let pnu = lookup_pnu_at_point(req.lng, req.lat).await?;
    let admin = lookup_admin_at_point(req.lng, req.lat).await?;

    sqlx::query!(
        "INSERT INTO listing (..., parcel_pnu, admin_code, ...) VALUES (..., $1, $2, ...)",
        pnu.as_str(), admin.code,
    ).execute(&pool).await?;
}
```

`lookup_pnu_at_point` 구현 1차안 — Bronze SHP을 백엔드 메모리에 R-tree index 로 (또는 SQLite spatialite 임시 테이블, polygon 데이터만). DB polygon 테이블 없이도 lookup 가능.

검색은 listing 테이블 단독:

```sql
SELECT * FROM listing
WHERE admin_code = '1168010100'  -- 역삼동
  AND parcel_land_use_type = 'FactorySite'
  AND price_per_m2 <= 10000000;
```

## 후속 (Phase 3+ 시 재검토)

다음 needs 발생 시 별도 ADR:
- 분석 dashboard ("산단별 빈 공장용지 통계", "월별 공시지가 변동" 등) → PostGIS polygon 테이블 + GROUP BY 쿼리 → ADR 0017?
- polygon 색상이 매물 등록에 따라 실시간 변동 → 백엔드 동적 MVT endpoint (develop 패턴 부분 도입)
- 건물 footprint (FU 40) → V-World `LT_C_SPBD` 별도

본 ADR 의 결정은 *지금 필요한 것만*. 미래 확장 path 는 열려 있음.

## SSS 7 기둥 적용

| 기둥 | 본 ADR 의 적용 |
|---|---|
| **1 일관성** | 같은 Bronze → 항상 같은 Gold (ETL deterministic) |
| **2 자동강제** | ETL smoke test, sha256 검증, cron 실패 → Sentry, redeploy 차단 |
| **3 추적성** | Bronze 12개월 archive, build manifest (timestamp + checksum + row count) |
| **4 안전성** | 정적 PMTiles → 런타임 에러 0. listing 등록 시 PNU lookup 검증 |
| **5 가시성** | manifest 비교 — "지금 배포된 Gold 가 어느 Bronze build 에서 왔는가" |
| **6 SSOT** | Bronze = 외부 진실 SSOT. Gold = derived. PostGIS 사본 안 만듦 |
| **7 명확성** | 본 ADR + Bronze/Gold 책임 분리 명시 |

## 대안 (재검토)

| 안 | 평가 |
|---|---|
| **A. PMTiles only — Bronze 미보존** | ❌ PMTiles 깨지면 복구 불가, 외부 검증 불가, 감사 불가 |
| **B. PostGIS only + GeoJSON API** | ❌ 모든 viewport 호출이 DB → 비용 폭증, CDN 캐시 이점 0 |
| **C. PostGIS + Martin/자체 MVT 서버** | 🟡 SSS 적합. 그러나 운영 부담 +$300-2000/월 + Phase 1-2 needs 미정당화 |
| **D. Hybrid (PMTiles + PostGIS 사본)** | 🟡 분석 가능하지만 yagni — 같은 polygon 데이터 두 곳 저장 |
| **E. PMTiles 100% (Bronze + Gold) — 본 결정** | ✅ Phase 1-2 needs 충족, 비용 사실상 0, gongzzang-design-lab 검증 패턴, 미래 확장 path 열림 |

## 결과

**즉시**:
- ADR 0014 → status: Superseded by ADR 0016
- SP9 sub-project: SHP 다운로드 → tippecanoe → R2 업로드 + 프론트 PMTiles layer + listing denormalize 컬럼
- [crates/data-clients/r2-public-data/](../../crates/data-clients/r2-public-data/) PMTiles header parser 활성화 (정적 호스팅 직접 fetch + 클라이언트 필터링 검증용)

**조건부 추가 (Phase 3+)**:
- 분석 dashboard needs 발생 시 PostGIS polygon 테이블 점진 도입 → 별도 ADR
- 실시간 polygon 색상 변동 needs 시 백엔드 동적 MVT endpoint → 별도 ADR

## 참고

- Supersedes: [ADR 0014](./0014-base-layer-defer-pmtiles.md)
- 관련: [ADR 0015](./0015-v-world-acl-rearchitecture.md) (V-World ACL 재설계 — Bronze layer raw 보존 정책의 베이스라인)
- Spec: [`2026-05-06-sub-project-9-medallion-base-layer-design.md`](../superpowers/specs/2026-05-06-sub-project-9-medallion-base-layer-design.md)
- Plan: [`2026-05-06-sub-project-9-medallion-base-layer.md`](../superpowers/plans/2026-05-06-sub-project-9-medallion-base-layer.md)
- 형제 프로젝트 reference (PMTiles 패턴 검증):
  - [`gongzzang-design-lab/components/map/naver/UnifiedPolygonGLLayer.tsx`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\components\map\naver\UnifiedPolygonGLLayer.tsx)
  - [`gongzzang-design-lab/scripts/pipeline/steps/build-pmtiles.ts`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\scripts\pipeline\steps\build-pmtiles.ts)
  - [`gongzzang-design-lab/docs/PMTILES_GUIDE.md`](C:\Users\admin\Desktop\gongzzang\apps\gongzzang-design-lab\docs\PMTILES_GUIDE.md)
- 도구: [tippecanoe](https://github.com/felt/tippecanoe), [PMTiles spec](https://github.com/protomaps/PMTiles), [pmtiles JS lib](https://www.npmjs.com/package/pmtiles)
- AGENTS.md § 6 SSOT, § 8 SSOT 매트릭스
