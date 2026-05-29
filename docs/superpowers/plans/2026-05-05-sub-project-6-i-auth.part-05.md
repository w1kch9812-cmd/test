## Task 5: crates/auth JTI denylist + AuthEvent + audit_log emit + OTel span

**Files:**
- Create: `crates/auth/src/jti_denylist.rs`
- Create: `crates/auth/src/audit.rs`
- Modify: `crates/auth/src/claims.rs`
- Modify: `crates/auth/src/lib.rs`
- Modify: `crates/auth/src/middleware.rs`
- Modify: `crates/auth/Cargo.toml`
- Create: `services/api/src/routes/auth_event.rs`
- Modify: `services/api/src/main.rs`
- Test: `crates/auth/src/jti_denylist.rs` (#[cfg(test)])
- Test: `crates/auth/src/audit.rs` (#[cfg(test)])
- Test: `services/api/tests/auth_event_integration.rs` (DB + Redis)

- [ ] **Step 5.1: Cargo.toml — deadpool-redis + sqlx 추가**

`crates/auth/Cargo.toml` 의 `[dependencies]` 에 추가:

```toml
deadpool-redis = "0.18"
sqlx = { workspace = true, features = ["postgres", "runtime-tokio", "macros", "json", "chrono"] }
chrono = { workspace = true, features = ["serde"] }
```

- [ ] **Step 5.2: claims.rs — jti 추가**

`crates/auth/src/claims.rs` 의 `Claims` 구조체에 필드 추가 (line 9 의 struct):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// `JWT` ID — `JTI` denylist key. Zitadel 가 항상 발급.
    pub jti: String,
    pub exp: i64,
    #[serde(default)]
    pub nbf: Option<i64>,
    pub iss: String,
    pub aud: Audience,
}
```

기존 테스트 fixture 들 (deserialize_*) 에 `"jti":"..."` 추가:

```rust
let json = r#"{"sub":"u1","jti":"j1","exp":1000,"iss":"http://i","aud":"client-x"}"#;
```

(모든 test JSON 에 `"jti":"j1"` 추가; Claims 인스턴스 생성에도 `jti: "j1".into()` 추가)

- [ ] **Step 5.3: jti_denylist.rs trait + impl**

`crates/auth/src/jti_denylist.rs`:

```rust
//! JTI 무효화 목록 (logout / refresh rotation / role change 시 token 즉시 무효).

use async_trait::async_trait;
use deadpool_redis::{redis::AsyncCommands, Pool};

/// `JWT` `JTI` denylist 트레잇.
///
/// 검증 단계에서 `is_denied(jti)` 를 호출 → `true` 면 `AuthError::Expired` 처럼 거부해요.
#[async_trait]
pub trait JtiDenylist: Send + Sync {
    /// 해당 jti 가 무효인지 (denylist hit).
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError>;

    /// jti 를 ttl 초 동안 무효화.
    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError>;
}

/// `JTI` denylist 작업 중 발생할 수 있는 오류.
#[derive(Debug, thiserror::Error)]
pub enum JtiError {
    /// Redis 연결 실패 또는 명령 오류.
    #[error("redis: {0}")]
    Redis(String),
}

impl From<deadpool_redis::PoolError> for JtiError {
    fn from(e: deadpool_redis::PoolError) -> Self {
        Self::Redis(e.to_string())
    }
}

impl From<deadpool_redis::redis::RedisError> for JtiError {
    fn from(e: deadpool_redis::redis::RedisError) -> Self {
        Self::Redis(e.to_string())
    }
}

/// Redis 기반 `JTI` denylist 구현.
pub struct RedisJtiDenylist {
    pool: Pool,
}

impl RedisJtiDenylist {
    /// `Pool` 로 새 인스턴스 생성.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn key(jti: &str) -> String {
        format!("jti:deny:{jti}")
    }
}

#[async_trait]
impl JtiDenylist for RedisJtiDenylist {
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.exists(Self::key(jti)).await?;
        Ok(exists)
    }

    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set_ex(Self::key(jti), "1", ttl_sec).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use deadpool_redis::{Config, Runtime};

    fn pool() -> Option<Pool> {
        let url = std::env::var("REDIS_URL").ok()?;
        let cfg = Config::from_url(url);
        cfg.create_pool(Some(Runtime::Tokio1)).ok()
    }

    #[tokio::test]
    async fn deny_then_is_denied_true() {
        let Some(p) = pool() else {
            eprintln!("REDIS_URL not set, skipping");
            return;
        };
        let dl = RedisJtiDenylist::new(p);
        let jti = format!("test-{}", uuid::Uuid::new_v4());
        assert!(!dl.is_denied(&jti).await.expect("query"));
        dl.deny(&jti, 60).await.expect("deny");
        assert!(dl.is_denied(&jti).await.expect("query"));
    }
}
```

(NOTE: tests 에 uuid crate 의존; 이미 workspace 에 있는지 확인 필요. 없으면 dev-dep 추가.)

- [ ] **Step 5.4: audit.rs — AuthEvent + writer**

`crates/auth/src/audit.rs`:

```rust
//! 인증 이벤트 → `audit_log` writer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

/// 인증 흐름에서 발생하는 6 종 이벤트.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event")]
pub enum AuthEvent {
    /// 첫 로그인 또는 새 세션 발급.
    Login { user_sub: String, jti: String, exp: i64 },
    /// 로그아웃 (back-channel).
    Logout { user_sub: String, jti: String },
    /// Refresh 성공 (jti rotation 포함).
    RefreshSucceeded {
        user_sub: String,
        prev_jti: String,
        new_jti: String,
        exp: i64,
    },
    /// Refresh 실패 (Zitadel 거부 / 네트워크 실패).
    RefreshFailed { user_sub: String, jti: String },
    /// 권한 가드 거부 (role mismatch 등).
    RoleGuardDenied {
        user_sub: String,
        required_role: String,
        actual_role: String,
        path: String,
    },
    /// Role 변경 — 모든 활성 jti 가 denylist 추가됨.
    RoleChanged {
        user_sub: String,
        prev_role: String,
        new_role: String,
        invalidated_jti_count: u32,
    },
}

impl AuthEvent {
    /// `audit_log.action` 컬럼에 들어갈 dotted name.
    #[must_use]
    pub const fn action(&self) -> &'static str {
        match self {
            Self::Login { .. } => "auth.login",
            Self::Logout { .. } => "auth.logout",
            Self::RefreshSucceeded { .. } => "auth.refresh.succeeded",
            Self::RefreshFailed { .. } => "auth.refresh.failed",
            Self::RoleGuardDenied { .. } => "auth.role_guard.denied",
            Self::RoleChanged { .. } => "auth.role.changed",
        }
    }

    /// 추적용 user_sub 추출.
    #[must_use]
    pub fn user_sub(&self) -> &str {
        match self {
            Self::Login { user_sub, .. }
            | Self::Logout { user_sub, .. }
            | Self::RefreshSucceeded { user_sub, .. }
            | Self::RefreshFailed { user_sub, .. }
            | Self::RoleGuardDenied { user_sub, .. }
            | Self::RoleChanged { user_sub, .. } => user_sub.as_str(),
        }
    }
}

/// `audit_log` 에 인증 이벤트를 기록해요.
///
/// # Errors
///
/// Postgres INSERT 실패 시 `sqlx::Error` 반환.
pub async fn write(pool: &PgPool, event: &AuthEvent, correlation_id: &str) -> Result<(), sqlx::Error> {
    let id = format!("aud_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);
    let payload = serde_json::to_value(event).expect("AuthEvent serialize");

    sqlx::query(
        r#"
        INSERT INTO audit_log
          (id, actor_id, action, resource_kind, resource_id,
           before_state, after_state, correlation_id, created_at)
        VALUES ($1, NULL, $2, 'user', $3, NULL, $4, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(event.action())
    .bind(event.user_sub())
    .bind(&payload)
    .bind(correlation_id)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_name_matches() {
        let e = AuthEvent::Login {
            user_sub: "u".into(),
            jti: "j".into(),
            exp: 1000,
        };
        assert_eq!(e.action(), "auth.login");
        assert_eq!(e.user_sub(), "u");
    }

    #[test]
    fn role_changed_action() {
        let e = AuthEvent::RoleChanged {
            user_sub: "u".into(),
            prev_role: "Buyer".into(),
            new_role: "Broker".into(),
            invalidated_jti_count: 3,
        };
        assert_eq!(e.action(), "auth.role.changed");
    }

    #[test]
    fn round_trip_serde() {
        let e = AuthEvent::RefreshSucceeded {
            user_sub: "u".into(),
            prev_jti: "j1".into(),
            new_jti: "j2".into(),
            exp: 1000,
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: AuthEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }
}
```

- [ ] **Step 5.5: lib.rs export 추가**

`crates/auth/src/lib.rs` 의 `pub mod` 목록 끝에 추가:

```rust
pub mod audit;
pub mod jti_denylist;
```

- [ ] **Step 5.6: middleware.rs 에 jti denylist 검증 hook 추가**

먼저 현 `crates/auth/src/middleware.rs` 의 `AuthState` 구조체 + `auth_layer` 함수의 verify 호출 위치를 Read 로 확인 (`AuthState { verifier, user_repo }` 와 `verifier.verify(token).await?` 호출). 그 다음 두 가지 변경:

**변경 1**: `AuthState` 에 `jti_denylist: Option<Arc<dyn crate::jti_denylist::JtiDenylist>>` 필드 추가 (`pub user_repo` 다음 줄):

```rust
pub struct AuthState {
    pub verifier: Arc<Verifier>,
    pub user_repo: Arc<dyn UserRepository>,
    /// `JTI` denylist (`SP6-i`) — `None` 이면 검증 skip (fail-open).
    pub jti_denylist: Option<Arc<dyn crate::jti_denylist::JtiDenylist>>,
}
```

**변경 2**: `auth_layer` 의 `let claims = state.verifier.verify(token).await?;` 직후 (User 자동 생성/조회 직전) hook 추가:

```rust
let claims = state.verifier.verify(token).await?;

// SP6-i: JTI denylist (logout / refresh rotation / role change 시 즉시 무효).
// fail-open 정책: Redis 장애 시 가용성 우선 (JWT 검증만으로 통과). audit log 만 남김.
if let Some(dl) = &state.jti_denylist {
    match dl.is_denied(&claims.jti).await {
        Ok(true) => return Err(AuthError::Expired),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, jti = %claims.jti, "jti denylist check failed (fail-open)"),
    }
}

// 기존: User 자동 생성 또는 조회 (변경 없음)
```

`AuthState::new(verifier, user_repo)` 같은 builder 가 있다면 `jti_denylist: None` default 추가 + `with_jti_denylist(...)` 메서드 추가.

- [ ] **Step 5.7: services/api/src/routes/auth_event.rs**

`services/api/src/routes/auth_event.rs`:

```rust
//! `POST /internal/auth/event` — frontend 가 emit 하는 `AuthEvent` 수신 → `audit_log` INSERT.

use auth::audit::{self, AuthEvent};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;

/// 핸들러용 상태 (DB pool).
#[derive(Clone)]
pub struct AuthEventState {
    pub pool: PgPool,
}

/// 요청 본문.
#[derive(Debug, Deserialize)]
pub struct AuthEventPayload {
    pub event: String,
    pub payload: serde_json::Value,
}

/// 핸들러 — `event` + `payload` 를 합쳐 `AuthEvent` 로 deserialize 한 후 `audit_log` 에 기록.
///
/// # Errors
///
/// JSON 파싱 / DB INSERT 실패 시 500 반환.
pub async fn post_auth_event(
    State(state): State<AuthEventState>,
    Json(body): Json<AuthEventPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut combined = body.payload;
    if let Some(obj) = combined.as_object_mut() {
        obj.insert("event".into(), serde_json::Value::String(body.event));
    } else {
        return Err((StatusCode::BAD_REQUEST, "payload must be object".to_owned()));
    }

    let event: AuthEvent = serde_json::from_value(combined)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event: {e}")))?;

    let correlation_id = format!("cor_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);

    audit::write(&state.pool, &event, &correlation_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}
```

- [ ] **Step 5.8: services/api/src/main.rs modify (라우트 + jti_denylist init)**

`services/api/src/main.rs` 에 추가:

```rust
mod routes {
    pub mod auth_event;
}

use deadpool_redis::{Config as RedisCfg, Runtime as RedisRt};

// ... main() 내 ...
let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
let redis_pool = RedisCfg::from_url(redis_url)
    .create_pool(Some(RedisRt::Tokio1))
    .expect("redis pool");
let jti_denylist: Arc<dyn auth::jti_denylist::JtiDenylist> =
    Arc::new(auth::jti_denylist::RedisJtiDenylist::new(redis_pool));

let auth_state = AuthState {
    verifier,
    user_repo,
    jti_denylist: Some(jti_denylist),
};

let auth_event_state = routes::auth_event::AuthEventState { pool: pool.clone() };

let internal: Router<()> = Router::new()
    .route("/internal/auth/event", axum::routing::post(routes::auth_event::post_auth_event))
    .with_state(auth_event_state);

let app = public.merge(protected).merge(internal).layer(TraceLayer::new_for_http());
```

(`deadpool-redis` 를 services/api 의 Cargo.toml 에도 dependency 추가 — `deadpool-redis = "0.18"`.)

- [ ] **Step 5.9: cargo check + clippy**

```
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5.10: cargo test**

```
REDIS_URL=redis://localhost:6379 cargo test -p auth
cargo test -p api
```

Expected: PASS (jti_denylist + audit + auth_event integration).

- [ ] **Step 5.11: Commit**

```bash
git add crates/auth/ services/api/ Cargo.lock
git commit -m "feat(6i-T5): crates/auth jti_denylist + audit_log emit + auth_event endpoint

- claims.rs: jti field 추가 (Zitadel 가 항상 발급)
- jti_denylist.rs: trait + RedisJtiDenylist (deadpool-redis)
- audit.rs: AuthEvent enum (6종) + write(pool, event, correlation_id)
- middleware.rs: verify 후 jti denylist check (fail-open 정책)
- services/api: POST /internal/auth/event 라우트
- main.rs: REDIS_URL env + AuthState.jti_denylist 주입"
```

---

## Task 6: V004 migration + sqlx prepare hook + first-sign-in external_account insert

**Files:**
- Create: `migrations/30008_user_ci_external_account.sql`
- Modify: `crates/auth/src/middleware.rs` (first sign-in 시 external_account zitadel insert)
- Modify: `lefthook.yml` (pre-push 에 sqlx prepare --check)
- Modify: `tarpaulin.toml` (auth crate 새 모듈 포함 확인)

- [ ] **Step 6.1: migration 작성**

`migrations/30008_user_ci_external_account.sql`:

```sql
-- V003_08: SP6-i Auth Core 의 schema 자리.
-- users.ci 는 SP6-CI (KISA 본인확인) 가 채움.
-- external_account 의 kakao/naver/google 행은 SP6-Social federation 이 채움.

ALTER TABLE "user" ADD COLUMN ci VARCHAR(88) UNIQUE NULL;
COMMENT ON COLUMN "user".ci IS
  'KISA Connecting Information (88-char hash). NULL until SP6-CI verifies via NICE/Toss/PASS.';

CREATE TABLE external_account (
    id           CHAR(30) PRIMARY KEY,
    user_id      CHAR(30) NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    provider     VARCHAR(32) NOT NULL,
    external_id  VARCHAR(255) NOT NULL,
    linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, external_id)
);

CREATE INDEX external_account_user_idx ON external_account(user_id);
CREATE INDEX external_account_provider_idx ON external_account(provider, linked_at DESC);

COMMENT ON TABLE external_account IS
  'Multi-IdP linking. SP6-i populates only zitadel rows on first sign-in. SP6-Social federation populates kakao/naver/google.';

-- provider 값 제약 (SP6-Social 이 추가 시 ALTER 가능)
ALTER TABLE external_account
  ADD CONSTRAINT external_account_provider_chk
  CHECK (provider IN ('zitadel', 'kakao', 'naver', 'google', 'apple'));
```

(NOTE: 기존 `user` 테이블 이름이 `"user"` quoted — V001 패턴 일관 유지. id 는 `char(30)` `usr_...` 형식.)

- [ ] **Step 6.2: migration 적용 + sqlx prepare**

```
psql $DATABASE_URL -f migrations/30008_user_ci_external_account.sql
cargo sqlx prepare --workspace
```

Expected: `.sqlx/` 의 query json 갱신 (auth 가 user 테이블 select 하는 경우).

- [ ] **Step 6.3: first sign-in 시 external_account insert**

`crates/auth/src/middleware.rs` 의 first-sign-in 분기에 추가 (User 자동 생성 후, 같은 트랜잭션 또는 best-effort INSERT):

```rust
// User 자동 생성 직후
if was_first_sign_in {
    let external_id = format!("ea_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO external_account (id, user_id, provider, external_id)
        VALUES ($1, $2, 'zitadel', $3)
        ON CONFLICT (provider, external_id) DO NOTHING
        "#,
    )
    .bind(&external_id)
    .bind(user.id.as_str())
    .bind(&claims.sub)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "external_account zitadel insert failed (best-effort)");
    }
}
```

(실제 위치는 middleware.rs 의 first sign-in 로직 확인 후 결정. 현재 코드 미확인 시 Step 7 의 코드 검토 후 정확한 위치 적용.)

- [ ] **Step 6.4: lefthook.yml 에 sqlx prepare check 추가**

`lefthook.yml` 의 `pre-push:` 섹션에 추가:

```yaml
    sqlx-prepare-check:
      run: command -v cargo >/dev/null 2>&1 && (DATABASE_URL=${DATABASE_URL:-postgres://gongzzang:gongzzang@localhost:5432/gongzzang} cargo sqlx prepare --workspace --check) || echo "cargo not installed - CI enforces"
      skip:
        - merge
        - rebase
```

- [ ] **Step 6.5: tarpaulin.toml 검토**

`tarpaulin.toml` — `crates/auth/` 가 이미 포함되어 있는지 확인. exclude 목록에 jti_denylist / audit 가 없어야 함 (90% threshold 적용).

- [ ] **Step 6.6: db-migrations workflow assertion 갱신**

`tests/migrations/test_v001_full.sh` 의 `EXPECTED_TABLES` 배열에 `external_account` 추가 (SP7-iii 에서 이미 동적 count 사용 중 — 새 테이블 1개 추가 시 자동 반영되지만 명시 등록은 필요):

```bash
# 변경 전 (SP7-iii 후 상태):
EXPECTED_TABLES=(... api_health_check)

# 변경 후 (SP6-i 추가):
EXPECTED_TABLES=(... api_health_check external_account)
```

확인 명령:

```bash
grep -n "EXPECTED_TABLES" tests/migrations/test_v001_full.sh
# 해당 라인의 배열에 external_account 추가
bash tests/migrations/test_v001_full.sh  # 로컬 검증 (필요한 환경 변수 설정 후)
```

- [ ] **Step 6.7: 전체 빌드 + clippy + test**

```
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
DATABASE_URL=postgres://... cargo sqlx prepare --workspace --check
```

Expected: PASS.

- [ ] **Step 6.8: Commit**

```bash
git add migrations/30008_user_ci_external_account.sql crates/auth/src/middleware.rs lefthook.yml .sqlx/ tests/migrations/ .github/workflows/db-migrations.yml
git commit -m "feat(6i-T6): V004 schema (users.ci + external_account) + sqlx prepare hook

- migrations/30008: users.ci VARCHAR(88) UNIQUE NULL (SP6-CI 채움) + external_account 테이블 (SP6-Social 채움), zitadel 한 줄만 first sign-in 시 자동 insert
- middleware.rs: first sign-in 시 external_account('zitadel', sub) INSERT (best-effort)
- lefthook.yml: pre-push 에 cargo sqlx prepare --check 추가 (V004 schema drift 차단)"
```

---

