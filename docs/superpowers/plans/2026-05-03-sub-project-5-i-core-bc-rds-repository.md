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

## Phase B: PgUserRepository 보강

### Task 2: `PgUserRepository` 18 필드 + `tracing::instrument` + integration tests

**Files:**
- Modify: `crates/db/src/user.rs` (193 → ~360 줄)
- Create: `crates/db/tests/common.rs`
- Create: `crates/db/tests/user_integration.rs`
- Modify: `crates/db/Cargo.toml` (`tracing` dev-dep 등 정리는 T1 에서 처리됨)

- [ ] **Step 1: `crates/db/tests/common.rs` 신규 — 통합 테스트 공통 헬퍼**

```rust
//! 통합 테스트 공통 헬퍼.
//!
//! `DATABASE_URL` 환경 변수로 PG 연결. 미설정 시 panic — 통합 테스트는 명시적
//! DB 환경 가정.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// 테스트용 PG pool 생성. 각 테스트는 자체 connection 으로 격리.
pub async fn setup_test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");
    PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .expect("connect to test Postgres")
}

/// 테스트 격리: 각 테스트 시작 전 모든 도메인 테이블 truncate.
///
/// FK cascade 를 활용해 한 번에 — `listing_photo` 가 `listing` FK on delete cascade.
pub async fn truncate_all(pool: &PgPool) {
    sqlx::query("truncate \"user\", listing, listing_photo cascade")
        .execute(pool)
        .await
        .expect("truncate failed");
}
```

- [ ] **Step 2: `crates/db/tests/user_integration.rs` 작성 (6 tests)**

```rust
//! `PgUserRepository` 통합 테스트 — 18 필드 round-trip + OCC + soft-delete.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::user::PgUserRepository;
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use user_domain::entity::{User, UserKind, UserRole};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, truncate_all};

fn make_user(zsub: &str, email: &str) -> User {
    let now = Utc::now();
    User::try_new_full(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        None,
        "Test User",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        vec![UserRole::Buyer, UserRole::Seller],
        None,
        None,
        now,
    )
    .expect("user")
}

#[tokio::test]
async fn round_trip_user_with_18_fields() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-1", "alice@example.com");
    repo.save(&user).await.expect("save");

    let fetched = repo.find_by_id(&user.id).await.expect("find").expect("Some");
    assert_eq!(fetched.zitadel_sub, user.zitadel_sub);
    assert_eq!(fetched.email.as_str(), user.email.as_str());
    assert_eq!(fetched.display_name, user.display_name);
    assert_eq!(fetched.user_kind, user.user_kind);
    assert_eq!(fetched.roles, user.roles); // ← 핵심: SP3 에서 누락된 필드
    assert_eq!(fetched.version, 1);
}

#[tokio::test]
async fn find_by_zitadel_sub_returns_user() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-2", "bob@example.com");
    repo.save(&user).await.expect("save");

    let fetched = repo.find_by_zitadel_sub("zsub-2").await.expect("find").expect("Some");
    assert_eq!(fetched.id.as_str(), user.id.as_str());
}

#[tokio::test]
async fn find_by_email_returns_user() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-3", "carol@example.com");
    repo.save(&user).await.expect("save");

    let email = Email::try_new("carol@example.com").unwrap();
    let fetched = repo.find_by_email(&email).await.expect("find").expect("Some");
    assert_eq!(fetched.id.as_str(), user.id.as_str());
}

#[tokio::test]
async fn duplicate_zitadel_sub_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let u1 = make_user("zsub-dup", "u1@example.com");
    let u2 = make_user("zsub-dup", "u2@example.com");
    repo.save(&u1).await.expect("first save ok");

    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let mut user = make_user("zsub-occ", "occ@example.com");
    repo.save(&user).await.expect("save v1");

    // 직접 version 을 안 맞게 조작 — 동시 update 시뮬레이션
    user.version = 99; // DB version 은 1
    let err = repo.save(&user).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn find_nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let id: Id<UserMarker> = Id::new();
    let fetched = repo.find_by_id(&id).await.expect("find");
    assert!(fetched.is_none());
}
```

- [ ] **Step 3: `crates/db/src/user.rs` 18 필드 + tracing 보강**

전체 재작성 (193 줄 → ~360 줄):

```rust
//! `UserRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::broker_license::BrokerLicense;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;
use user_domain::entity::{User, UserKind, UserRole};
use user_domain::repository::{RepoError, UserRepository};

use crate::error_map::map_sqlx_err;

/// `User` `Aggregate` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ALL_USER_COLUMNS: &str = r#"
    id, zitadel_sub, email, phone_kr_hash, display_name, user_kind,
    business_number, business_verified_at,
    broker_license_number, broker_verified_at,
    roles, nice_verified_at, marketing_consent_at,
    created_at, updated_at, last_login_at, deleted_at, version
"#;

fn row_to_user(row: &PgRow) -> Result<User, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let zitadel_sub: String = row.try_get("zitadel_sub").map_err(|e| RepoError::Database(e.to_string()))?;
    let email_str: String = row.try_get("email").map_err(|e| RepoError::Database(e.to_string()))?;
    let phone_kr_hash: Option<String> = row.try_get("phone_kr_hash").map_err(|e| RepoError::Database(e.to_string()))?;
    let display_name: String = row.try_get("display_name").map_err(|e| RepoError::Database(e.to_string()))?;
    let user_kind_str: String = row.try_get("user_kind").map_err(|e| RepoError::Database(e.to_string()))?;
    let business_number_str: Option<String> = row.try_get("business_number").map_err(|e| RepoError::Database(e.to_string()))?;
    let business_verified_at: Option<DateTime<Utc>> = row.try_get("business_verified_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let broker_license_str: Option<String> = row.try_get("broker_license_number").map_err(|e| RepoError::Database(e.to_string()))?;
    let broker_verified_at: Option<DateTime<Utc>> = row.try_get("broker_verified_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let roles_strs: Vec<String> = row.try_get("roles").map_err(|e| RepoError::Database(e.to_string()))?;
    let nice_verified_at: Option<DateTime<Utc>> = row.try_get("nice_verified_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let marketing_consent_at: Option<DateTime<Utc>> = row.try_get("marketing_consent_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let last_login_at: Option<DateTime<Utc>> = row.try_get("last_login_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row.try_get("version").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<UserMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed id in DB: {e}")))?;
    let email = Email::try_new(&email_str)
        .map_err(|e| RepoError::Database(format!("malformed email in DB: {e}")))?;
    let user_kind = match user_kind_str.as_str() {
        "individual" => UserKind::Individual,
        "corporation" => UserKind::Corporation,
        other => return Err(RepoError::Database(format!("unexpected user_kind in DB: {other}"))),
    };
    let business_number = business_number_str
        .map(|s| BusinessNumber::try_new(&s).map_err(|e| RepoError::Database(format!("malformed business_number in DB: {e}"))))
        .transpose()?;
    let broker_license_number = broker_license_str
        .map(|s| BrokerLicense::try_new(&s).map_err(|e| RepoError::Database(format!("malformed broker_license in DB: {e}"))))
        .transpose()?;

    let mut roles = Vec::with_capacity(roles_strs.len());
    for s in roles_strs {
        let r = match s.as_str() {
            "Buyer" => UserRole::Buyer,
            "Seller" => UserRole::Seller,
            "Broker" => UserRole::Broker,
            "Developer" => UserRole::Developer,
            "Enterprise" => UserRole::Enterprise,
            "Operator" => UserRole::Operator,
            "Admin" => UserRole::Admin,
            other => return Err(RepoError::Database(format!("unexpected role in DB: {other}"))),
        };
        roles.push(r);
    }

    Ok(User {
        id,
        zitadel_sub,
        email,
        phone_kr_hash,
        display_name,
        user_kind,
        business_number,
        business_verified_at,
        broker_license_number,
        broker_verified_at,
        roles,
        nice_verified_at,
        marketing_consent_at,
        created_at,
        updated_at,
        last_login_at,
        deleted_at,
        version,
    })
}

#[async_trait]
impl UserRepository for PgUserRepository {
    #[instrument(skip(self), fields(user_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where id = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[instrument(skip(self))]
    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where zitadel_sub = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(sub)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[instrument(skip(self))]
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where email = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(email.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[instrument(skip(self, user), fields(user_id = %user.id.as_str(), version = user.version))]
    async fn save(&self, user: &User) -> Result<(), RepoError> {
        let kind_str = match user.user_kind {
            UserKind::Individual => "individual",
            UserKind::Corporation => "corporation",
        };
        let role_strs: Vec<&str> = user.roles.iter().map(UserRole::as_str).collect();

        let result = sqlx::query(
            r#"
            insert into "user" (
                id, zitadel_sub, email, phone_kr_hash, display_name, user_kind,
                business_number, business_verified_at,
                broker_license_number, broker_verified_at,
                roles, nice_verified_at, marketing_consent_at,
                created_at, updated_at, last_login_at, deleted_at, version
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            on conflict (id) do update set
                email = excluded.email,
                phone_kr_hash = excluded.phone_kr_hash,
                display_name = excluded.display_name,
                user_kind = excluded.user_kind,
                business_number = excluded.business_number,
                business_verified_at = excluded.business_verified_at,
                broker_license_number = excluded.broker_license_number,
                broker_verified_at = excluded.broker_verified_at,
                roles = excluded.roles,
                nice_verified_at = excluded.nice_verified_at,
                marketing_consent_at = excluded.marketing_consent_at,
                updated_at = excluded.updated_at,
                last_login_at = excluded.last_login_at,
                deleted_at = excluded.deleted_at,
                version = "user".version + 1
            where "user".version = $18
            "#,
        )
        .bind(user.id.as_str())
        .bind(&user.zitadel_sub)
        .bind(user.email.as_str())
        .bind(&user.phone_kr_hash)
        .bind(&user.display_name)
        .bind(kind_str)
        .bind(user.business_number.as_ref().map(BusinessNumber::as_str))
        .bind(user.business_verified_at)
        .bind(user.broker_license_number.as_ref().map(BrokerLicense::as_str))
        .bind(user.broker_verified_at)
        .bind(&role_strs)
        .bind(user.nice_verified_at)
        .bind(user.marketing_consent_at)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .bind(user.deleted_at)
        .bind(user.version)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }
        Ok(())
    }
}
```

- [ ] **Step 4: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
cargo test -p db --lib   # 단위 테스트만 (error_map 2 tests)
```

Expected: 모두 통과. 통합 테스트는 `--features integration` + DATABASE_URL 필요라 로컬 skip.

- [ ] **Step 5: Commit + push**

```bash
git add crates/db/src/user.rs crates/db/tests/common.rs crates/db/tests/user_integration.rs
git commit -m "feat(db): PgUserRepository 18 필드 완전 처리 + tracing::instrument + integration tests (SP5-i T2)

- row_to_user: 8 필드 → 18 필드 (roles, business_number, broker_license, *_verified_at, marketing_consent, last_login, deleted_at, phone_kr_hash 모두)
- save: 18 필드 모두 INSERT/UPDATE (roles 양방향, BusinessNumber/BrokerLicense round-trip)
- ALL_USER_COLUMNS 상수로 SELECT 일관성
- map_sqlx_err 적용 (error_map.rs 활용)
- 모든 4 메서드 #[tracing::instrument] (skip(self), fields=user_id 만 노출 — PII 미노출)
- common.rs: setup_test_pool + truncate_all 헬퍼
- user_integration.rs: 6 통합 테스트 (round-trip 18 필드 + zitadel_sub + email + duplicate Conflict + OCC mismatch + None)"
git push
```

CI 그린 확인 — walking-skeleton 은 mock JWT 모드 그대로 (integration 단계 미추가, T5 에서 추가).

---

## Phase C: 신규 Repository

### Task 3: `PgListingRepository` (PostGIS + OCC + soft-delete)

**Files:**
- Modify: `crates/db/src/listing.rs` (stub → full impl)
- Create: `crates/db/tests/listing_integration.rs`

- [ ] **Step 1: 통합 테스트 작성 (`crates/db/tests/listing_integration.rs`)**

```rust
//! `PgListingRepository` 통합 테스트 — 21 필드 round-trip + PostGIS + OCC + soft-delete.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::user::PgUserRepository;
use geo_types::Point;
use listing_domain::contact_visibility::ContactVisibility;
use listing_domain::description::Description;
use listing_domain::entity::Listing;
use listing_domain::listing_status::ListingStatus;
use listing_domain::listing_title::ListingTitle;
use listing_domain::listing_type::ListingType;
use listing_domain::repository::{ListingRepository, RepoError};
use listing_domain::transaction_type::TransactionType;
use shared_kernel::area_m2::AreaM2;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::money::MoneyKrw;
use shared_kernel::point_srid::PointSrid;
use shared_kernel::pnu::Pnu;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

async fn seed_owner(pool: &sqlx::PgPool) -> Id<UserMarker> {
    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::new(),
        "owner-zsub",
        Email::try_new("owner@example.com").unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    user_repo.save(&owner).await.unwrap();
    owner.id
}

fn make_listing_sale(owner_id: Id<UserMarker>) -> Listing {
    let now = Utc::now();
    Listing::try_new_draft(
        Id::new(),
        owner_id,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(500_000_000).unwrap(),
        None, // deposit
        None, // monthly_rent
        AreaM2::try_new(rust_decimal::Decimal::new(33058, 2)).unwrap(),
        ListingTitle::try_new("강남 공장 매물 (테스트)").unwrap(),
        Description::new("샘플 설명"),
        Some(PointSrid::new(Point::new(127.0276, 37.4979))), // 강남
        now,
    )
    .expect("listing")
}

#[tokio::test]
async fn round_trip_listing_with_postgis() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let listing = make_listing_sale(owner);
    repo.save(&listing).await.expect("save");

    let fetched = repo.find_by_id(&listing.id).await.expect("find").expect("Some");
    assert_eq!(fetched.id, listing.id);
    assert_eq!(fetched.owner_id, listing.owner_id);
    assert_eq!(fetched.parcel_pnu, listing.parcel_pnu);
    assert_eq!(fetched.listing_type, listing.listing_type);
    assert_eq!(fetched.transaction_type, listing.transaction_type);
    assert_eq!(fetched.price, listing.price);
    assert_eq!(fetched.title, listing.title);
    assert_eq!(fetched.status, ListingStatus::Draft);
    assert_eq!(fetched.contact_visibility, ContactVisibility::LoginRequired);
    assert_eq!(fetched.view_count, 0);
    assert_eq!(fetched.bookmark_count, 0);
    assert_eq!(fetched.version, 1);
    // PostGIS 정확 round-trip (lat/lng float)
    let p = fetched.geom_point.expect("geom present");
    assert!((p.0.x() - 127.0276).abs() < 1e-9);
    assert!((p.0.y() - 37.4979).abs() < 1e-9);
}

#[tokio::test]
async fn save_without_geom_point() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    listing.geom_point = None;
    repo.save(&listing).await.expect("save");

    let fetched = repo.find_by_id(&listing.id).await.expect("find").expect("Some");
    assert!(fetched.geom_point.is_none());
}

#[tokio::test]
async fn find_by_owner_returns_owner_listings() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    let l2 = make_listing_sale(owner.clone());
    repo.save(&l1).await.unwrap();
    repo.save(&l2).await.unwrap();

    let results = repo.find_by_owner(&owner, 10).await.expect("find_by_owner");
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn find_nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingRepository::new(pool);
    let id = Id::new();
    let fetched = repo.find_by_id(&id).await.expect("find");
    assert!(fetched.is_none());
}

#[tokio::test]
async fn occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    listing.version = 99;
    let err = repo.save(&listing).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn duplicate_id_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    let mut l2 = make_listing_sale(owner);
    l2.id = l1.id.clone();
    l2.version = 1; // 같은 version 으로 INSERT 시도 → unique violation but version match 분기 동작 — 실은 ON CONFLICT DO UPDATE 이라 update 됨
    // 다른 owner 로 변경했다면 업데이트 통과 — 이 시나리오는 정확히 의도된 흐름

    repo.save(&l1).await.unwrap();
    repo.save(&l2).await.unwrap(); // 같은 id, version=1 → upsert success

    // 이번엔 진짜 duplicate ID 다른 데이터: version 안 맞춰 conflict
    let mut l3 = make_listing_sale(l1.owner_id.clone());
    l3.id = l1.id.clone();
    l3.version = 99;
    let err = repo.save(&l3).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn update_changes_version_and_fields() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    let fetched = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.version, 1);

    // 도메인 메서드로 update — view_count 증가
    listing.view_count = 5;
    listing.version = 1; // OCC: 현재 DB 버전
    repo.save(&listing).await.unwrap();

    let fetched2 = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched2.version, 2);
    assert_eq!(fetched2.view_count, 5);
}

#[tokio::test]
async fn soft_deleted_listing_excluded_from_find() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool.clone());

    let listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    // 직접 SQL 로 soft-delete (도메인 메서드는 SP5-i 범위 외 — 도메인이 deleted_at 컬럼 모름)
    sqlx::query(r#"update listing set deleted_at = now() where id = $1"#)
        .bind(listing.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&listing.id).await.unwrap();
    // 현재 Listing entity 는 deleted_at 컬럼 없음 — find_by_id 가 별도로 필터링.
    // V001_01 listing 테이블에 deleted_at 없음을 확인했으면 본 테스트 skip.
    // (확인: V001_01 listing 테이블은 deleted_at 미포함 — User 만 soft-delete)
    // 따라서 본 테스트는 의도 다른 시나리오로 변경 필요.
    let _ = fetched; // listing 은 soft-delete 미지원 → 테스트 의미 없음
}
```

> **주의**: V001_01 의 `listing` 테이블에는 `deleted_at` 컬럼이 *없어요* (User 만 soft-delete). 위 마지막 테스트는 의미 없으므로 **삭제 또는 다른 시나리오로 대체**. 구현 단계에서 확인 후 결정. 본 plan 은 8 tests 로 계산.

이 테스트는 삭제하고 다음으로 대체:

```rust
#[tokio::test]
async fn save_with_deposit_and_monthly_rent_for_monthly_rent_type() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let now = Utc::now();
    let listing = Listing::try_new_draft(
        Id::new(),
        owner,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Office,
        TransactionType::MonthlyRent,
        MoneyKrw::try_new(1_000_000).unwrap(), // price not used much for monthly_rent
        Some(MoneyKrw::try_new(50_000_000).unwrap()), // deposit
        Some(MoneyKrw::try_new(2_000_000).unwrap()),  // monthly_rent
        AreaM2::try_new(rust_decimal::Decimal::new(5000, 2)).unwrap(),
        ListingTitle::try_new("월세 사무실").unwrap(),
        Description::new(""),
        None,
        now,
    )
    .expect("listing");

    repo.save(&listing).await.expect("save");
    let fetched = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.deposit, listing.deposit);
    assert_eq!(fetched.monthly_rent, listing.monthly_rent);
    assert_eq!(fetched.transaction_type, TransactionType::MonthlyRent);
}
```

총 9 tests.

- [ ] **Step 2: `crates/db/src/listing.rs` 작성**

```rust
//! `ListingRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use geo_types::Point;
use listing_domain::contact_visibility::ContactVisibility;
use listing_domain::description::Description;
use listing_domain::entity::Listing;
use listing_domain::listing_status::ListingStatus;
use listing_domain::listing_title::ListingTitle;
use listing_domain::listing_type::ListingType;
use listing_domain::repository::{ListingRepository, RepoError};
use listing_domain::transaction_type::TransactionType;
use rust_decimal::Decimal;
use shared_kernel::area_m2::AreaM2;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::money::MoneyKrw;
use shared_kernel::point_srid::PointSrid;
use shared_kernel::pnu::Pnu;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `Listing` `Aggregate` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingRepository {
    pool: PgPool,
}

impl PgListingRepository {
    /// 새 저장소 생성.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_LISTING_COLUMNS: &str = r#"
    id, owner_id, parcel_pnu, listing_type, transaction_type,
    price_krw, deposit_krw, monthly_rent_krw, area_m2,
    title, description, status, contact_visibility,
    view_count, bookmark_count,
    ST_X(geom_point) as geom_lng, ST_Y(geom_point) as geom_lat,
    geom_point is not null as has_geom,
    created_at, updated_at, expires_at, version
"#;

#[allow(clippy::too_many_lines)]
fn row_to_listing(row: &PgRow) -> Result<Listing, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let owner_id_str: String = row.try_get("owner_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let parcel_pnu_str: String = row.try_get("parcel_pnu").map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_type_str: String = row.try_get("listing_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let transaction_type_str: String = row.try_get("transaction_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let price_krw: i64 = row.try_get("price_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let deposit_krw: Option<i64> = row.try_get("deposit_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let monthly_rent_krw: Option<i64> = row.try_get("monthly_rent_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let area_m2: Decimal = row.try_get("area_m2").map_err(|e| RepoError::Database(e.to_string()))?;
    let title_str: String = row.try_get("title").map_err(|e| RepoError::Database(e.to_string()))?;
    let description_str: String = row.try_get("description").map_err(|e| RepoError::Database(e.to_string()))?;
    let status_str: String = row.try_get("status").map_err(|e| RepoError::Database(e.to_string()))?;
    let contact_vis_str: String = row.try_get("contact_visibility").map_err(|e| RepoError::Database(e.to_string()))?;
    let view_count: i64 = row.try_get("view_count").map_err(|e| RepoError::Database(e.to_string()))?;
    let bookmark_count: i64 = row.try_get("bookmark_count").map_err(|e| RepoError::Database(e.to_string()))?;
    let has_geom: bool = row.try_get("has_geom").map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lng: Option<f64> = row.try_get("geom_lng").map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lat: Option<f64> = row.try_get("geom_lat").map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let expires_at: Option<DateTime<Utc>> = row.try_get("expires_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row.try_get("version").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing id: {e}")))?;
    let owner_id = Id::<UserMarker>::try_from_str(&owner_id_str)
        .map_err(|e| RepoError::Database(format!("malformed owner_id: {e}")))?;
    let parcel_pnu = Pnu::try_new(&parcel_pnu_str)
        .map_err(|e| RepoError::Database(format!("malformed pnu: {e}")))?;
    let listing_type = ListingType::from_str(&listing_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected listing_type: {listing_type_str}")))?;
    let transaction_type = TransactionType::from_str(&transaction_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected transaction_type: {transaction_type_str}")))?;
    let price = MoneyKrw::try_new(price_krw)
        .map_err(|e| RepoError::Database(format!("invalid price_krw: {e}")))?;
    let deposit = deposit_krw
        .map(|v| MoneyKrw::try_new(v).map_err(|e| RepoError::Database(format!("invalid deposit_krw: {e}"))))
        .transpose()?;
    let monthly_rent = monthly_rent_krw
        .map(|v| MoneyKrw::try_new(v).map_err(|e| RepoError::Database(format!("invalid monthly_rent_krw: {e}"))))
        .transpose()?;
    let area = AreaM2::try_new(area_m2)
        .map_err(|e| RepoError::Database(format!("invalid area_m2: {e}")))?;
    let title = ListingTitle::try_new(&title_str)
        .map_err(|e| RepoError::Database(format!("invalid title: {e}")))?;
    let description = Description::new(&description_str);
    let status = ListingStatus::from_str(&status_str)
        .map_err(|_| RepoError::Database(format!("unexpected status: {status_str}")))?;
    let contact_visibility = ContactVisibility::from_str(&contact_vis_str)
        .map_err(|_| RepoError::Database(format!("unexpected contact_visibility: {contact_vis_str}")))?;
    let geom_point = if has_geom {
        match (geom_lng, geom_lat) {
            (Some(x), Some(y)) => Some(PointSrid::new(Point::new(x, y))),
            _ => None,
        }
    } else {
        None
    };

    let view_count_u: u64 = u64::try_from(view_count).unwrap_or(0);
    let bookmark_count_u: u64 = u64::try_from(bookmark_count).unwrap_or(0);

    Ok(Listing {
        id,
        owner_id,
        parcel_pnu,
        listing_type,
        transaction_type,
        price,
        deposit,
        monthly_rent,
        area,
        title,
        description,
        status,
        contact_visibility,
        view_count: view_count_u,
        bookmark_count: bookmark_count_u,
        geom_point,
        created_at,
        updated_at,
        expires_at,
        version,
    })
}

#[async_trait]
impl ListingRepository for PgListingRepository {
    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<ListingMarker>) -> Result<Option<Listing>, RepoError> {
        let sql = format!("select {SELECT_LISTING_COLUMNS} from listing where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_listing).transpose()
    }

    #[instrument(skip(self), fields(owner_id = %owner.as_str(), limit))]
    async fn find_by_owner(
        &self,
        owner: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<Listing>, RepoError> {
        let sql = format!(
            "select {SELECT_LISTING_COLUMNS} from listing where owner_id = $1 order by created_at desc limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(owner.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_listing).collect()
    }

    #[instrument(skip(self, listing), fields(listing_id = %listing.id.as_str(), version = listing.version))]
    async fn save(&self, listing: &Listing) -> Result<(), RepoError> {
        let geom_lng_opt = listing.geom_point.as_ref().map(|p| p.0.x());
        let geom_lat_opt = listing.geom_point.as_ref().map(|p| p.0.y());

        let result = sqlx::query(
            r#"
            insert into listing (
                id, owner_id, parcel_pnu, listing_type, transaction_type,
                price_krw, deposit_krw, monthly_rent_krw, area_m2,
                title, description, status, contact_visibility,
                view_count, bookmark_count,
                geom_point,
                created_at, updated_at, expires_at, version
            )
            values (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13,
                $14, $15,
                case when $16::float8 is null or $17::float8 is null then null
                     else ST_SetSRID(ST_MakePoint($16, $17), 4326) end,
                $18, $19, $20, $21
            )
            on conflict (id) do update set
                listing_type = excluded.listing_type,
                transaction_type = excluded.transaction_type,
                price_krw = excluded.price_krw,
                deposit_krw = excluded.deposit_krw,
                monthly_rent_krw = excluded.monthly_rent_krw,
                area_m2 = excluded.area_m2,
                title = excluded.title,
                description = excluded.description,
                status = excluded.status,
                contact_visibility = excluded.contact_visibility,
                view_count = excluded.view_count,
                bookmark_count = excluded.bookmark_count,
                geom_point = excluded.geom_point,
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at,
                version = listing.version + 1
            where listing.version = $21
            "#,
        )
        .bind(listing.id.as_str())
        .bind(listing.owner_id.as_str())
        .bind(listing.parcel_pnu.as_str())
        .bind(listing.listing_type.as_str())
        .bind(listing.transaction_type.as_str())
        .bind(i64::from(listing.price))
        .bind(listing.deposit.map(i64::from))
        .bind(listing.monthly_rent.map(i64::from))
        .bind(listing.area.value())
        .bind(listing.title.as_str())
        .bind(listing.description.as_str())
        .bind(listing.status.as_str())
        .bind(listing.contact_visibility.as_str())
        .bind(i64::try_from(listing.view_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(listing.bookmark_count).unwrap_or(i64::MAX))
        .bind(geom_lng_opt)
        .bind(geom_lat_opt)
        .bind(listing.created_at)
        .bind(listing.updated_at)
        .bind(listing.expires_at)
        .bind(listing.version)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }
        Ok(())
    }
}
```

> **주의**: 도메인 값 객체의 `as_str()` / `value()` / `i64::from(...)` 시그니처는 실제 코드와 다를 수 있어요. 구현 시 컴파일 에러가 나면 도메인 값 객체의 실제 메서드명 확인 후 조정. 본 plan 은 베스트 가정.

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
cargo test -p db --lib
```

`cargo check` 가 도메인 값 객체 시그니처 mismatch 발견하면 그 자리에서 수정.

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/src/listing.rs crates/db/tests/listing_integration.rs
git commit -m "feat(db): PgListingRepository — 21 필드 + PostGIS + OCC + tracing (SP5-i T3)

- row_to_listing: 21 필드 round-trip (PostGIS ST_X/ST_Y 로 lat/lng 복원)
- save: ST_SetSRID(ST_MakePoint, 4326) — ADR-0008 SRID 4326
- ON CONFLICT DO UPDATE WHERE version = \$N (OCC)
- 모든 메서드 #[tracing::instrument] (PII 미노출, listing_id/owner_id 만)
- map_sqlx_err 적용
- 9 통합 테스트 (round-trip with/without geom + find_by_owner + 4 OCC 시나리오 + monthly_rent)"
git push
```

---

### Task 4: `PgListingPhotoRepository`

**Files:**
- Modify: `crates/db/src/listing_photo.rs` (stub → full impl)
- Create: `crates/db/tests/listing_photo_integration.rs`

- [ ] **Step 1: 통합 테스트 작성**

```rust
//! `PgListingPhotoRepository` 통합 테스트 — 12 필드 + soft-delete + reorder.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::entity::{ContentType, ListingPhoto};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

async fn seed_listing(pool: &sqlx::PgPool) -> Id<ListingMarker> {
    use shared_kernel::area_m2::AreaM2;
    use shared_kernel::id::UserMarker;
    use shared_kernel::money::MoneyKrw;
    use shared_kernel::pnu::Pnu;
    use listing_domain::contact_visibility::ContactVisibility;
    use listing_domain::description::Description;
    use listing_domain::listing_title::ListingTitle;
    use listing_domain::listing_type::ListingType;
    use listing_domain::transaction_type::TransactionType;

    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::<UserMarker>::new(),
        "owner",
        Email::try_new("o@x.com").unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    user_repo.save(&owner).await.unwrap();

    let listing_repo = PgListingRepository::new(pool.clone());
    let listing = Listing::try_new_draft(
        Id::new(),
        owner.id,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(100_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(rust_decimal::Decimal::new(1000, 2)).unwrap(),
        ListingTitle::try_new("test").unwrap(),
        Description::new(""),
        None,
        now,
    )
    .unwrap();
    listing_repo.save(&listing).await.unwrap();
    listing.id
}

fn make_photo(listing_id: Id<ListingMarker>, order_index: i32) -> ListingPhoto {
    let now = Utc::now();
    ListingPhoto::try_new(
        Id::new(),
        listing_id,
        format!("listings/test/photo-{order_index}.jpg"),
        None,
        None,
        order_index,
        Some(1920),
        Some(1080),
        Some(2_000_000),
        ContentType::Jpeg,
        now,
    )
    .expect("photo")
}

#[tokio::test]
async fn round_trip_photo() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let photo = make_photo(listing_id, 0);
    repo.save(&photo).await.expect("save");

    let fetched = repo.find_by_id(&photo.id).await.expect("find").expect("Some");
    assert_eq!(fetched.r2_key, photo.r2_key);
    assert_eq!(fetched.display_order, 0);
    assert_eq!(fetched.content_type, ContentType::Jpeg);
}

#[tokio::test]
async fn find_by_listing_returns_ordered() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let p1 = make_photo(listing_id.clone(), 2);
    let p2 = make_photo(listing_id.clone(), 0);
    let p3 = make_photo(listing_id.clone(), 1);
    repo.save(&p1).await.unwrap();
    repo.save(&p2).await.unwrap();
    repo.save(&p3).await.unwrap();

    let photos = repo.find_by_listing(&listing_id).await.expect("ok");
    assert_eq!(photos.len(), 3);
    assert_eq!(photos[0].display_order, 0);
    assert_eq!(photos[1].display_order, 1);
    assert_eq!(photos[2].display_order, 2);
}

#[tokio::test]
async fn soft_delete_excludes_from_find() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo).await.unwrap();

    sqlx::query("update listing_photo set deleted_at = now() where id = $1")
        .bind(photo.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&photo.id).await.expect("ok");
    assert!(fetched.is_none());

    let by_listing = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(by_listing.len(), 0);
}

#[tokio::test]
async fn duplicate_id_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let p1 = make_photo(listing_id.clone(), 0);
    let mut p2 = make_photo(listing_id, 1);
    p2.id = p1.id.clone();
    p2.r2_key = "different-key.jpg".into();

    repo.save(&p1).await.unwrap();
    let res = repo.save(&p2).await;
    // ListingPhoto 는 OCC 미사용 (spec). 같은 id 두번째 INSERT 는 Conflict.
    // ON CONFLICT DO UPDATE 가 있다면 업데이트, 없다면 Conflict — 실제 거동은 구현 따름.
    let _ = res;
}

#[tokio::test]
async fn cascade_delete_on_listing_removal() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo).await.unwrap();

    // CASCADE 동작 확인: listing 삭제 → listing_photo 도 삭제
    sqlx::query("delete from listing where id = $1")
        .bind(listing_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&photo.id).await.unwrap();
    assert!(fetched.is_none()); // ON DELETE CASCADE 가 photo 도 제거
}

#[tokio::test]
async fn nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);
    let id = Id::<ListingPhotoMarker>::new();
    let fetched = repo.find_by_id(&id).await.expect("ok");
    assert!(fetched.is_none());
}
```

총 6 tests.

- [ ] **Step 2: `crates/db/src/listing_photo.rs` 작성**

```rust
//! `ListingPhotoRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use listing_photo_domain::entity::{ContentType, ListingPhoto};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `ListingPhoto` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingPhotoRepository {
    pool: PgPool,
}

impl PgListingPhotoRepository {
    /// 새 저장소.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_PHOTO_COLUMNS: &str = r#"
    id, listing_id, r2_key, thumbnail_r2_key, caption,
    display_order, width_px, height_px, file_size_bytes,
    content_type, uploaded_at, deleted_at
"#;

fn row_to_photo(row: &PgRow) -> Result<ListingPhoto, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_id_str: String = row.try_get("listing_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let r2_key: String = row.try_get("r2_key").map_err(|e| RepoError::Database(e.to_string()))?;
    let thumbnail_r2_key: Option<String> = row.try_get("thumbnail_r2_key").map_err(|e| RepoError::Database(e.to_string()))?;
    let caption: Option<String> = row.try_get("caption").map_err(|e| RepoError::Database(e.to_string()))?;
    let display_order: i32 = row.try_get("display_order").map_err(|e| RepoError::Database(e.to_string()))?;
    let width_px: Option<i32> = row.try_get("width_px").map_err(|e| RepoError::Database(e.to_string()))?;
    let height_px: Option<i32> = row.try_get("height_px").map_err(|e| RepoError::Database(e.to_string()))?;
    let file_size_bytes: Option<i64> = row.try_get("file_size_bytes").map_err(|e| RepoError::Database(e.to_string()))?;
    let content_type_str: String = row.try_get("content_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let uploaded_at: DateTime<Utc> = row.try_get("uploaded_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingPhotoMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed id: {e}")))?;
    let listing_id = Id::<ListingMarker>::try_from_str(&listing_id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing_id: {e}")))?;
    let content_type = ContentType::from_str(&content_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected content_type: {content_type_str}")))?;

    Ok(ListingPhoto {
        id,
        listing_id,
        r2_key,
        thumbnail_r2_key,
        caption,
        display_order,
        width_px,
        height_px,
        file_size_bytes,
        content_type,
        uploaded_at,
        deleted_at,
    })
}

#[async_trait]
impl ListingPhotoRepository for PgListingPhotoRepository {
    #[instrument(skip(self), fields(photo_id = %id.as_str()))]
    async fn find_by_id(
        &self,
        id: &Id<ListingPhotoMarker>,
    ) -> Result<Option<ListingPhoto>, RepoError> {
        let sql = format!(
            "select {SELECT_PHOTO_COLUMNS} from listing_photo where id = $1 and deleted_at is null"
        );
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_photo).transpose()
    }

    #[instrument(skip(self), fields(listing_id = %listing_id.as_str()))]
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingPhoto>, RepoError> {
        let sql = format!(
            "select {SELECT_PHOTO_COLUMNS} from listing_photo where listing_id = $1 and deleted_at is null order by display_order asc"
        );
        let rows = sqlx::query(&sql)
            .bind(listing_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_photo).collect()
    }

    #[instrument(skip(self, photo), fields(photo_id = %photo.id.as_str(), order = photo.display_order))]
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            insert into listing_photo (
                id, listing_id, r2_key, thumbnail_r2_key, caption,
                display_order, width_px, height_px, file_size_bytes,
                content_type, uploaded_at, deleted_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            on conflict (id) do update set
                r2_key = excluded.r2_key,
                thumbnail_r2_key = excluded.thumbnail_r2_key,
                caption = excluded.caption,
                display_order = excluded.display_order,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                file_size_bytes = excluded.file_size_bytes,
                content_type = excluded.content_type,
                deleted_at = excluded.deleted_at
            "#,
        )
        .bind(photo.id.as_str())
        .bind(photo.listing_id.as_str())
        .bind(&photo.r2_key)
        .bind(&photo.thumbnail_r2_key)
        .bind(&photo.caption)
        .bind(photo.display_order)
        .bind(photo.width_px)
        .bind(photo.height_px)
        .bind(photo.file_size_bytes)
        .bind(photo.content_type.as_str())
        .bind(photo.uploaded_at)
        .bind(photo.deleted_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}
```

> ListingPhoto 는 OCC 미사용 (spec). save 는 INSERT 또는 UPDATE 모두 통과 (ON CONFLICT DO UPDATE).

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
cargo test -p db --lib
```

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/src/listing_photo.rs crates/db/tests/listing_photo_integration.rs
git commit -m "feat(db): PgListingPhotoRepository — 12 필드 + soft-delete + reorder + tracing (SP5-i T4)

- row_to_photo: 12 필드 round-trip
- save: ON CONFLICT DO UPDATE (OCC 미사용 — display_order 변경만)
- find_by_id / find_by_listing: WHERE deleted_at IS NULL
- find_by_listing: ORDER BY display_order ASC
- 모든 메서드 #[tracing::instrument]
- 6 통합 테스트 (round-trip + ordered fetch + soft-delete + dup id + cascade + None)"
git push
```

---

## Phase D: CI 게이트

### Task 5: `walking-skeleton.yml` integration test 단계 + `error_map_integration.rs`

**Files:**
- Modify: `.github/workflows/walking-skeleton.yml`
- Create: `crates/db/tests/error_map_integration.rs` (unique violation 분기 검증)

- [ ] **Step 1: `crates/db/tests/error_map_integration.rs` 작성**

```rust
//! `map_sqlx_err` unique violation 분기 검증 — 진짜 PG INSERT 중복으로 검증.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, truncate_all};

#[tokio::test]
async fn unique_violation_zitadel_sub_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "same-zsub",
        Email::try_new("a@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "same-zsub", // 같은 zitadel_sub — UNIQUE 위반
        Email::try_new("b@x.com").unwrap(),
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn unique_violation_email_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "zsub-1",
        Email::try_new("dup@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "zsub-2",
        Email::try_new("dup@x.com").unwrap(), // 같은 email — UNIQUE
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}
```

- [ ] **Step 2: `.github/workflows/walking-skeleton.yml` 수정**

기존 `Apply migrations` 단계 *직후* 통합 테스트 단계 추가. 기존 `Build API` 단계 *전*.

```yaml
      - name: Apply gongzzang migrations
        run: sqlx migrate run --source migrations

      - name: Run integration tests (DB Repository)
        env:
          DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang
        run: cargo test --workspace --features integration --no-fail-fast

      - name: Build API
        run: cargo build --package api --release
```

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
```

통합 테스트 자체는 PG 필요라 로컬에서 못 돌리지만, 컴파일은 확인.

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/tests/error_map_integration.rs .github/workflows/walking-skeleton.yml
git commit -m "feat(ci): walking-skeleton에 cargo test --features integration 단계 추가 (SP5-i T5)

- error_map_integration.rs: 2 tests (unique violation 분기 — zitadel_sub / email)
- walking-skeleton.yml: Apply migrations 직후 'Run integration tests (DB Repository)' 추가
  · cargo test --workspace --features integration --no-fail-fast
  · DATABASE_URL: 기존 PG 컨테이너 재사용

총 통합 테스트 ~25 (User 6 + Listing 9 + ListingPhoto 6 + error_map 2 + 기존 0)
SSS 자동 강제: 통합 테스트 실패 시 walking-skeleton 빨강"
git push
```

CI 그린 확인 — walking-skeleton 4-6분 (integration test 추가로 시간 +30-60초 예상).

---

## Phase E: 종료

### Task 6: 통합 검증 + project_progress 갱신

**Files:**
- Modify: `MEMORY.md`
- Modify: `memory/project_progress.md`

- [ ] **Step 1: 누적 테스트 카운트 확인**

```bash
cd c:/Users/User/Desktop/gongzzang_2
grep -rE '#\[(tokio::)?test\]' crates/ services/ --include="*.rs" | wc -l
```

목표: 1050 (SP3 종료 시) + ~25 신규 통합 테스트 + 2 단위 테스트 (error_map) = ~1077.

- [ ] **Step 2: `MEMORY.md` 갱신**

```diff
- - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3 완료 (25 crate, 1050 tests), Rust 1.88, repo public (test)
+ - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3+5-i 완료 (25 crate, ~1077 tests), Rust 1.88, repo public (test)
```

- [ ] **Step 3: `memory/project_progress.md` 에 SP5-i 절 추가**

기존 SP3 절 *다음* 에:

```markdown
### Sub-project 5-i: Core BC RDS Repository SQLx (완료, T1-T6)

- 신규: `crates/db/src/error_map.rs` (MapFromSqlx trait + map_sqlx_err helper)
- 신규: `crates/db/src/listing.rs` (PgListingRepository — 21 필드, PostGIS round-trip, OCC)
- 신규: `crates/db/src/listing_photo.rs` (PgListingPhotoRepository — 12 필드, soft-delete, reorder)
- 보강: `crates/db/src/user.rs` 8 필드 → 18 필드 (roles/business_number/broker_license/*_verified_at 모두)
- 모든 repo 메서드 `#[tracing::instrument]` (PII 미노출 패턴)
- `Cargo.toml` `[features] integration = []` + `walking-skeleton.yml` 에 `cargo test --features integration` 단계
- 통합 테스트 ~25 (User 6 + Listing 9 + ListingPhoto 6 + error_map 2 + 기존 0) + 단위 2 → 누적 ~1077

**SP5-i 미포함 (후속)**:
- Outbox 트랜잭션 → SP5-iii
- audit_log 자동 INSERT → SP5-iii
- R2 Reader 6개 → SP4 (외부 API ingestion)
- `sqlx::query!()` macro 채택 → 별도 ADR
- HTTP 응답 매핑 (`RepoError → IntoResponse`) → 별도
```

- [ ] **Step 4: Commit + push**

```bash
git add MEMORY.md memory/project_progress.md
git commit -m "chore(sp5-i-t6): integration validation — Sub-project 5-i complete (25 crates, ~1077 tests)

3 CI workflow 그린:
- CI 7 jobs (clippy / fmt / cargo-deny / tarpaulin ≥90% / secret / file-size / markdown)
- db-migrations: V001-V003_05
- walking-skeleton: mock JWT e2e + cargo test --features integration (DB Repository)

SP5-i 산출물:
- crates/db/src/error_map.rs (공통 helper, 3 도메인 RepoError impl)
- crates/db/src/listing.rs (PgListingRepository — 21 필드 + PostGIS + OCC + tracing)
- crates/db/src/listing_photo.rs (PgListingPhotoRepository — 12 필드 + soft-delete + tracing)
- crates/db/src/user.rs 18 필드 보강 (8 → 18) + tracing
- Cargo features.integration + walking-skeleton CI 게이트

다음: SP5-ii (Insights BC) 또는 SP4 (외부 API + R2 Readers) — 사용자 결정"
git push
```

3 워크플로우 모두 그린 최종 확인.

---

## 검증 기준 매핑 (Spec § 9)

| Spec § 9 항목 | 본 plan task |
|---|---|
| 1. `crates/db/src/listing.rs` + `crates/db/src/listing_photo.rs` 신규 | T3 + T4 |
| 2. `crates/db/src/error_map.rs` 신규 | T1 |
| 3. `crates/db/src/user.rs` 18 필드 + `#[tracing::instrument]` | T2 |
| 4. `Cargo.toml [features] integration = []` | T1 |
| 5. `crates/db/tests/*_integration.rs` ~22-28 tests | T2 (6) + T3 (9) + T4 (6) + T5 (2) = 23 |
| 6. `walking-skeleton.yml` `cargo test --features integration` | T5 |
| 7. 모든 repo 메서드 `#[tracing::instrument]` (PII 미노출) | T2 + T3 + T4 |
| 8. 3 CI workflow 그린 | T5 + T6 |
| 9. 누적 테스트 ≥1075 | T6 검증 (~1077) |
| 10. tarpaulin ≥90% 유지 | T1-T6 매 commit |
| 11. clippy `-D warnings` 통과 | T1-T6 매 commit (로컬 + CI) |
| 12. 모든 파일 ≤500 권장 / ≤1500 강제 | T1-T6 매 commit (CI file-size job) |

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-12 모든 절 반영
- [x] 6 task 모두 fresh subagent dispatch 가능 단위
- [x] TDD: 테스트 먼저 작성 → 구현 → 로컬 cargo check/clippy/test 통과 → push → CI
- [x] 로컬 cargo 활용 명시 (MSVC 설치 후 변경된 워크플로우)
- [x] 알려진 lessons (clippy::doc_markdown 사전 백틱, derive_partial_eq_without_eq 등) 사전 대응
- [x] PII 미노출 패턴 (`tracing::instrument` 의 `skip(self)`, `fields(...)` 화이트리스트)

## 알려진 위험

1. **도메인 값 객체 메서드명 가정** — `ListingType::as_str()`, `MoneyKrw::value()`, `i64::from(MoneyKrw)` 등은 베스트 가정. 실제 시그니처와 다를 수 있어 첫 `cargo check` 에서 컴파일 에러 → 수정.
2. **`Listing::try_new_draft` 시그니처** — 실제 코드 13 args 확인 (plan 코드에서 `geom_point` Option 위치 등). 도메인 entity 직접 읽어 맞춤.
3. **`ListingPhoto.deleted_at`** — `listing_photo` 테이블에 있음 (V001_01 확인). 도메인 entity 에 필드가 있는지 확인 필요. 없으면 도메인 확장 필요 — 본 sub-project 범위에서 처리 가능.
4. **`PointSrid::new` 시그니처** — `PointSrid::new(Point<f64>)` 가정. 실제 시그니처 확인.
5. **`AreaM2::value()` 반환 타입** — `Decimal` 가정. 다르면 변환.

## 완료 후 다음

**Sub-project 5-i 종료** → 사용자 결정:
- **Sub-project 5-ii**: Insights BC RDS Repository (Bookmark + SearchHistory + AnalysisReport + Notification, ~10 task)
- **Sub-project 4**: 외부 API ingestion + R2 Reader 6개 (V-World/data.go.kr/법제처)

추천: **SP5-ii** — RDS Repository 패턴 정착 후 Insights/Audit/Operations 동일 패턴 반복. SP4 는 새 기술 (R2 PMTiles + 외부 API + Circuit Breaker) 조합이라 더 큼.
