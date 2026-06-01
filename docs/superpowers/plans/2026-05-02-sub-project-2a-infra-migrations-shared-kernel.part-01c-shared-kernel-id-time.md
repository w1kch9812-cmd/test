# Sub-project 2a - Part 01C: Shared-Kernel Id And Time Value Objects

Parent index: [Sub-project 2a Part 01](./2026-05-02-sub-project-2a-infra-migrations-shared-kernel.part-01.md).

## Task 12: Id (ULID + 도메인 prefix)

**Files:**
- Create: `crates/shared-kernel/src/id.rs`

**스펙 참조:** ID 컨벤션 — `<prefix>_<26 ULID 문자>`, 총 30자 (`usr_01HXY...`, `lst_01HXY...`).

- [ ] **Step 1: 실패 테스트 작성** (`id.rs` 끝에 `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_has_prefix_and_26_ulid_chars() {
        let id: Id<UserMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("usr_"));
    }

    #[test]
    fn parse_valid_id_roundtrips() {
        let raw = "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let id = Id::<UserMarker>::try_from_str(raw).unwrap();
        assert_eq!(id.as_str(), raw);
    }

    #[test]
    fn parse_wrong_prefix_fails() {
        let raw = "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let err = Id::<UserMarker>::try_from_str(raw).unwrap_err();
        assert!(matches!(err, IdError::WrongPrefix { .. }));
    }

    #[test]
    fn parse_wrong_length_fails() {
        let err = Id::<UserMarker>::try_from_str("usr_short").unwrap_err();
        assert!(matches!(err, IdError::InvalidLength { .. }));
    }
}
```

- [ ] **Step 2: 실행 — 실패**

```bash
cargo test -p shared-kernel id::tests
```

- [ ] **Step 3: 최소 구현**

```rust
//! 도메인 ID — `<prefix>_<26자 ULID>` 형식, 총 30자.

use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// ID prefix (BC별 marker로 컴파일 타임 구분).
pub trait IdPrefix {
    /// 3-4자 prefix (예: `"usr"`, `"lst"`).
    const PREFIX: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct UserMarker;
impl IdPrefix for UserMarker { const PREFIX: &'static str = "usr"; }

#[derive(Debug, Clone, Copy)]
pub struct ListingMarker;
impl IdPrefix for ListingMarker { const PREFIX: &'static str = "lst"; }

// (BuildingMarker, IndustrialComplexMarker, ManufacturerMarker, NotificationMarker, …)
// 후속 task에서 추가.

/// Phantom-typed ID. 런타임 표현은 30자 String.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<P: IdPrefix> {
    inner: String,
    #[serde(skip)]
    _marker: PhantomData<P>,
}

impl<P: IdPrefix> Id<P> {
    /// 새 ULID 생성.
    #[must_use]
    pub fn new() -> Self {
        let raw = format!("{}_{}", P::PREFIX, Ulid::new());
        Self { inner: raw, _marker: PhantomData }
    }

    /// 검증 후 Id로 래핑.
    pub fn try_from_str(s: &str) -> Result<Self, IdError> {
        if s.len() != 30 {
            return Err(IdError::InvalidLength { actual: s.len() });
        }
        let (prefix, rest) = s.split_once('_').ok_or(IdError::MissingDelimiter)?;
        if prefix != P::PREFIX {
            return Err(IdError::WrongPrefix {
                expected: P::PREFIX,
                actual: prefix.to_owned(),
            });
        }
        Ulid::from_string(rest).map_err(|_| IdError::InvalidUlid)?;
        Ok(Self { inner: s.to_owned(), _marker: PhantomData })
    }

    #[must_use]
    pub fn as_str(&self) -> &str { &self.inner }
}

impl<P: IdPrefix> Default for Id<P> { fn default() -> Self { Self::new() } }

#[derive(Debug, Error)]
pub enum IdError {
    #[error("invalid id length: expected 30, got {actual}")]
    InvalidLength { actual: usize },
    #[error("missing prefix delimiter '_'")]
    MissingDelimiter,
    #[error("wrong prefix: expected {expected}, got {actual}")]
    WrongPrefix { expected: &'static str, actual: String },
    #[error("invalid ULID body")]
    InvalidUlid,
}
```

- [ ] **Step 4: 실행 — 통과** (`cargo test -p shared-kernel id::tests`)

- [ ] **Step 5: lib.rs에 `pub mod id;` 추가, `cargo clippy --all-targets -- -D warnings` 통과**

- [ ] **Step 6: Commit**

```bash
git add crates/shared-kernel/src/id.rs crates/shared-kernel/src/lib.rs
git commit -m "feat(shared-kernel): Id<P> — ULID + domain prefix (30 chars, phantom-typed)"
```

---

## Task 13: Time (timestamp 헬퍼 + Asia/Seoul)

**Files:**
- Create: `crates/shared-kernel/src/time.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn now_utc_is_close_to_chrono_now() {
        let our = now_utc();
        let theirs = Utc::now();
        assert!((our - theirs).num_seconds().abs() < 2);
    }

    #[test]
    fn to_kst_converts_offset() {
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let kst = to_kst(utc);
        assert_eq!(kst.hour(), 9);
    }
}
```

- [ ] **Step 2: 실패 확인**

- [ ] **Step 3: 구현**

```rust
//! 시각 헬퍼 — UTC 저장 / KST 표시 분리.

use chrono::{DateTime, FixedOffset, Timelike, Utc};

/// 현재 UTC. 도메인 내부 표준.
#[must_use]
pub fn now_utc() -> DateTime<Utc> { Utc::now() }

/// KST(+09:00)로 변환. 사용자 노출 전용.
#[must_use]
pub fn to_kst(t: DateTime<Utc>) -> DateTime<FixedOffset> {
    let kst = FixedOffset::east_opt(9 * 3600).expect("valid offset");
    t.with_timezone(&kst)
}
```

- [ ] **Step 4: 통과 + clippy**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): Time helpers (now_utc, to_kst — UTC store / KST display)"
```

---
