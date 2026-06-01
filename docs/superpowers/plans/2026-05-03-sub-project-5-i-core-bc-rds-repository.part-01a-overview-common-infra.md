# Sub-project 5-i Core BC RDS Repository - Part 01A: Overview And Common Infra

Parent index: [Sub-project 5-i Core BC RDS Repository - Part 01](./2026-05-03-sub-project-5-i-core-bc-rds-repository.part-01.md).
# Sub-project 5-i: Core BC RDS Repository SQLx 구현 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **CRITICAL pre-read:** [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) + [docs/superpowers/specs/2026-05-03-sub-project-5-i-core-bc-rds-repository-design.md](../specs/2026-05-03-sub-project-5-i-core-bc-rds-repository-design.md)

**Goal:** Core BC (`Listing`, `ListingPhoto`) 의 `Postgres` 저장소 구현 + 기존 `PgUserRepository` 18 필드 완전 처리 + 모든 repo 메서드 `tracing::instrument` 적용 + integration test CI 게이트 명시.

**Architecture:** `crates/db/` 에 신규 `error_map.rs` 공통 helper + `listing.rs` + `listing_photo.rs`. 기존 `user.rs` 18 필드로 확장. CI walking-skeleton 워크플로우에 `cargo test --features integration` 단계 추가.

**Tech Stack:** Rust 1.88, sqlx 0.8 (runtime queries), Postgres 17 + PostGIS 3.5, async-trait, tracing 0.1.

**환경**: 로컬 cargo 작동 (MSVC 설치). 단위 테스트는 로컬 `cargo test` (5-30초). 통합 테스트는 CI walking-skeleton 의 PG 컨테이너에서 `cargo test --features integration` 실행. 로컬 통합 테스트는 옵션 (DATABASE_URL 설정 + 로컬 PG 필요).

**Repo**: `https://github.com/w1kch9812-cmd/test` (public, Actions 무제한 무료).

---

## Task 분해 (6 task)

- **Phase A (T1):** error_map.rs + Cargo features + ToRepoError 트레이트
- **Phase B (T2):** PgUserRepository 18 필드 보강 + tracing 적용 + 통합 테스트
- **Phase C (T3-T4):** PgListingRepository / PgListingPhotoRepository 신규
- **Phase D (T5):** walking-skeleton.yml integration 단계 추가 + CI 그린
- **Phase E (T6):** 통합 검증 + project_progress / MEMORY 갱신

각 task: 로컬 TDD 루프 (`cargo check` → `cargo clippy` → `cargo test` (단위만)) 통과 후 push → CI 통합 테스트.

---

## File Structure

신규:
```
crates/db/src/
├── error_map.rs            (신규 — MapFromSqlx trait + map_sqlx_err helper)
├── listing.rs              (신규 — PgListingRepository, ~280줄)
├── listing_photo.rs        (신규 — PgListingPhotoRepository, ~180줄)
└── (lib.rs 갱신)

crates/db/tests/
├── common.rs               (신규 — setup_test_pool() 헬퍼)
├── user_integration.rs     (신규 — 6 tests)
├── listing_integration.rs  (신규 — 9 tests, PostGIS 포함)
├── listing_photo_integration.rs (신규 — 6 tests)
└── error_map_integration.rs (신규 — 2 tests, unique violation)

크레이트 변경:
- crates/db/Cargo.toml — `[features] integration = []` + dev-deps `tokio` macros, `chrono`
- crates/db/src/lib.rs — `pub mod error_map; pub mod listing; pub mod listing_photo;`
- crates/db/src/user.rs — 8 필드 → 18 필드 + tracing::instrument
- crates/db 가 의존하는 도메인 추가: `listing-domain`, `listing-photo-domain`

CI:
- .github/workflows/walking-skeleton.yml — `cargo test --features integration` 단계 추가
```

---

## Phase A: 공통 인프라

### Task 1: `error_map.rs` + Cargo features + `MapFromSqlx` trait

**Files:**
- Create: `crates/db/src/error_map.rs`
- Modify: `crates/db/src/lib.rs`
- Modify: `crates/db/Cargo.toml`

- [ ] **Step 1: `crates/db/Cargo.toml` 업데이트**

```toml
[package]
name = "db"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 SQLx Repository 구현체"

[features]
integration = []

[dependencies]
shared-kernel = { path = "../domain/core/shared-kernel", version = "0.1.0" }
user-domain = { path = "../domain/core/user", version = "0.1.0" }
listing-domain = { path = "../domain/core/listing", version = "0.1.0" }
listing-photo-domain = { path = "../domain/core/listing-photo", version = "0.1.0" }
sqlx = { workspace = true }
chrono = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
geo-types = { workspace = true }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[lints]
workspace = true
```

- [ ] **Step 2: `crates/db/src/error_map.rs` 신규 작성**

```rust
//! `sqlx::Error` → 도메인 `RepoError` 공통 매핑.
//!
//! 모든 `Pg*Repository` 가 사용하는 단일 helper. 각 도메인 crate 의 `RepoError`
//! 가 [`MapFromSqlx`] 를 구현하면 [`map_sqlx_err`] 로 변환할 수 있어요.

use sqlx::Error as SqlxError;

/// 도메인 `RepoError` 가 `sqlx::Error` 로부터 생성될 수 있음을 표시하는 trait.
///
/// 본 trait 의 impl 은 본 crate (`db`) 안에서 정의되어요. orphan rule 때문에
/// trait 자체를 본 crate 가 정의하면 외부 타입에 impl 가능해요.
pub trait MapFromSqlx: Sized {
    /// Unique 제약 위반 — `Conflict`.
    fn conflict() -> Self;
    /// 일반 DB 에러 — 메시지만 보존 (정보 누설 방지).
    fn database(msg: String) -> Self;
}

/// `sqlx::Error` 를 도메인 `RepoError` 로 매핑.
///
/// - Unique violation → [`MapFromSqlx::conflict`]
/// - 그 외 → [`MapFromSqlx::database`]`(e.to_string())`
///
/// `RowNotFound` 은 `fetch_optional` 사용 시 `Ok(None)` 으로 반환되므로 본 함수에 도달
/// 하지 않아요.
#[must_use]
pub fn map_sqlx_err<E: MapFromSqlx>(e: SqlxError) -> E {
    if let SqlxError::Database(ref db_err) = e {
        if db_err.is_unique_violation() {
            return E::conflict();
        }
    }
    E::database(e.to_string())
}

// User domain RepoError
impl MapFromSqlx for user_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// Listing domain RepoError
impl MapFromSqlx for listing_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

// ListingPhoto domain RepoError
impl MapFromSqlx for listing_photo_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    /// `sqlx::Error::Io` 변종으로 `database()` 분기 검증 (unique violation 분기는 통합
    /// 테스트에서 진짜 DB 로 검증 — 본 함수에서 `DatabaseError` mock 을 만들 수 없음).
    #[test]
    fn io_error_maps_to_database() {
        let io = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test");
        let e = SqlxError::Io(io);
        let err: user_domain::repository::RepoError = map_sqlx_err(e);
        match err {
            user_domain::repository::RepoError::Database(s) => {
                assert!(s.contains("test") || s.contains("ConnectionRefused"));
            }
            _ => panic!("expected Database variant"),
        }
    }

    #[test]
    fn protocol_error_maps_to_database_for_listing() {
        let e = SqlxError::Protocol("bad protocol".into());
        let err: listing_domain::repository::RepoError = map_sqlx_err(e);
        assert!(matches!(
            err,
            listing_domain::repository::RepoError::Database(_)
        ));
    }
}
```

- [ ] **Step 3: `crates/db/src/lib.rs` 모듈 선언 추가**

```rust
//! `SQLx` `Postgres` `Repository` 구현체.
//!
//! 도메인 BC 가 정의한 `*Repository` trait 의 구현. `crates/db/src/error_map.rs`
//! 가 공통 `sqlx::Error` 매핑을 제공해요.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error_map;
pub mod listing;
pub mod listing_photo;
pub mod user;
```

- [ ] **Step 4: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p db
```

Expected: `error_map.rs` 컴파일 OK. `listing.rs` / `listing_photo.rs` 가 module 선언만 있고 파일 미존재 → 컴파일 에러.

`crates/db/src/listing.rs` + `crates/db/src/listing_photo.rs` 빈 stub 만들어서 컴파일 통과:

```rust
// crates/db/src/listing.rs
//! `PgListingRepository` (placeholder, T3 에서 구현).
```

```rust
// crates/db/src/listing_photo.rs
//! `PgListingPhotoRepository` (placeholder, T4 에서 구현).
```

다시 `cargo check -p db` → 통과 확인.

```bash
cargo test -p db --lib   # 2 unit tests in error_map
cargo clippy -p db --all-features -- -D warnings
```

Expected: 2 tests pass, clippy clean.

- [ ] **Step 5: Commit + push**

```bash
git add crates/db/Cargo.toml crates/db/src/lib.rs crates/db/src/error_map.rs crates/db/src/listing.rs crates/db/src/listing_photo.rs
git commit -m "feat(db): error_map common helper + MapFromSqlx trait + features.integration (SP5-i T1)

- error_map.rs: map_sqlx_err helper + MapFromSqlx trait (orphan rule 우회)
- 3 도메인 RepoError 에 impl (user / listing / listing-photo)
- Cargo.toml: [features] integration = [] + listing-domain/listing-photo-domain dep
- lib.rs: 모듈 선언 (listing/listing_photo 는 stub)
- 2 unit tests (Io / Protocol 에러 → Database 매핑); unique violation 은 T2-T4 통합 테스트"
git push
```

CI 그린 확인:
```bash
gh run list --branch main --limit 3
gh run watch <CI-run-id> --exit-status
```

3 워크플로우 모두 그린 (walking-skeleton 은 mock JWT 모드 그대로 통과 — integration test 단계 미추가 상태).

---
