# Sub-project 6-i Auth - Part 05A: JTI Denylist and Auth Events

Parent index: [Sub-project 6-i Auth - Part 05](./2026-05-05-sub-project-6-i-auth.part-05.md).
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
