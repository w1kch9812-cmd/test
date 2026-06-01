# Sub-project 2a - Part 02A: Shared-Kernel Korean Value Objects

Parent index: [Sub-project 2a Part 02](./2026-05-02-sub-project-2a-infra-migrations-shared-kernel.part-02.md).
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
