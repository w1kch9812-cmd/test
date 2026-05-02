# Sub-project 2b-ii: R2 정적 BC 4개 (Parcel/Building/IndustrialComplex/Manufacturer) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.
>
> **CRITICAL:** Read [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) before each task.

**Goal:** Spec § 4가 R2 정적으로 분류한 4 BC (Parcel, Building, IndustrialComplex, Manufacturer)의 *Aggregate struct + Reader trait* 정의. Repository (write) 아님 — *Reader (read-only)*. Implementation은 sub-project 4 (`crates/data-clients/r2-public-data/`).

**Architecture:** 8 task. shared-kernel에 공통 enum 추가 → 4 BC crate 생성 → 통합 검증.

**Tech Stack:** Rust 1.88, async-trait, geo-types (Polygon), 기존 shared-kernel 값 객체.

**Patterns from 2a/2b-i:**
- spec § 4 + § 8.4 verbatim 참조
- 값 객체 표준 패턴 (#[serde(transparent)], try_new, Display, FromStr)
- Aggregate 패턴 (struct + try_new + Reader trait port-only)
- CI 그린 게이트 (3 workflows)
- 348 누적 테스트 → +50 추가 목표

**Reader vs Repository (핵심 구분):**
- *Repository* (RDS 동적, ex: User/Listing) — find + save (mutation)
- *Reader* (R2 정적, 외부 공공 데이터) — fetch only, *no save/update* — 외부 API에서 우리 R2로 ETL 동기화는 sub-project 4의 worker 책임

**알려진 위험:** Aggregate 필드는 V-World/data.go.kr 응답 스키마 기반 *추정*. Sub-project 4 통합 시 실제 응답에 맞춰 *refactor 예상*. 본 plan은 *상호 작용 인터페이스*만 정의 — 구체 컬럼은 implementer 재량 + spec § 8.4 근거.

---

## File Structure

### shared-kernel 추가 (Tasks 1-3)
```
crates/domain/core/shared-kernel/src/
├── land_use_type.rs       NEW (지목 — 대/전/답/임야/공장용지/...)
├── zoning.rs              NEW (용도지역 — 주거/상업/공업/녹지)
├── polygon_srid.rs        NEW (Polygon + SRID, geo-types::Polygon wrapper)
└── bounding_box.rs        NEW (지도 영역 — 기존 listing-domain에서 추출)
```

### 4 BC crate (Tasks 4-7)
```
crates/domain/core/
├── parcel/                         NEW
│   ├── Cargo.toml
│   ├── README.md
│   └── src/{lib, entity, errors, reader}.rs
├── building/                       NEW
├── industrial-complex/             NEW
└── manufacturer/                   NEW
```

각 BC crate는 동일 4-파일 구조 + 테스트.

---

## Task 1: shared-kernel — LandUseType + Zoning enums

**Spec 근거:** spec § 8.4 line 916-917 (`land_use_type: LandUseType`, `zoning: Zoning`).

**LandUseType (지목)** — 한국 토지대장 표준 28종 中 산업용 부동산 도메인에 자주 등장하는 ~10종으로 시작:
- `Building` (대) — 일반 건축물 부지
- `Field` (전) — 밭
- `Paddy` (답) — 논
- `Forest` (임야)
- `FactorySite` (공장용지)
- `WarehouseSite` (창고용지)
- `Road` (도로)
- `Park` (공원)
- `Other` (기타) — 8 위 외 모두 포괄 (tax debt rerun 28종 전부 모델링하지 않음)

**Zoning (용도지역)** — 국토계획법 4 대분류:
- `Residential` (주거지역)
- `Commercial` (상업지역)
- `Industrial` (공업지역)
- `Green` (녹지지역)
- `Other` (관리지역/농림지역/자연환경보전지역 등)

**File:**
- `crates/domain/core/shared-kernel/src/land_use_type.rs`
- `crates/domain/core/shared-kernel/src/zoning.rs`
- `lib.rs` 갱신 (alphabetical)

**Pattern:** unit-like enum, 동일 표준 패턴 (rename_all = "snake_case" + 7 derives + as_str + Display + FromStr + Error::Unknown). Tests ≥7 each.

- [ ] Step 1: 두 파일 + tests 작성
- [ ] Step 2: lib.rs 갱신 (alphabetical)
- [ ] Step 3: Commit + push + CI green

```bash
git commit -m "feat(shared-kernel): LandUseType (지목 9값) + Zoning (용도지역 5값)"
```

---

## Task 2: shared-kernel — PolygonSrid

**Spec 근거:** spec § 8.4 line 920 (`geom: Polygon`).

**Pattern:** PointSrid의 Polygon 버전. WGS84 강제, exterior ring + holes 지원.

```rust
use crate::srid::Srid;
use geo_types::Polygon as GeoPolygon;

pub struct PolygonSrid {
    pub polygon: GeoPolygon<f64>,
    pub srid: Srid,
}

pub fn try_new_wgs84(polygon: GeoPolygon<f64>) -> Result<Self, GeometryError>
pub fn to_geo_polygon(&self) -> &GeoPolygon<f64>
```

**Validation:**
- exterior ring 좌표 모두 finite
- 모든 lng ∈ [-180, 180], lat ∈ [-90, 90]
- exterior ring 최소 4 점 (closing point 포함, GeoJSON 표준)
- *self-intersection 검증 안 함* (geo-types crate가 알고리즘 제공하지만 비용 큼 — 외부 데이터 신뢰)

**File:** `crates/domain/core/shared-kernel/src/polygon_srid.rs`

- [ ] Tests (≥10): WGS84 valid simple polygon, polygon with hole, lng out of range, lat out of range, NaN, exterior < 4 points, geom-types interop, Copy semantics (Clone only — Polygon is heap), serde.
- [ ] Commit + CI green.

```bash
git commit -m "feat(shared-kernel): PolygonSrid — WGS84 Polygon with explicit SRID + bounds check"
```

---

## Task 3: shared-kernel — BoundingBox 통합 + listing-domain 정리

**기존:** `crates/domain/core/listing/src/repository.rs`에 `BoundingBox` 정의.

**문제:** Plan 2b-ii Parcel/Building/IndustrialComplex Reader도 `BoundingBox` 필요 → 중복 방지 위해 shared-kernel로 이동.

**File:**
- Move: `crates/domain/core/listing/src/repository.rs::BoundingBox` → `crates/domain/core/shared-kernel/src/bounding_box.rs`
- Modify: listing-domain `repository.rs`에서 `use shared_kernel::bounding_box::BoundingBox` import

**Validation 추가:** `try_new(min_lng, min_lat, max_lng, max_lat) -> Result<Self, BoundingBoxError>`:
- min < max 강제
- 모든 좌표 finite + WGS84 범위 내

**Tests:** ≥8 (정상, min ≥ max 거부, NaN, 범위 외, contains 메서드 with PointSrid).

- [ ] Step 1: shared-kernel `bounding_box.rs` 작성 + tests
- [ ] Step 2: listing-domain `repository.rs` 갱신 (import 변경, 기존 BoundingBox 정의 삭제)
- [ ] Step 3: Commit + CI green

```bash
git commit -m "refactor(shared-kernel): extract BoundingBox to shared-kernel (used by listing + parcel readers)"
```

---

## Task 4: Parcel BC

**Spec 근거:** spec § 8.4 lines 907-936.

**Aggregate (10 fields):**

```rust
pub struct Parcel {
    pub pnu: Pnu,
    pub admin: AdminDivision,                          // sido + sigungu + dong codes
    pub road_address: Option<RoadAddress>,
    pub jibun_address: JibunAddress,                   // 항상 있음
    pub land_use_type: LandUseType,
    pub area: AreaM2,
    pub official_land_price_per_m2: Option<MoneyKrw>,  // 공시지가
    pub zoning: Zoning,
    pub geom: PolygonSrid,                             // 필지 폴리곤 (WGS84)
    pub fetched_at: DateTime<Utc>,                     // R2 객체 생성 시각
}
```

**Note:** `AdminDivision`은 단일 newtype 아님 — 3 newtypes (SidoCode/SigunguCode/EupmyeondongCode) 묶음. 이 task에서 **AdminDivision composite struct** 추가 필요:

```rust
// crates/domain/core/shared-kernel/src/admin_division.rs (기존 file에 추가)
pub struct AdminDivision {
    pub sido: SidoCode,
    pub sigungu: SigunguCode,
    pub eupmyeondong: EupmyeondongCode,
}

impl AdminDivision {
    pub fn try_new(sido: SidoCode, sigungu: SigunguCode, eupmyeondong: EupmyeondongCode)
        -> Result<Self, AdminDivisionError>
    {
        // sigungu 가 sido 와 일치 + eupmyeondong 가 sigungu 와 일치 검증
        if sigungu.sido_code() != sido { return Err(...) }
        if eupmyeondong.sigungu_code() != sigungu { return Err(...) }
        Ok(Self { sido, sigungu, eupmyeondong })
    }
}
```

3 코드 일관성 강제 → invariant 보장.

### Reader trait

```rust
// crates/domain/core/parcel/src/reader.rs
#[async_trait]
pub trait ParcelReader: Send + Sync {
    /// 단일 필지 조회 (R2 PMTiles 또는 JSON 인덱스).
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError>;
    /// 지도 영역 내 마커 (lightweight projection).
    async fn fetch_markers_in_bbox(
        &self,
        bbox: &BoundingBox,
    ) -> Result<Vec<ParcelMarker>, ReaderError>;
}

pub struct ParcelMarker {
    pub pnu: Pnu,
    pub centroid: PointSrid,
    pub area: AreaM2,
    pub land_use_type: LandUseType,
}

#[derive(Debug, thiserror::Error)]
pub enum ReaderError {
    #[error("not found")]
    NotFound,
    #[error("R2 fetch failed: {0}")]
    Fetch(String),
    #[error("PMTiles parse failed: {0}")]
    Parse(String),
}
```

### Files
- `crates/domain/core/parcel/{Cargo.toml, README.md, src/{lib,entity,errors,reader}.rs}`
- Modify: `crates/domain/core/shared-kernel/src/admin_division.rs` (composite struct 추가)
- Modify: 루트 `Cargo.toml` workspace.members

### Tests
- AdminDivision composite: 3 코드 일치/불일치 (5+ 테스트)
- Parcel struct: 정상 생성, geom WGS84 강제 (이미 PolygonSrid가 강제)
- Spec § 4 invariant: 모든 필드 R2-from-fetch (no mutation 메서드 정의 X)

- [ ] Steps 1-3: 각 파일 작성 + tests + commit + CI

```bash
git commit -m "feat(parcel-domain): Parcel Aggregate + ParcelReader trait + AdminDivision composite"
```

---

## Task 5: Building BC

**Spec 근거:** spec § 4 line 107 + 한국 건축물대장 표준 필드.

**Aggregate (≈12 fields, V-World 응답 추정):**

```rust
pub struct Building {
    pub pnu: Pnu,                                       // 필지 참조 (multi-building per parcel 가능)
    pub building_name: Option<String>,                  // 건물명 (≤200자)
    pub main_purpose_code: BuildingPurposeCode,         // 주용도 (단독주택/공장/창고/...)
    pub structure_code: BuildingStructureCode,          // 구조 (철근콘크리트/철골/...)
    pub total_floor_area_m2: AreaM2,                    // 연면적
    pub ground_floors: u8,                              // 지상층수
    pub underground_floors: u8,                         // 지하층수
    pub height_m: Option<f64>,                          // 높이
    pub use_approval_date: Option<chrono::NaiveDate>,   // 사용승인일
    pub geom: PolygonSrid,                              // 건물 폴리곤
    pub fetched_at: DateTime<Utc>,
}
```

**신규 enums (shared-kernel 추가 OR building-domain 내부):**
- `BuildingPurposeCode` — 한국 건축물대장 주용도 코드 (대분류 ~30종 → 산업용 도메인 핵심 ~10종 + Other)
- `BuildingStructureCode` — 구조 (~10종)

**Decision:** building-domain crate 내부에 두기 (shared-kernel은 *공통 invariant* 만, BC-specific enum은 BC crate에).

### Reader trait

```rust
#[async_trait]
pub trait BuildingReader: Send + Sync {
    /// 단일 PNU의 모든 건물 (한 필지에 여러 건물 가능).
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Vec<Building>, ReaderError>;
    /// 단일 건물 ID로 조회 (R2 키 기반).
    async fn fetch_by_id(&self, building_id: &str) -> Result<Option<Building>, ReaderError>;
}
```

`building_id`는 PNU + 건물 일련번호 결합 (정부 표준 형식이 정해진 기준 없음 — implementer 재량).

### Files + tests + commit + CI

```bash
git commit -m "feat(building-domain): Building Aggregate + BuildingReader trait + purpose/structure enums"
```

---

## Task 6: IndustrialComplex BC

**Spec 근거:** spec § 4 line 108 + 한국산업단지공단 산업단지 데이터.

**Aggregate (8 fields):**

```rust
pub struct IndustrialComplex {
    pub code: String,                                   // 산단 식별자 (정부 표준 코드)
    pub name: String,                                   // 산단명
    pub kind: IndustrialComplexKind,                    // 국가/일반/도시첨단/농공
    pub sigungu: SigunguCode,                           // 위치 행정구역
    pub designated_at: Option<chrono::NaiveDate>,       // 지정일
    pub total_area_m2: AreaM2,                          // 총 면적
    pub geom: PolygonSrid,                              // 산단 폴리곤
    pub fetched_at: DateTime<Utc>,
}

pub enum IndustrialComplexKind {
    National,           // 국가
    General,            // 일반
    UrbanHighTech,      // 도시첨단
    AgriculturalIndustrial,  // 농공
}
```

### Reader trait

```rust
#[async_trait]
pub trait IndustrialComplexReader: Send + Sync {
    async fn fetch_by_code(&self, code: &str) -> Result<Option<IndustrialComplex>, ReaderError>;
    /// 시군구 내 모든 산단 (사용자가 시군구 검색 시).
    async fn fetch_by_sigungu(
        &self,
        sigungu: &SigunguCode,
    ) -> Result<Vec<IndustrialComplex>, ReaderError>;
    async fn fetch_in_bbox(
        &self,
        bbox: &BoundingBox,
    ) -> Result<Vec<IndustrialComplex>, ReaderError>;
}
```

### Files + tests + commit + CI

```bash
git commit -m "feat(industrial-complex-domain): IndustrialComplex Aggregate + Reader (4-kind enum)"
```

---

## Task 7: Manufacturer BC

**Spec 근거:** spec § 4 line 109 + KOSIS/통계청 사업체조사 데이터.

**Aggregate (≈10 fields):**

```rust
pub struct Manufacturer {
    pub business_number: BusinessNumber,                // PK (FK 아님 — R2)
    pub company_name: String,                           // 회사명
    pub industrial_complex_code: Option<String>,        // 입주 산단 (있으면)
    pub pnu: Option<Pnu>,                               // 위치 (필지 매핑, 있으면)
    pub ksic_code: KsicCode,                            // 산업분류
    pub employee_count_band: EmployeeCountBand,         // 인원 구간 (개별 수치 X)
    pub founded_year: Option<u16>,                      // 설립연도
    pub representative_name: Option<String>,            // 대표자 (공시 정보)
    pub fetched_at: DateTime<Utc>,
}

pub enum EmployeeCountBand {
    OneToFour,           // 1-4
    FiveToNine,          // 5-9
    TenToFortyNine,      // 10-49
    FiftyToNinetyNine,   // 50-99
    OneHundredToTwoNinetyNine,  // 100-299
    ThreeHundredPlus,    // 300+
}
```

**Note:** `employee_count_band` enum (개별 수치 아님) — 통계청 공개 정보는 구간 단위. PIPA 회피 + 정확도 균형.

### Reader trait

```rust
#[async_trait]
pub trait ManufacturerReader: Send + Sync {
    async fn fetch_by_business_number(
        &self,
        bn: &BusinessNumber,
    ) -> Result<Option<Manufacturer>, ReaderError>;
    /// 산단 내 모든 입주 기업.
    async fn fetch_by_industrial_complex(
        &self,
        ic_code: &str,
    ) -> Result<Vec<Manufacturer>, ReaderError>;
    /// KSIC 대분류 letter로 필터 (예: 'C' = 제조업).
    async fn fetch_by_ksic_section(
        &self,
        section: char,
    ) -> Result<Vec<Manufacturer>, ReaderError>;
}
```

### Files + tests + commit + CI

```bash
git commit -m "feat(manufacturer-domain): Manufacturer Aggregate + Reader (KOSIS employee bands)"
```

---

## Task 8: 통합 검증

**Files:** 없음 (검증 task).

검증 명령어:

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo tarpaulin --workspace --skip-clean --out Lcov --fail-under 90
```

(CI 자동 실행)

**기대 결과:**
- 누적 테스트: 348 (2b-i 종료 시점) + ~50 (이번 plan) ≈ 400
- crate count: 5 (2b-i) → 9 (이번 plan: parcel + building + industrial-complex + manufacturer 추가)
- tarpaulin ≥90% 유지

**MEMORY.md 갱신:** Plan 2b-ii 완료 추가 + crate count + 테스트 수 갱신.

- [ ] Step 1: workspace 전체 cargo check
- [ ] Step 2: 누적 테스트 카운트 확인
- [ ] Step 3: project_progress.md 갱신
- [ ] Step 4: Commit

```bash
git commit -m "chore(2b-ii): integration validation — 9 BC crates, ~400 tests, tarpaulin ≥90%"
```

---

## Self-Review Checklist (plan 작성자 — 끝났음)

- [x] spec § 4 R2 정적 4 BC 모두 다룸
- [x] spec § 8.4 Parcel skeleton 따름 + 다른 3 BC도 동일 패턴
- [x] Reader trait는 read-only (mutation 없음)
- [x] Repository와 명확히 구분
- [x] BoundingBox 중복 제거 (shared-kernel 이동)
- [x] AdminDivision composite struct 추가 (3 코드 일관성)
- [x] BC-specific enum (BuildingPurposeCode 등)은 BC crate 내부
- [x] V-World 응답 미상 → 추정 + sub-project 4에서 refactor 예상 표시
- [x] tarpaulin 90% 게이트 유지

## 알려진 위험

1. **V-World 실제 응답 모름** — Aggregate 필드 추정. Sub-project 4에서 refactor 예상. 본 plan은 *인터페이스 형태*만 lock.
2. **R2 객체 키 형식 미정** — sub-project 4 (R2 디렉토리 구조)에서 정의. Reader trait은 *추상 식별자만* 받음.
3. **PMTiles vs JSON** — Parcel/Building은 PMTiles, Manufacturer는 JSON 가능성. Reader trait은 형식 무관.

## 완료 후 다음

- **Plan 2c** — Market BC (RealTransaction + CourtAuction Reader, Subscription/Inquiry Aggregate Phase 2+) + Insights BC (Bookmark + SearchHistory + AnalysisReport + Notification Aggregate) + Operations BC (Admin actions + queues + reports + featured + alerts) + Pipeline schedule/run + Outbox event publisher trait + 도메인 이벤트
- **Sub-project 3** — Auth (Zitadel JWT 미들웨어)
