//! `User` Aggregate 단위 테스트 — `entity.rs`/`methods.rs`/`errors.rs` 동작 검증.
//!
//! `entity.rs`에서 `#[path = "entity_tests.rs"] mod tests;` 로 포함해요. 테스트 양이 커서
//! (≥500 줄) 행동별 그룹으로 추가 분해 — 이 파일은 *aggregator* 역할만 하고 실제
//! `#[test]` 함수들은 `entity_tests/<group>.rs` 에 있어요.
//!
//! 그룹:
//! - `constructors` — `try_new` (minimal 6-arg) + `display_name` / `zitadel_sub` 검증 + `UserKind`.
//! - `full_and_serde` — `try_new_full` (14-arg) + `UserRole` + 전체 serde round-trip.
//! - `mutations` — 도메인 mutation 메서드 (`verify_*`, `revoke_*`, `add_role`, `soft_delete` 등).
//!
//! 각 서브 파일은 `use super::super::*;` 로 `crate::entity::*` 를, `use super::fixtures::*;`
//! 로 공통 헬퍼를 임포트해요.

#![allow(clippy::expect_used, clippy::unwrap_used)]

/// 모든 서브 테스트 모듈이 공유하는 fixture 헬퍼.
mod fixtures {
    use chrono::{DateTime, Utc};
    use shared_kernel::broker_license::BrokerLicense;
    use shared_kernel::business_number::BusinessNumber;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;

    use super::super::{User, UserKind};

    pub(super) fn sample_email() -> Email {
        Email::try_new("alice@example.com").expect("valid")
    }

    // 64-char hex (lowercase). SHA-256 of "test" — fixture only.
    pub(super) const SAMPLE_PHONE_HASH: &str =
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    pub(super) fn sample_business_number() -> BusinessNumber {
        BusinessNumber::try_new("1234567891").expect("valid")
    }

    pub(super) fn sample_broker_license() -> BrokerLicense {
        BrokerLicense::try_new("11-2024-12345").expect("valid")
    }

    pub(super) fn sample_user(now: DateTime<Utc>) -> User {
        User::try_new(
            Id::new(),
            "zitadel-sub",
            sample_email(),
            "Alice",
            UserKind::Individual,
            now,
        )
        .expect("valid")
    }
}

#[path = "entity_tests/constructors.rs"]
mod constructors;
#[path = "entity_tests/full_and_serde.rs"]
mod full_and_serde;
#[path = "entity_tests/mutations.rs"]
mod mutations;
