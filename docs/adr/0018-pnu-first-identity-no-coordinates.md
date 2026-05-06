# ADR-0018: 매물 정체성 — PNU-First (좌표는 매칭/검색에 사용 안 함)

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (PMTiles 폴리곤 base layer) + [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md) (마커 렌더) 직후 — 매물 식별/매칭의 SSOT 키 결정 |

## 컨텍스트

매물(`Listing`) 을 식별·매칭할 때 두 후보 키가 있음:

1. **좌표** (`geom_point`, EPSG:4326 Point) — GPS / 사용자 클릭 / 지오코딩
2. **PNU** (`parcel_pnu`, 19자리 정부 식별자) — 시도(2)+시군구(3)+읍면동(3)+리(2)+산여부(1)+본번(4)+부번(4)

기존 SP6-ii MVP 는 bbox 기반 좌표 검색 + 핀 마커 (`listing-map.tsx`) 로 시작. SP9 가 PMTiles 폴리곤 base layer 를 깔면서, 사용자 인터랙션 모델이 *"핀 클릭"* 에서 *"필지 폴리곤 클릭"* 으로 자연스레 진화함.

| 비교 | 좌표 | PNU |
|---|---|---|
| 출처 | GPS/지오코딩/클릭 | 정부 발급 |
| 오차 | 수 m ~ 수십 m | 0 |
| 한 객체 매칭 | 무수히 많은 점 → 매번 조금씩 다름 | 정확히 1개, 영구 |
| SSOT | 추정값 | **법적 식별자** |
| 신뢰 | 떨어짐 | 높음 |
| 사용자 입력 | 직접 어려움 | 직접 어려움 |
| 시스템 입력 | 가능 | PNU lookup (V-World) |

## 결정

**매물의 모든 비즈니스 매칭은 PNU 로.** 좌표는 매물 식별/검색/매칭의 어디에도 사용하지 않음.

핵심 원칙:

1. **`parcel_pnu` 가 매물의 정체성** — `not null`, 변경 불가에 가까움
2. **모든 폴리곤/행정/지목 매핑은 PNU 파생** — `admin_code`, `parcel_land_use_type`, `parcel_zoning` 모두 V-World `LP_PA_CBND_BUBUN` (PNU lookup) 결과를 denormalize
3. **시각화 = 필지 폴리곤** — 사용자는 PMTiles 의 polygon 을 클릭, 프론트가 `properties.pnu` 추출 → `GET /listings?pnu=...`. 핀 안 찍음
4. **검색 = PNU IN 절** — `WHERE parcel_pnu IN (...)` 또는 `WHERE admin_code = '...' AND ...`. bbox geom 절 안 씀
5. **좌표는 `geom_point` 컬럼에 legacy 보존** — 단, 신규 코드는 안 읽음. 점진 deprecate path

### 매물에 좌표 (`geom_point`) 가 *불필요* 한 이유

| 과거 가정 (틀림) | 새 모델 (정답) |
|---|---|
| 핀을 어디 찍을지 좌표 필요 | 마커는 폴리곤. PNU 면 충분 |
| 반경 500m 검색 = 좌표 거리 | "이 필지 반경" = polygon 중심을 클라가 PMTiles 에서 계산. 매물에 좌표 저장 X |
| "산단 안 매물" = spatial JOIN | 매물.PNU 가 산단 PMTiles 의 PNU 목록에 있는지 — 좌표 안 봄 |
| bbox 검색 | viewport 안 PNU 추출 (PMTiles `queryRenderedFeatures`) → `WHERE parcel_pnu IN (...)` |

좌표가 진짜로 필요한 use case 가 미래에 등장하면 별도 ADR.

### `listing.geom_point` deprecate path (3단계)

[migrations/README.md](../../migrations/README.md) 의 "컬럼 제거 = 3단계" 정책 적용:

1. **즉시 (T4-T5)**: 신규 코드는 `geom_point` 안 읽음/안 씀. 기존 SP6-ii 의 bbox 검색은 동작 유지 (legacy)
2. **중기 (SP9 완료 후 ~1주)**: 프론트가 폴리곤 클릭 모델로 완전 전환. bbox 검색 endpoint deprecate. `geom_point` 미참조 grep 0 확인
3. **장기 (deprecate 후 ~1개월)**: `ALTER TABLE listing DROP COLUMN geom_point` + `DROP INDEX listing_geom_gist_idx` 별도 마이그레이션

### ADR 0017 과의 관계 — 마커의 *의미* 변경

ADR 0017 은 *"매물 마커 렌더 = Naver Marker + Canvas + BitmapStampCache"* 로 결정했음. 본 ADR 후 마커의 *의미* 가 명확화됨:

- ADR 0017 의 마커 = **PNU 위에 떠 있는 정보 라벨** (가격, 매물 수, 산단 라벨 등)
- 매물의 *위치 자체* = 필지 폴리곤 (PMTiles)
- 즉, "마커" 는 정보 표시 layer 일 뿐 매물 식별 키가 아님

ADR 0017 은 그대로 유효 — 라벨 렌더링은 여전히 lab 패턴으로.

## 코너케이스 (정직)

### 1. PNU 가 변경되는 경우 (분필/합병/지번 변경)

지적도 갱신 시 한 필지가 둘로 쪼개지거나 합쳐지면 PNU 가 바뀜. 이 경우:

- 월간 ETL (SP9 T6) 후 `listing.parcel_lookup_at` 이 stale 인 row 검출
- 운영 cron 이 V-World 재호출 → 매핑 갱신 또는 admin 수동 검토
- 빈도: 전국 1.4억 필지 중 분필/합병 연 < 1% — 부담 적음

### 2. 한 필지에 건물 여러 동 (같은 PNU 내부 위치)

같은 공장 단지 안에 빌딩 A, B 등 여러 매물이 있을 때:

- 모두 같은 PNU 공유 → 폴리곤 클릭 시 "이 필지 매물 N개" 패널
- 정확한 빌딩 위치는 *건물 footprint* (V-World `LT_C_SPBD`, FU 40) 별도 layer 로 해결
- 본 ADR 의 PNU-first 와 충돌 없음

### 3. 사용자가 PNU 를 모름 (등록 시)

매물 등록 화면에서 PNU 직접 입력 X. 둘 중 하나의 path:

- 주소 검색 → 백엔드 reverse geocoding (V-World 또는 Juso) → PNU
- 지도에서 필지 폴리곤 클릭 → `properties.pnu` 추출
- 둘 다 *프론트 단계* 에서 PNU 확보 후 `parcel_pnu` 필드로 백엔드에 전달

이 경로 자체는 좌표가 *입력 변환의 중간 단계* 로 잠시 등장할 수 있음. 단, 매물 row 에는 좌표 안 저장됨 — `geom_point` 그대로 NULL.

## 대안

| 안 | 평가 |
|---|---|
| **A. 본 결정 — PNU-First, 좌표 X** | ✅ SSOT 명료, 좌표 오차 원천 차단, ADR 0016 PMTiles 모델과 일관 |
| B. 좌표-First (legacy MVP) | ❌ 측정 오차/지오코딩 오차 누적, 정부 SSOT 와 분리, bbox 검색 비용 |
| C. PNU + 좌표 양쪽 저장 (현재 schema) | 🟡 작동은 하나 *두 SSOT* 됨 — 어느 쪽이 진실인지 헷갈림. AGENTS.md § 6 SSOT 위반 |
| D. PNU + polygon centroid 저장 (자동 계산) | 🟡 좌표가 PNU 파생이라 SSOT 단일은 유지. 그러나 *어디에도 안 쓰면* 그냥 dead column. 필요해지면 그때 |

## 결과

### 긍정
- **SSOT 명료** — 매물 식별 = PNU. 다른 키 없음 → 헷갈림 0
- **좌표 입력 오차 차단** — 사용자가 좌표 직접 안 보냄, 지오코딩 오차 무관
- **검색 단순화** — `WHERE parcel_pnu = ...` 또는 `WHERE admin_code = ...` 만. PostGIS spatial query 의존성 감소
- **ADR 0016 PMTiles 모델과 자연 정렬** — viewport 안 PNU → 매물 IN 검색
- **컬럼 제거 가능** — `geom_point` deprecate path 명확

### 부정
- **PNU 변경 시 listing stale 위험** — 월간 재매핑 cron 으로 mitigation, 빈도 낮음
- **건물 단위 정밀 위치 안 됨** — FU 40 (건물 footprint) 별도 해결
- **기존 SP6-ii bbox 검색 deprecate** — 프론트 전환 필요 (T5 + 후속)
- **`geom_point` 컬럼 dead 기간** — 3단계 deprecate 동안 schema 노이즈

### 영향 영역
- `migrations/10001_core_tables.sql` — `geom_point`, `listing_geom_gist_idx` 향후 제거 (SP9 완료 후)
- `crates/listing-domain/` — `Listing` 엔티티의 `geom_point` 필드 deprecate
- `services/api/src/routes/listings.rs` — `GET /listings` 의 `bounds` 파라미터 deprecate, `pnu` / `admin_code` 파라미터 추가 (T4-T5)
- `apps/web/components/listings/listing-map.tsx` — 핀 마커 렌더 제거, 폴리곤 클릭 모델로 (T5)
- 신규 [`crates/parcel-lookup/`](../../crates/) — V-World `fetch_by_pnu` wrapper

## 재검토 트리거

- 좌표가 진짜로 필요한 use case 발생 (예: 정확한 건물 위치, 외부 매물 import) → 좌표 SSOT 정의 별도 ADR
- PNU 가 변경되는 비율 > 5%/년 (지적도 대대적 갱신) → 별도 매핑 정책 ADR
- 해외 매물 추가 (PNU 미존재) → identity 모델 재설계
- PMTiles 폴리곤 클릭이 UX 적으로 사용자 혼란 → 핀 fallback 도입 검토

## 참조

- → [ADR 0013](./0013-listing-search-naver-maps.md) (Naver Maps SDK)
- → [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (PMTiles 폴리곤 base layer)
- → [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md) (마커 렌더 — 본 ADR 후 *정보 라벨* 의미로 명확화)
- → [migrations/10001_core_tables.sql](../../migrations/10001_core_tables.sql) (`listing.parcel_pnu` not null, `geom_point` nullable)
- → [migrations/30009_listing_polygon_denormalize.sql](../../migrations/30009_listing_polygon_denormalize.sql) (PNU 파생 컬럼 4개)
- → [SP9 spec](../superpowers/specs/2026-05-06-sub-project-9-medallion-base-layer-design.md) (PNU 기반 검색 흐름)
- AGENTS.md § 6 SSOT, § 7 도메인 어휘 (Parcel/Listing)
