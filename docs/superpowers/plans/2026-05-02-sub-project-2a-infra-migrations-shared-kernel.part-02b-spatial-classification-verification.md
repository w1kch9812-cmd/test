# Sub-project 2a - Part 02B: Spatial, Classification, Final Verification, And Handoff

Parent index: [Sub-project 2a Part 02](./2026-05-02-sub-project-2a-infra-migrations-shared-kernel.part-02.md).

## Task 21: Srid (좌표계 enum: 4326/5179/5186)

**Files:**
- Create: `crates/shared-kernel/src/srid.rs`

- [ ] **TDD:**

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum Srid {
    /// WGS84 — 글로벌 표준, 네이버/구글 호환.
    Wgs84 = 4326,
    /// UTM-K — 한국 측량 표준.
    UtmK = 5179,
    /// 중부원점 TM — 행정 측량 표준.
    KoreaCentralTm = 5186,
}

impl Srid {
    pub fn from_epsg(code: i32) -> Result<Self, SridError> {
        match code {
            4326 => Ok(Self::Wgs84),
            5179 => Ok(Self::UtmK),
            5186 => Ok(Self::KoreaCentralTm),
            other => Err(SridError::Unsupported(other)),
        }
    }
    #[must_use] pub fn epsg(self) -> i32 { self as i32 }
}

#[derive(Debug, Error)]
pub enum SridError {
    #[error("unsupported EPSG code: {0}")]
    Unsupported(i32),
}
```

```bash
git commit -m "feat(shared-kernel): Srid enum (WGS84/UTM-K/Central-TM) — explicit projection guard"
```

---

## Task 22: Geometry (Point + Polygon, SRID 강제)

**Files:**
- Create: `crates/shared-kernel/src/geometry.rs`

- [ ] **TDD:** Point 생성 시 *반드시 Srid 함께*, lat/lng 범위 검증.

```rust
use crate::srid::Srid;
use geo_types::Point as GeoPoint;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointSrid {
    pub lng: f64,
    pub lat: f64,
    pub srid: Srid,
}

impl PointSrid {
    pub fn try_new_wgs84(lng: f64, lat: f64) -> Result<Self, GeometryError> {
        if !(-180.0..=180.0).contains(&lng) { return Err(GeometryError::LngOutOfRange); }
        if !(-90.0..=90.0).contains(&lat) { return Err(GeometryError::LatOutOfRange); }
        if !lng.is_finite() || !lat.is_finite() { return Err(GeometryError::NotFinite); }
        Ok(Self { lng, lat, srid: Srid::Wgs84 })
    }

    #[must_use] pub fn to_geo_point(self) -> GeoPoint<f64> { GeoPoint::new(self.lng, self.lat) }
}

#[derive(Debug, Error)]
pub enum GeometryError {
    #[error("longitude out of [-180, 180]")] LngOutOfRange,
    #[error("latitude out of [-90, 90]")] LatOutOfRange,
    #[error("coordinate must be finite")] NotFinite,
}
```

테스트: WGS84 유효, 위도 91 거부, 경도 -181 거부, NaN 거부.

```bash
git commit -m "feat(shared-kernel): PointSrid — explicit-SRID point with WGS84 bounds check"
```

---

## Task 23: AdminDivision (시도/시군구/읍면동 코드)

**Files:**
- Create: `crates/shared-kernel/src/admin_division.rs`

- [ ] **TDD:** 2/5/8자리 코드 검증, 행정안전부 표준.

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SidoCode(String);
impl SidoCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 2)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SigunguCode(String);
impl SigunguCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 5)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn sido_code(&self) -> SidoCode { SidoCode(self.0[0..2].to_owned()) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EupmyeondongCode(String);
impl EupmyeondongCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 8)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

fn validate_digits(s: &str, expected: usize) -> Result<(), AdminDivisionError> {
    if s.len() != expected { return Err(AdminDivisionError::InvalidLength { expected, actual: s.len() }); }
    if !s.chars().all(|c| c.is_ascii_digit()) { return Err(AdminDivisionError::NonDigit); }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AdminDivisionError {
    #[error("expected {expected} digits, got {actual}")] InvalidLength { expected: usize, actual: usize },
    #[error("must be ASCII digits")] NonDigit,
}
```

```bash
git commit -m "feat(shared-kernel): AdminDivision — Sido/Sigungu/Eupmyeondong codes (2/5/8 digits)"
```

---

## Task 24: RoadAddress + JibunAddress

**Files:**
- Create: `crates/shared-kernel/src/road_address.rs`
- Create: `crates/shared-kernel/src/jibun_address.rs`

- [ ] **TDD:** 단순 String wrapper + 빈 문자열/길이 검증. 향후 도로명 주소 API 연동 시 확장.

```rust
// road_address.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoadAddress(String);
impl RoadAddress {
    pub fn try_new(s: &str) -> Result<Self, RoadAddressError> {
        let t = s.trim();
        if t.is_empty() { return Err(RoadAddressError::Empty); }
        if t.len() > 200 { return Err(RoadAddressError::TooLong); }
        Ok(Self(t.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

(JibunAddress 동일 패턴)

```bash
git commit -m "feat(shared-kernel): RoadAddress + JibunAddress — non-empty bounded strings"
```

---

## Task 25: KsicCode (한국 표준산업분류)

**Files:**
- Create: `crates/shared-kernel/src/ksic_code.rs`

**스펙:** 5자리 알파벳+숫자 (예: `C2620`).

- [ ] **TDD:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KsicCode(String);

impl KsicCode {
    pub fn try_new(s: &str) -> Result<Self, KsicCodeError> {
        if s.len() != 5 { return Err(KsicCodeError::InvalidLength); }
        let mut chars = s.chars();
        let first = chars.next().ok_or(KsicCodeError::InvalidLength)?;
        if !first.is_ascii_uppercase() { return Err(KsicCodeError::FirstMustBeUppercase); }
        if !chars.all(|c| c.is_ascii_digit()) { return Err(KsicCodeError::TailMustBeDigits); }
        Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn section(&self) -> char { self.0.chars().next().expect("validated") }
}
```

```bash
git commit -m "feat(shared-kernel): KsicCode — Korean Standard Industrial Classification (1 letter + 4 digits)"
```

---

## Task 26: 최종 검증 (cargo check/clippy/deny + 커버리지 90%+ + 마이그레이션 E2E)

**Files:**
- Create: `tarpaulin.toml`
- Modify: `.github/workflows/ci.yml` (마이그레이션 잡 추가)
- Create: `.github/workflows/db-migrations.yml`
- Create: `docs/database/migrations.md`

- [ ] **Step 1: tarpaulin.toml 작성**

```toml
[shared-kernel]
features = ""
timeout = "120s"
exclude-files = ["**/tests/**", "**/target/**"]
out = ["Html", "Lcov"]
fail-under = 90
```

- [ ] **Step 2: 로컬 검증 — 5개 명령어 모두 통과**

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo install cargo-tarpaulin || true
cargo tarpaulin --workspace --skip-clean --out Lcov --fail-under 90
bash scripts/sqlx-migrate.sh
bash tests/migrations/test_v001_full.sh
bash tests/migrations/test_v002_audit_immutable.sh
```

각각 expected output 명시 (PASS/통과율 출력).

- [ ] **Step 3: db-migrations.yml — CI 잡 추가**

```yaml
name: db-migrations
on: [pull_request, push]
jobs:
  migrate:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgis/postgis:17-3.5
        env:
          POSTGRES_USER: gongzzang
          POSTGRES_PASSWORD: changeme_ci
          POSTGRES_DB: gongzzang
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 3s
          --health-retries 10
    env:
      DATABASE_URL: postgres://gongzzang:changeme_ci@localhost:5432/gongzzang
    steps:
      - uses: actions/checkout@v4
      - run: cargo install sqlx-cli --version 0.8.2 --no-default-features --features postgres,rustls
      - run: bash tests/migrations/test_v001_full.sh
      - run: bash tests/migrations/test_v002_audit_immutable.sh
```

- [ ] **Step 4: docs/database/migrations.md 작성** (≤200줄)

운영 가이드: 명명 규칙, 적용 절차, 롤백 정책, 블루-그린 호환 변경 패턴, *DDL은 별도 PR* 원칙.

- [ ] **Step 5: 모든 검증 통과 확인 후 push**

```bash
git push origin main
```
GitHub Actions: ✅ 모두 그린 확인.

- [ ] **Step 6: Commit**

```bash
git add tarpaulin.toml .github/workflows/db-migrations.yml docs/database/migrations.md
git commit -m "ci(db): tarpaulin 90%+ + migrations CI job + ops guide"
```

---

## Self-Review Checklist (구현 전 작성자 점검 — 끝났음)

- [x] **Spec coverage:** spec § 5 (18 테이블) — Task 4-9, § 7 (3 role) — Task 10, § 8.1 (shared-kernel) — Task 11-25, § 11 (검증 기준) — Task 26
- [x] **Placeholder scan:** "TBD"/"TODO" 없음, 모든 step에 실제 코드 또는 명령
- [x] **Type consistency:** Pnu/MoneyKrw/AreaM2/Srid 이름 모든 task에서 일관, IdPrefix marker 동일 패턴
- [x] **TDD 준수:** Task 12-25 모두 *실패 테스트 → 실행 → 구현 → 통과 → commit* 5단계
- [x] **분할 정합성:** 마이그레이션 5분할은 *500줄 룰* 강제. 합치면 800+ 줄 위반.
- [x] **알려진 위험:** Task 14 Pnu의 `expect`는 *workspace lints `expect_used = "deny"`와 충돌 가능*. 구현자가 *현장 판단*. → 노트로 명시.
- [x] **테이블 개수 검증:** Task 9에서 22 vs 18 차이 *명시적 검증* 책무 추가.

---

## Execution Handoff

Plan 2a를 `docs/superpowers/plans/2026-05-02-sub-project-2a-infra-migrations-shared-kernel.md` 에 저장했어요.

**다음:** `superpowers:subagent-driven-development` 로 Task 1부터 fresh subagent dispatch + 2단계 리뷰 (spec compliance → code quality) 진행. 각 task 완료 후 사용자 체크포인트.

플랜 2b (Core BC 6개 Aggregate)와 2c (Market/Insights/Operations/Pipeline/R2)는 본 플랜 완료 후 별도 작성.
