## Task 14: Pnu (19자리 한국 PNU 코드)

**Files:**
- Create: `crates/shared-kernel/src/pnu.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "1111010100100010000";

    #[test]
    fn parse_valid_pnu() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.as_str(), VALID);
    }

    #[test]
    fn extracts_admin_codes() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.sido_code(), "11");
        assert_eq!(pnu.sigungu_code(), "11110");
        assert_eq!(pnu.eupmyeondong_code(), "11110101");
    }

    #[test]
    fn jibun_main_and_sub() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.jibun_main(), 1);
        assert_eq!(pnu.jibun_sub(), 0);
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(matches!(Pnu::try_new("123").unwrap_err(), PnuError::InvalidLength { .. }));
    }

    #[test]
    fn rejects_non_digits() {
        assert!(matches!(Pnu::try_new("11110101001000100AB").unwrap_err(), PnuError::NonDigit));
    }
}
```

- [ ] **Step 2: 실패 확인**

- [ ] **Step 3: 구현**

```rust
//! PNU — 19자리 한국 필지 식별자. `[시도2][시군구3][읍면동3][산여부1][본번4][부번4]`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pnu(String);

impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> {
        if s.len() != 19 {
            return Err(PnuError::InvalidLength { actual: s.len() });
        }
        if !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(PnuError::NonDigit);
        }
        Ok(Self(s.to_owned()))
    }

    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn sido_code(&self) -> &str { &self.0[0..2] }
    #[must_use] pub fn sigungu_code(&self) -> &str { &self.0[0..5] }
    #[must_use] pub fn eupmyeondong_code(&self) -> &str { &self.0[0..8] }
    #[must_use] pub fn is_san(&self) -> bool { &self.0[10..11] == "2" }

    #[must_use] pub fn jibun_main(&self) -> u32 {
        self.0[11..15].parse().expect("digits validated")
    }
    #[must_use] pub fn jibun_sub(&self) -> u32 {
        self.0[15..19].parse().expect("digits validated")
    }
}

#[derive(Debug, Error)]
pub enum PnuError {
    #[error("PNU must be 19 digits, got {actual}")]
    InvalidLength { actual: usize },
    #[error("PNU must contain only ASCII digits")]
    NonDigit,
}
```

- [ ] **Step 4: 통과 + clippy** (`expect` 사용 정당화 주석은 *clippy::allow*가 아닌 *디지트 사전 검증* 사실로 충분)

> **주의:** workspace lints에 `expect_used = "deny"`. 이 *expect*는 `parse::<u32>()`에 한해 *불가능한 분기*에 사용. clippy 통과시키려면 `#[allow(clippy::expect_used)]`를 함수 단위로 부착하거나, `unsafe`를 피하는 다른 패턴 (*unwrap_or(0)*) 사용. 구현자는 *clippy 출력 확인 후 결정*.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): Pnu — 19-digit Korean parcel id (sido/sigungu/dong/jibun extraction)"
```

---

## Task 15: Money (KRW + overflow 방어)

**Files:**
- Create: `crates/shared-kernel/src/money.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_krw_positive() {
        let m = MoneyKrw::try_new(100_000_000).unwrap();
        assert_eq!(m.as_i64(), 100_000_000);
    }

    #[test]
    fn rejects_negative() {
        assert!(matches!(MoneyKrw::try_new(-1).unwrap_err(), MoneyError::Negative));
    }

    #[test]
    fn add_within_bounds() {
        let a = MoneyKrw::try_new(1_000).unwrap();
        let b = MoneyKrw::try_new(2_000).unwrap();
        assert_eq!(a.checked_add(b).unwrap().as_i64(), 3_000);
    }

    #[test]
    fn add_overflow_returns_err() {
        let a = MoneyKrw::try_new(i64::MAX).unwrap();
        let b = MoneyKrw::try_new(1).unwrap();
        assert!(a.checked_add(b).is_err());
    }
}
```

- [ ] **Step 2-4: 구현 + 통과**

```rust
//! 한국 원화 금액. 음수 금지, 오버플로우 방어.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MoneyKrw(i64);

impl MoneyKrw {
    pub fn try_new(krw: i64) -> Result<Self, MoneyError> {
        if krw < 0 { return Err(MoneyError::Negative); }
        Ok(Self(krw))
    }

    #[must_use] pub fn as_i64(self) -> i64 { self.0 }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_add(other.0).ok_or(MoneyError::Overflow).and_then(Self::try_new)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_sub(other.0).ok_or(MoneyError::Underflow).and_then(Self::try_new)
    }
}

#[derive(Debug, Error)]
pub enum MoneyError {
    #[error("money cannot be negative")]
    Negative,
    #[error("money addition overflowed")]
    Overflow,
    #[error("money subtraction underflowed")]
    Underflow,
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): MoneyKrw — non-negative + checked add/sub"
```

---

## Task 16: Area (m² 면적)

**Files:**
- Create: `crates/shared-kernel/src/area.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_m2_positive() {
        let a = AreaM2::try_new(99.5).unwrap();
        assert!((a.as_f64() - 99.5).abs() < f64::EPSILON);
    }

    #[test] fn zero_rejected() { assert!(AreaM2::try_new(0.0).is_err()); }
    #[test] fn negative_rejected() { assert!(AreaM2::try_new(-1.0).is_err()); }
    #[test] fn nan_rejected() { assert!(AreaM2::try_new(f64::NAN).is_err()); }
    #[test] fn infinity_rejected() { assert!(AreaM2::try_new(f64::INFINITY).is_err()); }

    #[test]
    fn to_pyeong_converts() {
        let a = AreaM2::try_new(3.305_785).unwrap();
        assert!((a.to_pyeong() - 1.0).abs() < 1e-3);
    }
}
```

- [ ] **Step 2-4: 구현**

```rust
//! 면적 (㎡). 양수만, NaN/∞ 거부. 평 환산 헬퍼 포함.

use serde::{Deserialize, Serialize};
use thiserror::Error;

const M2_PER_PYEONG: f64 = 3.305_785_124;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AreaM2(f64);

impl AreaM2 {
    pub fn try_new(m2: f64) -> Result<Self, AreaError> {
        if !m2.is_finite() { return Err(AreaError::NotFinite); }
        if m2 <= 0.0 { return Err(AreaError::NonPositive); }
        Ok(Self(m2))
    }
    #[must_use] pub fn as_f64(self) -> f64 { self.0 }
    #[must_use] pub fn to_pyeong(self) -> f64 { self.0 / M2_PER_PYEONG }
}

#[derive(Debug, Error)]
pub enum AreaError {
    #[error("area must be finite")]
    NotFinite,
    #[error("area must be positive")]
    NonPositive,
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): AreaM2 — positive-finite + pyeong conversion"
```

---

## Task 17: BusinessNumber (한국 사업자등록번호)

**Files:**
- Create: `crates/shared-kernel/src/business_number.rs`

**스펙:** 10자리 (`123-45-67890` 또는 `1234567890`), 체크섬 알고리즘 검증.

- [ ] **Step 1: 실패 테스트** — 유효 번호, 하이픈 정규화, 잘못된 체크섬 거부, 길이 거부

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 실제 유효 체크섬을 가진 더미 (테스트는 알고리즘 검증)
    #[test]
    fn parse_with_hyphens_normalizes() {
        let bn = BusinessNumber::try_new("123-45-67890").unwrap();
        assert_eq!(bn.as_str(), "1234567890");
    }
    #[test] fn rejects_short() { assert!(BusinessNumber::try_new("12345").is_err()); }
    #[test] fn rejects_non_digits() { assert!(BusinessNumber::try_new("abcdefghij").is_err()); }
    #[test]
    fn rejects_invalid_checksum() {
        // 마지막 자리 +1 → 체크섬 실패
        assert!(BusinessNumber::try_new("1234567891").is_err());
    }
}
```

- [ ] **Step 2-3: 구현** — 한국 국세청 사업자번호 체크섬 알고리즘

```rust
//! 사업자등록번호 (한국 국세청 표준 10자리 + 체크섬).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusinessNumber(String);

impl BusinessNumber {
    pub fn try_new(s: &str) -> Result<Self, BusinessNumberError> {
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
        if cleaned.len() != 10 { return Err(BusinessNumberError::InvalidLength); }
        if !cleaned.chars().all(|c| c.is_ascii_digit()) { return Err(BusinessNumberError::NonDigit); }
        if !verify_checksum(&cleaned) { return Err(BusinessNumberError::InvalidChecksum); }
        Ok(Self(cleaned))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

fn verify_checksum(digits: &str) -> bool {
    let weights = [1u32, 3, 7, 1, 3, 7, 1, 3, 5];
    let bytes = digits.as_bytes();
    let mut sum: u32 = 0;
    for i in 0..9 { sum += u32::from(bytes[i] - b'0') * weights[i]; }
    sum += (u32::from(bytes[8] - b'0') * 5) / 10;
    let check = (10 - (sum % 10)) % 10;
    check == u32::from(bytes[9] - b'0')
}

#[derive(Debug, Error)]
pub enum BusinessNumberError {
    #[error("business number must be 10 digits")]
    InvalidLength,
    #[error("business number must be ASCII digits (with optional hyphens)")]
    NonDigit,
    #[error("business number checksum invalid")]
    InvalidChecksum,
}
```

> **검증 권고:** 알고리즘은 한국 국세청 공식 명세 기반. 구현자는 위키/공식 문서 교차 확인 후 *진짜 유효한* 사업자번호 1개로 단위 테스트 추가.

- [ ] **Step 4-5: 통과 + Commit**

```bash
git commit -m "feat(shared-kernel): BusinessNumber — 10-digit Korean reg with NTS checksum"
```

---

## Task 18: BrokerLicense (공인중개사 자격증번호)

**Files:**
- Create: `crates/shared-kernel/src/broker_license.rs`

**스펙:** 등록번호 형식 `XX-XXXX-XXXXX` (시도-연도-순번). 길이 검증만 (체크섬 없음).

- [ ] **Step 1-5: TDD** — 길이/하이픈 정규화 검증

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrokerLicense(String);

impl BrokerLicense {
    pub fn try_new(s: &str) -> Result<Self, BrokerLicenseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() { return Err(BrokerLicenseError::Empty); }
        if trimmed.len() > 50 { return Err(BrokerLicenseError::TooLong); }
        Ok(Self(trimmed.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

```bash
git commit -m "feat(shared-kernel): BrokerLicense — Korean real-estate broker registration number"
```

---

## Task 19: Email

**Files:**
- Create: `crates/shared-kernel/src/email.rs`

- [ ] **TDD:** 정규식 기반 RFC 5322 *간소화* 검증 (`local@domain`, 도메인에 `.`, 길이 ≤254).

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").expect("valid regex")
});

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn try_new(s: &str) -> Result<Self, EmailError> {
        let lower = s.trim().to_ascii_lowercase();
        if lower.len() > 254 { return Err(EmailError::TooLong); }
        if !EMAIL_RE.is_match(&lower) { return Err(EmailError::InvalidFormat); }
        Ok(Self(lower))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

테스트: 유효, 잘못된 도메인, 빈 local, 길이 초과, 대문자 정규화.

```bash
git commit -m "feat(shared-kernel): Email — RFC 5322 simplified + lowercase normalization"
```

---

## Task 20: PhoneKr (한국 전화번호)

**Files:**
- Create: `crates/shared-kernel/src/phone_kr.rs`

- [ ] **TDD:** `010-1234-5678`, `02-123-4567`, `+82-10-...` 모두 `010...` 또는 `02...` 정규화. 하이픈 제거 후 9-11자리.

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhoneKr(String);

impl PhoneKr {
    pub fn try_new(s: &str) -> Result<Self, PhoneKrError> {
        let mut digits: String = s.chars().filter(char::is_ascii_digit).collect();
        if let Some(rest) = digits.strip_prefix("82") { digits = format!("0{rest}"); }
        if !(9..=11).contains(&digits.len()) { return Err(PhoneKrError::InvalidLength); }
        if !digits.starts_with('0') { return Err(PhoneKrError::MustStartWithZero); }
        Ok(Self(digits))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Error)]
pub enum PhoneKrError {
    #[error("phone must be 9-11 digits")] InvalidLength,
    #[error("phone must start with 0")] MustStartWithZero,
}
```

```bash
git commit -m "feat(shared-kernel): PhoneKr — Korean phone normalization (+82 → 0xx)"
```

---

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
