# Walking Skeleton — End-to-End Vertical Slice

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.

**Goal:** Plan 2a 토대 위에서 *실제로 돌아가는* 수직 절단 1회 만든다. 외부 사용자 입장에서 "User 1개 등록 → DB 저장 → 조회"가 HTTP로 작동하는 것을 확인.

**Why now:** Plan 2a는 토대 (DB 스키마 + 값 객체) 완성. 하지만 *실제 기능 0개*. 수직 절단 1회로 모든 layer (HTTP → 도메인 → DB) 통합 검증한 후 Plan 2b-i 재개. "Walking Skeleton" 패턴.

**Scope (의도적 최소):**
- *오직 User Aggregate 1개*. Listing/ListingPhoto는 Plan 2b-i에서.
- *오직 2 endpoint:* `POST /users`, `GET /users/:id`.
- *No auth* (Zitadel JWT는 sub-project 3). HTTP 요청 그대로 받음.
- *No validation 추가* (도메인 invariant만 — `User::try_new`).
- *No 프론트엔드* — `curl`로 검증.

**Out of scope (이 plan):**
- Frontend, Auth, OpenTelemetry, error mapping, RFC 9457 Problem Details, observability, R2, V-World, ...

이건 *진짜 만들기* 아님 — *진짜 작동 확인*. 각 layer 미니멀 구현.

---

## Architecture

```
HTTP request
    ↓ Axum handler (services/api/src/users.rs)
    ↓ validate via User::try_new (domain)
    ↓ UserRepository trait (port)
    ↓ PgUserRepository (sqlx, crates/db)
    ↓ PostgreSQL ("user" 테이블, V001_01)
    ↑ User struct
    ↑ JSON response
```

5 task. 각 task = 1 layer.

---

## Task 1: User Aggregate (minimal)

**Files:**
- `crates/domain/core/user/Cargo.toml` (신규 BC crate)
- `crates/domain/core/user/src/lib.rs` (`pub mod entity; pub mod repository; pub mod errors;`)
- `crates/domain/core/user/src/entity.rs` — `User` struct + `try_new`
- `crates/domain/core/user/src/errors.rs` — `UserError` enum
- `crates/domain/core/user/src/repository.rs` — `UserRepository` trait + `RepoError` enum
- Modify: 루트 `Cargo.toml` workspace.members + workspace.dependencies (`async-trait = "0.1"`)
- Modify: `crates/shared-kernel/Cargo.toml` workspace path 변경 X (Plan 2b-i에서 이동)

**의도적 단순화:** Plan 2b-i Task 8/9의 *축소판*. WS 범위:
- 필드만 (메서드 X): `id`, `zitadel_sub`, `email`, `display_name`, `user_kind`, `created_at`, `updated_at`, `version`
- 도메인 메서드는 `try_new`, `as_str` 정도만
- soft-delete, business_verified, broker, roles 등 *전부 생략* (Plan 2b-i 본격)

**스펙 참조:** spec § 5.1 user 테이블 (lines 152-176) 中 *간소 필드만*.

```rust
pub struct User {
    pub id: Id<UserMarker>,
    pub zitadel_sub: String,            // 이번엔 String 그대로 (BoundedString 같은 wrapper 생략)
    pub email: Email,
    pub display_name: String,
    pub user_kind: UserKind,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

pub enum UserKind {
    Individual,
    Corporation,
}
```

`try_new`에서 검증: `display_name` 비어있지 않음 + ≤100자, `zitadel_sub` 비어있지 않음 + ≤255자.

**Repository trait:**

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError>;
    async fn save(&self, user: &User) -> Result<(), RepoError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("conflict (version mismatch)")]
    Conflict,
    #[error("database: {0}")]
    Database(String),
}
```

**Tests (≥8):**
- `try_new` happy path
- `display_name` 빈 문자열 거부
- `display_name` 101자 거부
- `zitadel_sub` 빈 문자열 거부
- `zitadel_sub` 256자 거부
- `UserKind::Individual` / `Corporation` Display + FromStr
- `version` 초기값 1
- serde JSON roundtrip

CI green 후 commit.

---

## Task 2: PgUserRepository (SQLx 구현)

**Files:**
- `crates/db/Cargo.toml` (신규 crate, 또는 기존이 있다면 사용)
- `crates/db/src/lib.rs` (`pub mod user;`)
- `crates/db/src/user.rs` — `PgUserRepository` 구현체
- Modify: 루트 `Cargo.toml` workspace.dependencies — sqlx 추가됐어야 함 (Task 2 이미 있음)

**스펙 참조:** spec § 8.1 line 802 — `crates/db/` 위치.

**구현:**

```rust
use sqlx::{PgPool, Row};
use shared_kernel::id::{Id, UserMarker};
use user_domain::entity::{User, UserKind};
use user_domain::errors::RepoError;
use user_domain::repository::UserRepository;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError> {
        let row = sqlx::query!(
            r#"
            select id, zitadel_sub, email, display_name, user_kind,
                   created_at, updated_at, version
            from "user"
            where id = $1 and deleted_at is null
            "#,
            id.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(row.map(|r| User {
            id: Id::try_from_str(&r.id).expect("DB schema enforces format"),
            zitadel_sub: r.zitadel_sub,
            email: Email::try_new(&r.email).expect("DB schema enforces format"),
            display_name: r.display_name,
            user_kind: match r.user_kind.as_str() {
                "individual" => UserKind::Individual,
                "corporation" => UserKind::Corporation,
                other => panic!("unexpected user_kind in DB: {other}"),
            },
            created_at: r.created_at,
            updated_at: r.updated_at,
            version: r.version,
        }))
    }

    async fn save(&self, user: &User) -> Result<(), RepoError> {
        let kind = match user.user_kind {
            UserKind::Individual => "individual",
            UserKind::Corporation => "corporation",
        };
        sqlx::query!(
            r#"
            insert into "user"
              (id, zitadel_sub, email, display_name, user_kind,
               phone_kr_hash, business_number, business_verified_at,
               broker_license_number, broker_verified_at, roles,
               nice_verified_at, marketing_consent_at,
               created_at, updated_at, last_login_at, deleted_at, version)
            values
              ($1, $2, $3, $4, $5,
               null, null, null, null, null, '{}',
               null, null,
               $6, $7, null, null, $8)
            on conflict (id) do update set
                email = excluded.email,
                display_name = excluded.display_name,
                user_kind = excluded.user_kind,
                updated_at = excluded.updated_at,
                version = "user".version + 1
            where "user".version = $8
            "#,
            user.id.as_str(),
            user.zitadel_sub,
            user.email.as_str(),
            user.display_name,
            kind,
            user.created_at,
            user.updated_at,
            user.version,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }
}
```

**`expect()` 처리:** DB 스키마가 강제하는 invariant라 도달 불가능. workspace lints `expect_used = "deny"` 회피 위해 `#[allow(clippy::expect_used)]` + `# Panics` rustdoc.

**테스트:** sqlx의 `query!` 매크로는 *컴파일 타임 DB 검증* 필요 → Plan 2a가 마련한 `DATABASE_URL` 활용. `cargo build`만 통과해도 SQL 문법 + 컬럼 매핑 검증됨.

**런타임 테스트 (CI):**
- 새 워크플로우 잡 (또는 db-migrations 잡 확장) — Postgres + 마이그 적용 + `cargo test --package db --features integration` 같은 패턴
- 또는 Task 5에서 `curl` E2E로 검증

이 task에서는 구현 + cargo build 통과만. 통합 테스트는 Task 5.

CI green 후 commit.

---

## Task 3: Axum HTTP server skeleton

**Files:**
- `services/api/Cargo.toml` (신규)
- `services/api/src/main.rs` — Axum app + DB pool + 2 endpoint
- Modify: 루트 `Cargo.toml` workspace.members
- Modify: `pnpm-workspace.yaml` 변경 X (services/api는 Rust binary)
- Modify: `infrastructure/docker/docker-compose.yml` 변경 X (DB만 Docker)

**Cargo.toml (services/api):**
```toml
[package]
name = "api"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 HTTP API service (Walking Skeleton)"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
sqlx = { workspace = true }
serde = { workspace = true }
serde_json = "1"
chrono = { workspace = true }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
async-trait = { workspace = true }
shared-kernel = { path = "../../crates/shared-kernel" }
user-domain = { path = "../../crates/domain/core/user" }
db = { path = "../../crates/db" }

[lints]
workspace = true
```

**main.rs (~80줄):**

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::time::now_utc;
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

#[derive(Clone)]
struct AppState {
    user_repo: Arc<dyn UserRepository>,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    zitadel_sub: String,
    email: String,
    display_name: String,
    user_kind: String,  // "individual" | "corporation"
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    zitadel_sub: String,
    email: String,
    display_name: String,
    user_kind: String,
    created_at: String,
    version: i64,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.as_str().to_owned(),
            zitadel_sub: u.zitadel_sub,
            email: u.email.as_str().to_owned(),
            display_name: u.display_name,
            user_kind: match u.user_kind {
                UserKind::Individual => "individual".to_owned(),
                UserKind::Corporation => "corporation".to_owned(),
            },
            created_at: u.created_at.to_rfc3339(),
            version: u.version,
        }
    }
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, String)> {
    let email = Email::try_new(&req.email).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let kind = match req.user_kind.as_str() {
        "individual" => UserKind::Individual,
        "corporation" => UserKind::Corporation,
        _ => return Err((StatusCode::BAD_REQUEST, "user_kind must be individual|corporation".into())),
    };
    let now = now_utc();
    let user = User::try_new(
        Id::new(),
        req.zitadel_sub,
        email,
        req.display_name,
        kind,
        now,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    state.user_repo.save(&user).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;

    Ok((StatusCode::CREATED, Json(user.into())))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let id = Id::<UserMarker>::try_from_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let user = state.user_repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".into()))?;
    Ok(Json(user.into()))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool));
    let state = AppState { user_repo };

    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**`expect`/`unwrap` in main:** *허용*. main은 init failure 시 panic이 정답. lint `#[allow(clippy::expect_used, clippy::unwrap_used)]` 모듈 단위.

CI green 후 commit.

---

## Task 4: Walking Skeleton 통합 테스트 (CI)

**Files:**
- `.github/workflows/walking-skeleton.yml` (신규)
- `tests/walking-skeleton/smoke.sh`

**워크플로우:**
1. PG17+PostGIS 서비스 컨테이너 기동
2. sqlx-cli 설치 + 마이그 적용
3. `cargo build --package api`
4. `services/api/target/debug/api &` 백그라운드 실행
5. `tests/walking-skeleton/smoke.sh` — curl POST + GET 검증

**smoke.sh:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Wait for server
for i in {1..30}; do
    curl -sf http://localhost:8080/healthz >/dev/null 2>&1 && break || true
    sleep 1
done

# 1. Create user
RESPONSE=$(curl -sf -X POST http://localhost:8080/users \
    -H 'content-type: application/json' \
    -d '{"zitadel_sub":"test-sub-1","email":"alice@example.com","display_name":"Alice","user_kind":"individual"}')

USER_ID=$(echo "$RESPONSE" | grep -oE '"id":"[^"]+' | cut -d'"' -f4)
if [ -z "$USER_ID" ]; then
    echo "FAIL: no id in response: $RESPONSE" >&2
    exit 1
fi
echo "Created user: $USER_ID"

# 2. Get user
GET_RESPONSE=$(curl -sf "http://localhost:8080/users/$USER_ID")
EMAIL=$(echo "$GET_RESPONSE" | grep -oE '"email":"[^"]+' | cut -d'"' -f4)
if [ "$EMAIL" != "alice@example.com" ]; then
    echo "FAIL: email mismatch: $EMAIL" >&2
    exit 1
fi
echo "PASS: round-trip works (id=$USER_ID, email=$EMAIL)"
```

`/healthz` endpoint 추가 — Task 3 main.rs에 `Router::new().route("/healthz", get(|| async { "ok" }))` 한 줄.

CI green = *진짜 작동 확인 완료*.

---

## Task 5: 로컬 검증 + 사용자에게 시연

이 task는 *코드 작업 0줄*. 사용자가 직접 실행해보는 단계:

```bash
# 1. Docker 기동
docker compose -f infrastructure/docker/docker-compose.yml up -d

# 2. 마이그 적용
bash scripts/sqlx-migrate.sh

# 3. API 서버 기동 (별도 터미널)
cd services/api && DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:5432/gongzzang cargo run

# 4. 사용자 등록
curl -X POST http://localhost:8080/users \
    -H 'content-type: application/json' \
    -d '{"zitadel_sub":"test-1","email":"alice@example.com","display_name":"Alice","user_kind":"individual"}'

# → {"id":"usr_01HXY...","zitadel_sub":"test-1",...}

# 5. 조회
curl http://localhost:8080/users/usr_01HXY...
```

**사용자 입장에서:** "처음으로 작동하는 거 확인". 화면은 없지만 *진짜 데이터가 PostgreSQL에 저장되고 다시 읽힘*.

---

## 완료 기준

- [ ] CI green (기존 + walking-skeleton 워크플로우)
- [ ] 사용자 로컬 시연 성공 (curl POST + GET round-trip)
- [ ] 누적 코드: ~250줄 (User Aggregate 80 + PgUserRepository 60 + Axum main 100 + tests 50)

이 5 task 후 Plan 2b-i 재개. 토대 작동 확인됨 → 동기 부여 + foundation 검증.

## 위험

1. **Workspace 위치 변경** — Plan 2b-i Task 1에서 `crates/shared-kernel` → `crates/domain/core/shared-kernel` 이동 예정. WS는 *기존 위치*에서 시작. 이후 2b-i Task 1에서 함께 이동.
2. **services/api/ 위치** — AGENTS.md §2 참조. workspace.members 추가 필요.
3. **sqlx offline mode** — `query!` 매크로가 컴파일 타임에 DB 연결 필요. `DATABASE_URL` 환경변수 또는 `.sqlx/` cache 디렉토리 (sqlx-cli prepare). CI에서는 서비스 컨테이너로 해결.
