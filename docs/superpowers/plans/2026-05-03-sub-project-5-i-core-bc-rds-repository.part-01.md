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

