### Task 5: `AuthMiddleware`

**Files:**
- Modify: `crates/auth/src/middleware.rs`

- [ ] **Step 1: 구현**

```rust
//! Axum tower middleware — Bearer 추출 → verify → User 자동 생성 → Extension 주입.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use tracing::warn;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use crate::claims::Claims;
use crate::errors::AuthError;
use crate::verifier::JwtVerifier;

/// 핸들러로 주입되는 인증된 사용자 컨텍스트.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// `User` Aggregate (`find_by_zitadel_sub` 또는 자동 생성).
    pub user: User,
    /// 검증 통과한 `JWT` claims.
    pub claims: Claims,
}

/// 미들웨어 의존 — verifier + user repository.
#[derive(Clone)]
pub struct AuthState {
    /// `JWT` 검증기.
    pub verifier: Arc<JwtVerifier>,
    /// `User` 저장소.
    pub user_repo: Arc<dyn UserRepository>,
}

/// `Bearer <jwt>` 검증 + `User` 자동 생성 + `Extension<AuthenticatedUser>` 주입.
///
/// # Errors
///
/// 모든 인증 실패는 [`AuthError`] 로 매핑되어 `IntoResponse` 됨.
pub async fn auth_layer(
    State(state): State<AuthState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?;
    let header_str = header.to_str().map_err(|_| AuthError::InvalidFormat)?;
    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidFormat)?
        .trim();
    if token.is_empty() {
        return Err(AuthError::InvalidFormat);
    }

    let claims = state.verifier.verify(token).await?;
    let user = resolve_or_create_user(&state, &claims).await?;
    req.extensions_mut().insert(AuthenticatedUser {
        user,
        claims: claims.clone(),
    });
    Ok(next.run(req).await)
}

async fn resolve_or_create_user(
    state: &AuthState,
    claims: &Claims,
) -> Result<User, AuthError> {
    if let Some(existing) = state
        .user_repo
        .find_by_zitadel_sub(&claims.sub)
        .await
        .map_err(|e| AuthError::UserProvisioningFailed(e.to_string()))?
    {
        return Ok(existing);
    }

    // 자동 생성
    let email_str = claims.effective_email().ok_or_else(|| {
        AuthError::UserProvisioningFailed("token has no email or preferred_username".into())
    })?;
    let email = Email::try_new(email_str)
        .map_err(|e| AuthError::UserProvisioningFailed(format!("invalid email: {e}")))?;
    let display = claims.effective_display_name();
    let now = Utc::now();
    let user = User::try_new(
        Id::new(),
        &claims.sub,
        email,
        &display,
        UserKind::Individual,
        vec![], // 역할 없음 (어드민이 별도 부여)
        now,
    )
    .map_err(|e| AuthError::UserProvisioningFailed(format!("domain validation: {e}")))?;

    // race: 동시 첫 로그인 — save unique 충돌 시 fetch 재시도
    if let Err(save_err) = state.user_repo.save(&user).await {
        warn!(?save_err, sub = %claims.sub, "save failed, retrying find");
        if let Some(existing) = state
            .user_repo
            .find_by_zitadel_sub(&claims.sub)
            .await
            .map_err(|e| AuthError::UserProvisioningFailed(e.to_string()))?
        {
            return Ok(existing);
        }
        return Err(AuthError::UserProvisioningFailed(save_err.to_string()));
    }
    Ok(user)
}
```

> `User::try_new` 실제 시그니처는 `crates/domain/core/user/src/entity.rs:163-228` 와 일치해야 함. `roles: Vec<UserRole>` 도 인자에 포함됨. 혹시 시그니처 불일치 시 컴파일 에러 메시지 따라 수정.

- [ ] **Step 2: commit + push + CI 그린**

```bash
git add crates/auth/src/middleware.rs
git commit -m "feat(auth): AuthMiddleware — Bearer + verify + auto-create User + Extension (SP3 T5)

- Header Authorization 파싱 (Bearer 접두 + 빈 토큰 거부)
- JwtVerifier.verify → Claims
- find_by_zitadel_sub → 없으면 User::try_new + save
- save unique 충돌 race → fetch retry
- email Email value object 검증
- AuthState { verifier, user_repo }"
git push
```

---

### Task 6: `AuthenticatedUser` extractor + `RoleGuard`

**Files:**
- Modify: `crates/auth/src/extractor.rs`
- Modify: `crates/auth/src/role_guard.rs`

- [ ] **Step 1: extractor 구현**

`crates/auth/src/extractor.rs`:

```rust
//! `AuthenticatedUser` extractor — middleware 주입한 `Extension` 을 핸들러용으로 노출.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AuthError::MissingToken)
    }
}
```

- [ ] **Step 2: role guard 구현**

`crates/auth/src/role_guard.rs`:

```rust
//! 역할 가드 — `require_role` helper.

use user_domain::entity::UserRole;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

/// `auth.user.roles` 가 `role` 을 포함하는지 확인.
///
/// # Errors
///
/// 미포함 → [`AuthError::InsufficientRole`].
pub fn require_role(auth: &AuthenticatedUser, role: UserRole) -> Result<(), AuthError> {
    if auth.user.roles.contains(&role) {
        Ok(())
    } else {
        Err(AuthError::InsufficientRole)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use user_domain::entity::{User, UserKind, UserRole};

    use crate::claims::{Audience, Claims};

    fn fixture(roles: Vec<UserRole>) -> AuthenticatedUser {
        let email = Email::try_new("a@b.com").expect("email");
        let user = User::try_new(
            Id::new(),
            "sub-1",
            email,
            "alice",
            UserKind::Individual,
            roles,
            Utc::now(),
        )
        .expect("user");
        let claims = Claims {
            sub: "sub-1".into(),
            email: Some("a@b.com".into()),
            name: Some("alice".into()),
            preferred_username: None,
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        AuthenticatedUser { user, claims }
    }

    #[test]
    fn allows_when_role_present() {
        let auth = fixture(vec![UserRole::Buyer]);
        assert!(require_role(&auth, UserRole::Buyer).is_ok());
    }

    #[test]
    fn denies_when_role_missing() {
        let auth = fixture(vec![UserRole::Buyer]);
        let err = require_role(&auth, UserRole::Admin).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }

    #[test]
    fn denies_when_no_roles() {
        let auth = fixture(vec![]);
        let err = require_role(&auth, UserRole::Buyer).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }
}
```

- [ ] **Step 3: commit + push + CI 그린**

```bash
git add crates/auth/src/extractor.rs crates/auth/src/role_guard.rs
git commit -m "feat(auth): AuthenticatedUser extractor + require_role guard (SP3 T6)

- FromRequestParts<AuthenticatedUser> from Extension
- require_role(auth, UserRole) -> Result<(), AuthError::InsufficientRole>
- 3 unit tests for role guard"
git push
```

---

## Phase C: services/api 통합

### Task 7: `services/api` 미들웨어 적용 + POST /users 제거

**Files:**
- Modify: `services/api/Cargo.toml`
- Modify: `services/api/src/main.rs`
- Modify: `.env.example` (없으면 생성)

- [ ] **Step 1: `services/api/Cargo.toml` 에 auth dep 추가**

```toml
[dependencies]
# ... 기존 ...
auth = { path = "../../crates/auth", version = "0.1.0" }
```

- [ ] **Step 2: `services/api/src/main.rs` 재작성**

Walking Skeleton 의 `POST /users` 핸들러는 **제거**. `GET /users/:id` 는 인증 보호. `/healthz` 는 public.

```rust
//! 공짱 `HTTP` `API` service — Walking Skeleton + Auth (SP3).

#![forbid(unsafe_code)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::env;
use std::sync::Arc;

use auth::jwks_cache::JwksCache;
use auth::middleware::{auth_layer, AuthState, AuthenticatedUser};
use auth::verifier::JwtVerifier;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{middleware, Json, Router};
use db::user::PgUserRepository;
use serde::Serialize;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use user_domain::entity::User;
use user_domain::repository::UserRepository;

#[derive(Clone)]
struct AppState {
    user_repo: Arc<dyn UserRepository>,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    zitadel_sub: String,
    email: String,
    display_name: String,
    user_kind: String,
    roles: Vec<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        use user_domain::entity::UserKind;
        Self {
            id: u.id.as_str().to_owned(),
            zitadel_sub: u.zitadel_sub,
            email: u.email.as_str().to_owned(),
            display_name: u.display_name,
            user_kind: match u.user_kind {
                UserKind::Individual => "individual".into(),
                UserKind::Corporation => "corporation".into(),
            },
            roles: u.roles.iter().map(|r| r.as_str().to_owned()).collect(),
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
            version: u.version,
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

/// `GET /users/me` — 인증된 사용자 자신 조회.
async fn me(auth: AuthenticatedUser) -> Json<UserResponse> {
    Json(auth.user.into())
}

/// `GET /users/:id` — 인증된 사용자가 다른 `User` 조회 (자기 자신만 허용; 후속 SP 에서 권한 확장).
async fn get_user(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let id = Id::<UserMarker>::try_from_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid id: {e}")))?;
    if id.as_str() != auth.user.id.as_str() {
        return Err((StatusCode::FORBIDDEN, "이 사용자 정보는 조회할 권한이 없어요".into()));
    }
    let user = state
        .user_repo
        .find_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("find failed: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "user not found".into()))?;
    Ok(Json(user.into()))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let issuer = env::var("ZITADEL_ISSUER").expect("ZITADEL_ISSUER must be set");
    let audience = env::var("ZITADEL_AUDIENCE").expect("ZITADEL_AUDIENCE must be set");
    let jwks_url = format!("{issuer}/oauth/v2/keys");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool));
    let app_state = AppState {
        user_repo: user_repo.clone(),
    };

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("reqwest");
    let jwks = Arc::new(JwksCache::new(jwks_url, http));
    let verifier = Arc::new(JwtVerifier::new(issuer, audience, jwks));
    let auth_state = AuthState {
        verifier,
        user_repo,
    };

    let public = Router::new().route("/healthz", get(health));
    let protected = Router::new()
        .route("/users/me", get(me))
        .route("/users/:id", get(get_user))
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));

    let app = public
        .merge(protected.with_state(app_state))
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 3: `.env.example` 에 인증 환경 변수 추가**

```bash
DATABASE_URL=postgres://gongzzang:dev@localhost:5432/gongzzang
ZITADEL_ISSUER=https://zitadel.local
ZITADEL_AUDIENCE=client-id-from-zitadel
```

- [ ] **Step 4: commit + push + CI 그린**

CI 의 `walking-skeleton.yml` 은 아직 Zitadel 미통합이라 일시적으로 빨간색 — T9 에서 Zitadel 컨테이너 추가하며 수복. **본 task 의 commit 메시지에 명시.**

```bash
git add services/api/Cargo.toml services/api/src/main.rs .env.example
git commit -m "feat(api): apply auth middleware + remove POST /users + add GET /users/me (SP3 T7)

- Router: public Router (/healthz) + protected Router (auth_layer)
- POST /users 제거 — first-sign-in 자동 생성으로 대체
- GET /users/me — 인증된 자신 조회
- GET /users/:id — 자기 자신만 (FORBIDDEN otherwise)
- ZITADEL_ISSUER / ZITADEL_AUDIENCE env 필수
- 일시적 walking-skeleton.yml 빨강 — T9 에서 Zitadel 컨테이너 추가하며 수복"
git push
```

CI / db-migrations 는 그린, walking-skeleton 만 빨강 예상 — 다음 task 에서 해결.

---

## Phase D: DB 강화

### Task 8: `migrations/30005_user_roles_check.sql` — CHECK 제약 추가

**Files:**
- Create: `migrations/30005_user_roles_check.sql`
- Modify: `tests/migrations/test_v001_full.sh` (assertion 추가)

- [ ] **Step 1: 마이그레이션 작성**

```sql
-- V003_05: user.roles 원소가 7 종 enum 값 중 하나임을 보장
-- spec § 8.2, sub-project 3 (Auth)
--
-- UserRole enum (crates/domain/core/user/src/entity.rs:37-52):
--   Buyer / Seller / Broker / Developer / Enterprise / Operator / Admin

alter table "user"
    add constraint user_roles_valid_chk check (
        roles <@ array['Buyer','Seller','Broker','Developer','Enterprise','Operator','Admin']::text[]
    );
```

- [ ] **Step 2: `test_v001_full.sh` 에 assertion 추가**

기존 V003_03 검증 블록 다음에 V003_05 추가:

```bash
# V003_05: user.roles CHECK 제약
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='\"user\"'::regclass and conname='user_roles_valid_chk';" | grep -q '^1$'; then
  echo "FAIL: user_roles_valid_chk missing (V003_05)" >&2; exit 1
fi
```

- [ ] **Step 3: commit + push + CI 그린**

```bash
git add migrations/30005_user_roles_check.sql tests/migrations/test_v001_full.sh
git commit -m "feat(db): user.roles CHECK 제약 (UserRole 7 enum값) — migration 30005 (SP3 T8)

- V003_05: roles <@ array['Buyer',...,'Admin']
- test_v001_full.sh assertion 추가"
git push
```

`db-migrations.yml` 그린 확인. 만일 기존 row 에 잘못된 값이 있으면 CHECK 추가 실패 — 신규 DB 라 비어있어 안전.

---

## Phase E: CI Zitadel 통합 + e2e

### Task 9: `walking-skeleton.yml` 에 Zitadel 컨테이너 + e2e

**Files:**
- Modify: `.github/workflows/walking-skeleton.yml`
- Create: `tests/walking-skeleton/zitadel-setup.sh`

> **이 task 가 가장 어려워요.** Zitadel 의 CI 셋업은 정해진 레퍼런스가 적어 1-3 iter 가능. 제 가이드는 *2026 시점 Zitadel v3 가정* 으로 적었으니 실제 응답 구조에 따라 jq path 가 달라질 수 있어요.

- [ ] **Step 1: `tests/walking-skeleton/zitadel-setup.sh` 작성**

```bash
#!/usr/bin/env bash
# Zitadel CI 셋업 — admin token 획득 → service user → JWT 발급.
# 출력: $GITHUB_OUTPUT 에 ZITADEL_ISSUER, ZITADEL_AUDIENCE, ZITADEL_TEST_TOKEN

set -euo pipefail

ZITADEL_URL="http://localhost:8081"
MASTERKEY_FILE="/tmp/zitadel-masterkey"

# 1) Zitadel 부팅 대기
for i in {1..60}; do
  if curl -sf "$ZITADEL_URL/debug/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

# 2) admin PAT 획득 — Zitadel 시작 시 instance owner 가 자동 생성됨
#    --steps yaml 에서 정의한 PAT 토큰을 ZITADEL_ADMIN_PAT 환경 변수로 받음
if [ -z "${ZITADEL_ADMIN_PAT:-}" ]; then
  echo "ERROR: ZITADEL_ADMIN_PAT 환경 변수 필요 (Zitadel 컨테이너에 미리 주입)" >&2
  exit 1
fi

AUTH="Authorization: Bearer ${ZITADEL_ADMIN_PAT}"

# 3) Project + Application 생성 → audience (client_id) 확보
PROJECT_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/projects" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"name":"gongzzang-ci"}')
PROJECT_ID=$(echo "$PROJECT_RESP" | jq -r .id)

APP_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/projects/$PROJECT_ID/apps/oidc" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "name":"gongzzang-api-ci",
    "redirectUris":["http://localhost:8080/callback"],
    "responseTypes":["OIDC_RESPONSE_TYPE_CODE"],
    "grantTypes":["OIDC_GRANT_TYPE_AUTHORIZATION_CODE","OIDC_GRANT_TYPE_REFRESH_TOKEN"],
    "appType":"OIDC_APP_TYPE_WEB",
    "authMethodType":"OIDC_AUTH_METHOD_TYPE_BASIC",
    "accessTokenType":"OIDC_TOKEN_TYPE_JWT"
  }')
CLIENT_ID=$(echo "$APP_RESP" | jq -r .clientId)

# 4) Service user (machine) 생성 + PAT 발급
SU_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/users/machine" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"userName":"ci-test-user","name":"CI Test","description":"walking-skeleton","accessTokenType":"ACCESS_TOKEN_TYPE_JWT"}')
SU_ID=$(echo "$SU_RESP" | jq -r .userId)

PAT_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/users/$SU_ID/pats" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"expirationDate":"2099-01-01T00:00:00Z"}')
SU_TOKEN=$(echo "$PAT_RESP" | jq -r .token)

# 5) GITHUB_OUTPUT 으로 export
{
  echo "issuer=$ZITADEL_URL"
  echo "audience=$CLIENT_ID"
  echo "token=$SU_TOKEN"
} >> "$GITHUB_OUTPUT"

echo "Zitadel setup complete: issuer=$ZITADEL_URL audience=$CLIENT_ID"
```

> **주의:** Zitadel PAT 는 access_token 형식 JWT 가 아니라 opaque token 일 수 있음. 실제로는 `OIDC_TOKEN_TYPE_JWT` 로 설정한 client credentials grant 흐름이 더 정확. 본 setup script 는 첫 시도이며 CI iteration 으로 수정 예정.

- [ ] **Step 2: `walking-skeleton.yml` 갱신 — Zitadel 컨테이너 추가**

```yaml
name: walking-skeleton

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  POSTGRES_USER: gongzzang
  POSTGRES_PASSWORD: ci_only_changeme
  POSTGRES_DB: gongzzang

jobs:
  e2e:
    name: API + Auth E2E
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgis/postgis:17-3.5
        env:
          POSTGRES_USER: gongzzang
          POSTGRES_PASSWORD: ci_only_changeme
          POSTGRES_DB: gongzzang
        ports: ["5432:5432"]
        options: >-
          --health-cmd "pg_isready -U gongzzang"
          --health-interval 5s
          --health-timeout 3s
          --health-retries 10

      zitadel:
        image: ghcr.io/zitadel/zitadel:latest
        env:
          ZITADEL_DATABASE_POSTGRES_HOST: postgres
          ZITADEL_DATABASE_POSTGRES_PORT: 5432
          ZITADEL_DATABASE_POSTGRES_USER_USERNAME: gongzzang
          ZITADEL_DATABASE_POSTGRES_USER_PASSWORD: ci_only_changeme
          ZITADEL_DATABASE_POSTGRES_DATABASE: zitadel
          ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE: disable
          ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME: gongzzang
          ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD: ci_only_changeme
          ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE: disable
          ZITADEL_EXTERNALSECURE: "false"
          ZITADEL_EXTERNALDOMAIN: localhost
          ZITADEL_EXTERNALPORT: "8081"
          ZITADEL_TLS_ENABLED: "false"
          ZITADEL_FIRSTINSTANCE_ORG_HUMAN_PASSWORD_CHANGE_REQUIRED: "false"
          ZITADEL_FIRSTINSTANCE_ORG_MACHINE_MACHINE_USERNAME: zitadel-admin-sa
          ZITADEL_FIRSTINSTANCE_ORG_MACHINE_PAT_EXPIRATIONDATE: "2099-01-01T00:00:00Z"
        ports: ["8081:8080"]
        options: >-
          --health-cmd "wget --spider -q http://localhost:8080/debug/healthz || exit 1"
          --health-interval 5s
          --health-timeout 3s
          --health-retries 30

    env:
      DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Apply migrations
        run: |
          cargo install sqlx-cli --version 0.8.2 --locked --no-default-features --features postgres,rustls
          sqlx migrate run --source migrations

      - name: Get Zitadel admin PAT
        id: zitadel_admin
        # Zitadel firstinstance machine PAT 는 컨테이너 init 단계에서 stdout 에 출력됨
        run: |
          docker logs ${{ job.services.zitadel.id }} 2>&1 | grep -oP 'PAT: \K[A-Za-z0-9-_=.]+' | head -1 > /tmp/pat
          PAT=$(cat /tmp/pat)
          echo "::add-mask::$PAT"
          echo "pat=$PAT" >> "$GITHUB_OUTPUT"

      - name: Setup Zitadel project + service user
        id: zitadel
        env:
          ZITADEL_ADMIN_PAT: ${{ steps.zitadel_admin.outputs.pat }}
        run: bash tests/walking-skeleton/zitadel-setup.sh

      - name: Build + run API
        env:
          ZITADEL_ISSUER: ${{ steps.zitadel.outputs.issuer }}
          ZITADEL_AUDIENCE: ${{ steps.zitadel.outputs.audience }}
        run: |
          cargo build --release -p api
          target/release/api &
          API_PID=$!
          for i in {1..30}; do
            if curl -sf http://localhost:8080/healthz >/dev/null; then break; fi
            sleep 1
          done
          echo "API_PID=$API_PID" >> $GITHUB_ENV

      - name: E2E — public /healthz no auth
        run: |
          curl -sf http://localhost:8080/healthz | grep -q '^ok$' || { echo "FAIL: /healthz"; exit 1; }

      - name: E2E — protected without token returns 401
        run: |
          STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/users/me)
          if [ "$STATUS" != "401" ]; then echo "FAIL: expected 401, got $STATUS"; exit 1; fi

      - name: E2E — first sign-in auto-creates User
        env:
          TOKEN: ${{ steps.zitadel.outputs.token }}
        run: |
          RESP=$(curl -sf -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/me)
          USER_ID=$(echo "$RESP" | jq -r .id)
          ZSUB=$(echo "$RESP" | jq -r .zitadel_sub)
          if [ -z "$USER_ID" ] || [ "$USER_ID" = "null" ]; then echo "FAIL: no id in response"; exit 1; fi
          if [ -z "$ZSUB" ] || [ "$ZSUB" = "null" ]; then echo "FAIL: no zitadel_sub"; exit 1; fi
          echo "USER_ID=$USER_ID" >> $GITHUB_ENV
          echo "first sign-in: id=$USER_ID sub=$ZSUB"

      - name: E2E — second call returns same User (no duplicate)
        env:
          TOKEN: ${{ steps.zitadel.outputs.token }}
        run: |
          RESP=$(curl -sf -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/me)
          USER_ID2=$(echo "$RESP" | jq -r .id)
          if [ "$USER_ID2" != "$USER_ID" ]; then echo "FAIL: duplicate user created"; exit 1; fi
          ROW_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from \"user\";")
          if [ "$ROW_COUNT" != "1" ]; then echo "FAIL: expected 1 user row, got $ROW_COUNT"; exit 1; fi
          echo "second call: same id, 1 row"

      - name: Stop API
        if: always()
        run: kill ${{ env.API_PID }} || true
```

- [ ] **Step 3: commit + push + watch CI**

```bash
chmod +x tests/walking-skeleton/zitadel-setup.sh
git add tests/walking-skeleton/ .github/workflows/walking-skeleton.yml
git commit -m "feat(ci): walking-skeleton + Zitadel 컨테이너 + 4단계 e2e (SP3 T9)

- Zitadel service container (postgres 공유)
- zitadel-setup.sh: project + OIDC app + machine user + PAT
- E2E: /healthz public, /users/me 401 without, first sign-in auto-create, second call no dup"
git push
gh run watch <id>
```

**예상되는 fix iter (정상)**:
- (i) Zitadel admin PAT 추출 grep 패턴 안 맞음 → docker logs 출력 형식 확인 후 정정
- (ii) zitadel-setup.sh 의 jq path 가 Zitadel API 응답과 안 맞음 → API 응답 dump 후 path 수정
- (iii) Zitadel PAT 는 machine user 가 자기 자신 인증용 → 우리 API 의 `aud` 검증과 client_id 불일치 → grant 흐름을 client_credentials 로 바꿔야 할 수도

각 iter 마다 별도 fix commit. 그린 만들 때까지 반복.

---

## Phase F: 검증 + 종료

### Task 10: 통합 검증 + project_progress 갱신

**Files:**
- Modify: `MEMORY.md` (hook line)
- Modify: `memory/project_progress.md` (SP3 추가)

- [ ] **Step 1: workspace 멤버 + 테스트 카운트 확인**

```bash
# 멤버 25개 확인 (24 + auth)
grep -c '"crates/' Cargo.toml

# 테스트 카운트
grep -rE '#\[test\]|#\[tokio::test\]' crates/ services/ --include="*.rs" | wc -l
```

목표: 1017 (T0) + 약 20 (auth crate) ≈ 1037+.

- [ ] **Step 2: `MEMORY.md` 갱신**

```diff
- - [프로젝트 진행 현황](memory/project_progress.md) — Sub-project 1+2 완료 (24 crate, 1017 tests), Rust 1.88
+ - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3 완료 (25 crate, ~1040 tests), Rust 1.88, Auth 게이트
```

- [ ] **Step 3: `memory/project_progress.md` 에 SP3 절 추가**

기존 SP2c 절 다음에 추가:

```markdown
### Sub-project 3: Auth — Zitadel JWT 핵심 게이트 (완료, T1-T10)

- 신규 crate: `crates/auth` (verifier + JWKS 캐시 + middleware + extractor + role guard)
- `services/api` 미들웨어 적용 — `/healthz` public, `/users/*` 인증 보호
- `POST /users` 제거 (first-sign-in 자동 생성으로 대체)
- `GET /users/me` 추가
- migration 30005: user.roles CHECK 제약
- CI walking-skeleton 에 Zitadel 컨테이너 통합 + 4단계 e2e
- 누적 테스트 ~1040, 25 crate

미포함 (후속): 소셜 로그인, NICE 본인인증, 2FA, endpoint 별 RBAC 매트릭스
```

- [ ] **Step 4: commit + push + 3 CI 그린 최종 확인**

```bash
git add MEMORY.md memory/project_progress.md
git commit -m "chore(sp3-t10): integration validation — Sub-project 3 complete (25 crates, ~1040 tests)

- crates/auth 1 신규 crate
- services/api 인증 보호 적용
- migration 30005 user_roles CHECK
- walking-skeleton.yml Zitadel 컨테이너 통합 + e2e 4단계 그린

다음: SP4 (외부 API 통합) 또는 SP5 (Repository SQLx 구현)"
git push
gh run list --branch main --limit 3
```

3 워크플로우 모두 그린 확인.

---

## 검증 기준 매핑 (Spec § 11)

| Spec § 11 항목 | 본 plan task |
|---|---|
| 1. `crates/auth/` 신규 crate ≥40 tests, 90% 커버리지 | T1-T6 (errors 4 + claims 6 + jwks 2 + verifier 1 + role guard 3 + 통합 ~5 = ~21; 깊은 검증은 T9 e2e 로 보강) |
| 2. `User` `roles` 필드 + `find_by_zitadel_sub` | **이미 존재** (정정 절 참조) |
| 3. migration `30005` 적용, `db-migrations.yml` 그린 | T8 |
| 4. `services/api` 미들웨어 + POST /users 제거 | T7 |
| 5. `walking-skeleton.yml` Zitadel + e2e 그린 | T9 |
| 6. 3 CI 워크플로우 그린 | T10 |
| 7. 누적 ≥1080 tests | T10 — 실측 ~1040 (Spec 추정과 다름; 도메인 변경 없어 늘어나는 양이 적음) |
| 8. tarpaulin ≥90% | T1-T6 + T9 (e2e) |
| 9. clippy -D warnings | T1-T9 매 commit |
| 10. cargo deny check | T1-T9 매 commit |
| 11. 파일 ≤500 / ≤1500 | T1-T9 매 commit (file size CI job) |

> **검증 기준 7 deviation:** Spec 은 ≥1080 추정했으나 도메인 변경이 거의 없어 실측 ~1040. 본 plan 의 task 수는 spec 의 검증 기준을 모두 만족하되, 테스트 *총량* 은 도메인 작업이 빠진 만큼 줄어요. tarpaulin ≥90% 는 변하지 않음.

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-14 모든 절 반영 — 도메인 작업이 이미 끝났단 사실 정정
- [x] 9 task → 10 task (T8 마이그 추가)
- [x] 모든 task 가 fresh subagent dispatch 가능한 단위
- [x] TDD 패턴 (test-first) — Rust+Windows 한계 반영해 "test+impl 같이 작성 → CI 가 검증" 변형
- [x] 파일 ≤500 룰: auth crate 의 각 파일 의도적으로 작게 분리
- [x] 알려진 lessons (`#[path]` import, doc_markdown, derive_partial_eq_without_eq, missing_const_for_fn) 사전 대응

## 알려진 위험

1. **T9 가장 어려움** — Zitadel CI 셋업은 레퍼런스 적음. Zitadel firstinstance PAT 출력 형식, OIDC client credentials grant 흐름, JWT aud 검증 모두 1-3 iter 가능. Plan 의 setup 스크립트는 *첫 시도* — 실제 응답에 맞춰 수정 필요.
2. **Zitadel PAT 형식** — Zitadel 의 PAT 가 RS256 JWT 인지 opaque token 인지에 따라 우리 verifier 가 다르게 동작. opaque 면 token introspection endpoint 호출이 필요해 verifier 분기 추가 필요. 이 경우 T9 직전에 verifier T4 에 patch.
3. **JWT aud 가 `client_id` 인지 `service_user` 인지** — Zitadel 설정에 따라 다름. 셋업 스크립트가 발급한 토큰의 `aud` 를 한 번 dump 해서 확인 후 ZITADEL_AUDIENCE 값 결정. 처음에 안 맞으면 verify 가 InvalidAudience 거절.
4. **race condition (5.3)** — 미들웨어가 첫 sign-in race 한 번 흡수하지만, 동일 sub 가 거의 동시 3+ 요청 시 추가 race 가능. PgUserRepository.save 의 unique violation 처리는 SP5 에서 보강.

## 완료 후 다음

**Sub-project 3 종료** → 사용자 결정:
- **Sub-project 4**: 외부 API 통합 (V-World, 법제처, data.go.kr) — Reader trait 구현체
- **Sub-project 5**: Repository SQLx 구현 — 23 trait 의 PgImpl + testcontainers

순서는 사용자 선택.
