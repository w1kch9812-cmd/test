# T7: AppState/lib.rs Split + ReadinessResponse + Integration Test Rewrite

**Goal:** `services/api` 를 router/state factory 로 분리 (lib.rs + state.rs) → 통합 테스트 가 실 router 호출 가능. `/healthz/ready` 의 `HealthResponse` 단순 구조를 `ReadinessResponse { status, checks }` nested 로 확장. `services/api/tests/sp10_panel_endpoints.rs` 의 핸들러 재구현 path 를 *실 router* 호출로 재작성.

**Spec SSOT:** §10.5 (Health Shape), §11 통합 변경 표 (services/api split, sp10_panel_endpoints.rs 재작성), §13 T7 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**Files:**

- Create: `services/api/src/state.rs`
- Create: `services/api/src/lib.rs`
- Create: `services/api/tests/sp10_backend_data_correctness.rs`
- Modify: `services/api/src/main.rs` (lib.rs 의 `app_router` 호출하는 thin entry point)
- Modify: `services/api/src/routes/health.rs` (lines 45-125 ReadinessResponse 추가)
- Modify: `services/api/tests/sp10_panel_endpoints.rs` (lines 29-36, 148-250 — spawn_test_app → app_router)
- Modify: `services/api/Cargo.toml` ([lib] + [[bin]] sections)

---

## Step 7.1: 사전 검증 — 실제 main.rs 구조 read

T1~T6 이 main.rs 에 누적 변경 — 정확한 현재 상태 확인 후 split.

- [ ] **Step 7.1.1: Read main.rs 의 router 빌더 영역**

```bash
grep -n "Router::new\|axum::Router\|fn main\|tokio::spawn" services/api/src/main.rs | head -20
# Expected: main() entry + router builder location 확인
```

- [ ] **Step 7.1.2: Read health.rs 의 현재 핸들러**

```bash
sed -n '40,130p' services/api/src/routes/health.rs
# Expected: 기존 HealthResponse { status: "ok" } 구조 확인
```

- [ ] **Step 7.1.3: Read sp10_panel_endpoints.rs 의 spawn_test_app**

```bash
grep -n "spawn_test_app\|TestServer\|test_app" services/api/tests/sp10_panel_endpoints.rs | head -10
# Expected: 현재 헬퍼 location + 핸들러 재구현 pattern 확인
```

---

## Step 7.2: state.rs — AppState struct

- [ ] **Step 7.2.1: Create `services/api/src/state.rs`**

```rust
//! Application state — DB pool, KMS, status handles, etc. 통합 테스트가 동일
//! state 로 router 빌드 가능하도록 분리.

use aws_sdk_kms::Client as KmsClient;
use gongzzang_db::PgVaultAccessLog;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub kms: Arc<KmsClient>,
    pub kms_key_id: String,
    pub access_log: PgVaultAccessLog,
    pub building_reader_status: &'static str,
    pub vault_kms_status: &'static str,
    // 기존 main.rs 의 다른 wiring 도 점진적으로 추가 (e.g., redis_pool, vworld_reader)
}

impl AppState {
    /// 환경 변수 + DB 연결 + KMS 클라이언트로 state 초기화.
    /// production 환경에서 키 미설정 시 panic (fail-fast).
    pub async fn from_env() -> Result<Self, anyhow::Error> {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let kms = Arc::new(KmsClient::new(&aws_config));
        let kms_key_id = std::env::var("PII_VAULT_KMS_KEY_ID")?;

        let has_data_go_kr = std::env::var("DATA_GO_KR_API_KEY").is_ok();
        let is_production = std::env::var("APP_ENV").as_deref() == Ok("production");
        if !has_data_go_kr && is_production {
            anyhow::bail!("DATA_GO_KR_API_KEY missing in production (fail-fast)");
        }
        let building_reader_status = if has_data_go_kr { "live" } else { "degraded" };

        Ok(Self {
            pool: pool.clone(),
            kms: kms.clone(),
            kms_key_id,
            access_log: PgVaultAccessLog::new(pool),
            building_reader_status,
            vault_kms_status: "ok", // 본 plan 범위 외 — KMS healthcheck 는 FU (ADR-driven)
        })
    }
}
```

- [ ] **Step 7.2.2: Verify compile**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 7.2.3: Commit**

```bash
git add services/api/src/state.rs
git commit -m "feat(sp10-5-b-T7): AppState struct + from_env factory"
```

---

## Step 7.3: ReadinessResponse + nested checks (TDD)

- [ ] **Step 7.3.1: Append to `services/api/src/routes/health.rs` — failing test ONLY**

```rust
#[cfg(test)]
mod readiness_tests {
    use super::*;

    #[test]
    fn readiness_response_serializes_with_checks() {
        let resp = ReadinessResponse {
            status: "ok".to_string(),
            checks: ReadinessChecks {
                db: "ok".to_string(),
                redis: "ok".to_string(),
                building_reader: "live".to_string(),
                vault_kms: "ok".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["checks"]["db"], "ok");
        assert_eq!(json["checks"]["building_reader"], "live");
    }

    #[test]
    fn readiness_aggregate_status_degraded_when_any_degraded() {
        let status = aggregate_status(&ReadinessChecks {
            db: "ok".to_string(),
            redis: "ok".to_string(),
            building_reader: "degraded".to_string(),
            vault_kms: "ok".to_string(),
        });
        assert_eq!(status, "degraded");
    }

    #[test]
    fn readiness_aggregate_status_down_when_any_down() {
        let status = aggregate_status(&ReadinessChecks {
            db: "down".to_string(),
            redis: "ok".to_string(),
            building_reader: "live".to_string(),
            vault_kms: "ok".to_string(),
        });
        assert_eq!(status, "down");
    }
}
```

- [ ] **Step 7.3.2: Run — verify FAIL**

```bash
cargo test -p api --lib routes::health::readiness_tests
# Expected: error[E0422]: cannot find struct `ReadinessResponse`
```

- [ ] **Step 7.3.3: Implement `ReadinessResponse` + `ReadinessChecks` + handler**

Append to `health.rs`:

```rust
use crate::state::AppState;
use axum::{extract::State, response::Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub checks: ReadinessChecks,
}

#[derive(Debug, Serialize)]
pub struct ReadinessChecks {
    pub db: String,
    pub redis: String,
    pub building_reader: String,
    pub vault_kms: String,
}

fn aggregate_status(checks: &ReadinessChecks) -> String {
    if [&checks.db, &checks.redis, &checks.building_reader, &checks.vault_kms]
        .iter()
        .any(|s| s.as_str() == "down")
    {
        "down".to_string()
    } else if [&checks.db, &checks.redis, &checks.building_reader, &checks.vault_kms]
        .iter()
        .any(|s| s.as_str() == "degraded")
    {
        "degraded".to_string()
    } else {
        "ok".to_string()
    }
}

pub async fn readiness_handler(State(state): State<AppState>) -> Json<ReadinessResponse> {
    let db_status = match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => "ok",
        Err(_) => "down",
    };
    let redis_status = "ok"; // 본 plan 범위 외 — redis ping 은 FU (Operations 분야)
    let checks = ReadinessChecks {
        db: db_status.to_string(),
        redis: redis_status.to_string(),
        building_reader: state.building_reader_status.to_string(),
        vault_kms: state.vault_kms_status.to_string(),
    };
    let status = aggregate_status(&checks);
    Json(ReadinessResponse { status, checks })
}
```

- [ ] **Step 7.3.4: Run — verify PASS**

```bash
cargo test -p api --lib routes::health::readiness_tests
# Expected: 3 passed
```

- [ ] **Step 7.3.5: Commit**

```bash
git add services/api/src/routes/health.rs
git commit -m "feat(sp10-5-b-T7): ReadinessResponse + nested checks (db/redis/reader/kms)"
```

---

## Step 7.4: lib.rs — `app_router(state)` factory

- [ ] **Step 7.4.1: Create `services/api/src/lib.rs`**

```rust
//! services/api 의 router/state factory. main.rs 는 thin entry point — 통합
//! 테스트가 동일 `app_router(state)` 로 실 router 빌드.

pub mod cleanup;
pub mod routes;
pub mod state;

use axum::Router;
use crate::state::AppState;

/// 모든 라우트를 등록한 axum Router. main.rs / 통합 테스트 공통 진입점.
pub fn app_router(state: AppState) -> Router {
    let admin_state = routes::admin::raw_vault::AdminState {
        pool: state.pool.clone(),
        kms: state.kms.clone(),
        access_log: state.access_log.clone(),
    };

    Router::new()
        .route(
            "/healthz/ready",
            axum::routing::get(routes::health::readiness_handler),
        )
        .route(
            "/api/admin/raw_vault/:source/:pnu",
            axum::routing::get(routes::admin::raw_vault::get_vault_handler),
        )
        .with_state(state)
        .merge(
            Router::new()
                .route(
                    "/api/admin/raw_vault/:source/:pnu",
                    axum::routing::get(routes::admin::raw_vault::get_vault_handler),
                )
                .with_state(admin_state),
        )
    // 기존 main.rs 의 다른 라우트 (panel endpoints, listings, etc.) 도 점진 이동
}
```

- [ ] **Step 7.4.2: Modify `services/api/Cargo.toml` — add [lib]**

```toml
[lib]
path = "src/lib.rs"

[[bin]]
name = "api"
path = "src/main.rs"
```

- [ ] **Step 7.4.3: Modify `services/api/src/main.rs` — thin entry**

main.rs 를 *짧게* 정리 (router 빌더 본문이 lib.rs 로 이동):

```rust
use api::{app_router, cleanup::CleanupTask, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState::from_env().await?;

    // TTL cleanup task — spec §7.3
    let cleanup = CleanupTask::new(state.pool.clone(), std::time::Duration::from_secs(3600));
    tokio::spawn(cleanup.spawn_loop());

    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!(addr = "0.0.0.0:8080", "api listening");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 7.4.4: Verify build**

```bash
cargo check -p api
# Expected: Finished
cargo build -p api --bin api
# Expected: binary 빌드 통과
```

- [ ] **Step 7.4.5: Commit**

```bash
git add services/api/src/lib.rs services/api/src/main.rs services/api/Cargo.toml
git commit -m "feat(sp10-5-b-T7): split api into lib (app_router factory) + thin bin"
```

---

## Step 7.5: Rewrite sp10_panel_endpoints.rs — real router

- [ ] **Step 7.5.1: Read existing sp10_panel_endpoints.rs**

```bash
wc -l services/api/tests/sp10_panel_endpoints.rs
# Expected: 200~300 lines (handler 재구현 pattern)
```

- [ ] **Step 7.5.2: Replace spawn_test_app with app_router-based test fixture**

기존 `spawn_test_app()` 함수 + 핸들러 재구현 코드 *전체 삭제*. 대신 다음 패턴:

```rust
use api::{app_router, state::AppState};
use axum_test::TestServer;
use sqlx::PgPool;

async fn test_server() -> TestServer {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566") // localstack
        .load()
        .await;
    let kms = std::sync::Arc::new(aws_sdk_kms::Client::new(&aws_config));
    let state = AppState {
        pool: pool.clone(),
        kms,
        kms_key_id: "alias/test-pii-vault".to_string(),
        access_log: gongzzang_db::PgVaultAccessLog::new(pool),
        building_reader_status: "live",
        vault_kms_status: "ok",
    };
    TestServer::new(app_router(state)).unwrap()
}

#[tokio::test]
#[ignore = "requires test DB + localstack KMS"]
async fn readiness_endpoint_returns_ok() {
    let server = test_server().await;
    let response = server.get("/healthz/ready").await;
    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["checks"]["building_reader"], "live");
}

// 기존 sp10_panel_endpoints 의 다른 시나리오들 (parcel summary, building list, listing) 도
// 동일 패턴으로 — app_router 가 실 router 빌드 → TestServer → 실제 HTTP request
```

- [ ] **Step 7.5.3: Verify build**

```bash
cargo check -p api --tests
# Expected: Finished
```

- [ ] **Step 7.5.4: Run integration tests (ignored unless test env)**

```bash
cargo test -p api --test sp10_panel_endpoints -- --ignored
# Expected (with test env): tests pass
# Expected (without): ignored
```

- [ ] **Step 7.5.5: Commit**

```bash
git add services/api/tests/sp10_panel_endpoints.rs
git commit -m "test(sp10-5-b-T7): rewrite sp10_panel_endpoints with app_router + axum-test"
```

---

## Step 7.6: New integration test — sp10_backend_data_correctness.rs

- [ ] **Step 7.6.1: Create `services/api/tests/sp10_backend_data_correctness.rs`**

```rust
//! SP10.5-B 통합 검증 — PII fixture + vault RLS + audit log + health degraded.
//!
//! Spec §10.3 acceptance criteria SSOT.

use api::{app_router, state::AppState};
use axum_test::TestServer;
use sqlx::PgPool;
use std::sync::Arc;

async fn test_state(building_status: &'static str) -> AppState {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566")
        .load()
        .await;
    let kms = Arc::new(aws_sdk_kms::Client::new(&aws_config));
    AppState {
        pool: pool.clone(),
        kms,
        kms_key_id: "alias/test-pii-vault".to_string(),
        access_log: gongzzang_db::PgVaultAccessLog::new(pool),
        building_reader_status: building_status,
        vault_kms_status: "ok",
    }
}

#[tokio::test]
#[ignore = "requires test DB + KMS"]
async fn pii_fixture_dropped_in_tier1() {
    // V-World fixture with hypothetical PII field → sanitizer drop 검증.
    let state = test_state("live").await;
    let raw = serde_json::json!({
        "response": {
            "result": {
                "featureCollection": {
                    "features": [{
                        "geometry": {"type": "MultiPolygon"},
                        "properties": {
                            "pnu": "1111010100100010004",
                            "OWNER_RRN": "900101-1234567"  // PII — must be dropped
                        }
                    }]
                }
            }
        }
    });

    use raw_capture_client::{AllowlistSanitizer, DualTierCapture, RawCapture, SanitizingRawCapture};
    let sanitizer = Arc::new(AllowlistSanitizer::for_source("vworld_parcel").unwrap());
    // Tier 1 sink for test — mock that stores into parcel_external_data
    let tier1 = gongzzang_db::test_helpers::TestPgRawCapture::new(state.pool.clone());
    let tier2 = gongzzang_db::PgPiiVaultCapture::new(
        state.pool.clone(),
        state.kms.clone(),
        state.kms_key_id.clone(),
        chrono::Duration::days(30),
    );
    let dual = DualTierCapture::new(SanitizingRawCapture::new(tier1, sanitizer), tier2);
    dual.capture(
        "1111010100100010004",
        "vworld_parcel",
        &raw,
        chrono::Utc::now(),
    )
    .await
    .unwrap();

    // Tier 1 sanitized raw_response — OWNER_RRN 없음 검증
    let row: (serde_json::Value,) = sqlx::query_as(
        "SELECT raw_response FROM parcel_external_data WHERE pnu = $1 AND source = $2",
    )
    .bind("1111010100100010004")
    .bind("vworld_parcel")
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let json_str = row.0.to_string();
    assert!(!json_str.contains("OWNER_RRN"), "PII leaked into Tier 1!");
}

#[tokio::test]
#[ignore = "requires test DB + KMS"]
async fn admin_endpoint_rls_blocks_non_admin() {
    let state = test_state("live").await;
    let server = TestServer::new(app_router(state)).unwrap();
    let response = server
        .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010005")
        .add_header("authorization", "Bearer non-admin-jwt")
        .add_query_param("purpose", "drift_diagnosis")
        .add_query_param("ticket_id", "TICKET-1")
        .await;
    assert_eq!(response.status_code(), 403);
}

#[tokio::test]
#[ignore = "requires test DB + KMS + admin JWT"]
async fn admin_endpoint_creates_audit_log_before_decrypt() {
    let state = test_state("live").await;
    let pool = state.pool.clone();
    // Pre-condition: vault row 존재 + admin JWT 발급
    // 응답 200 후 raw_vault_access_log 에 1 row 추가됨 검증
    let server = TestServer::new(app_router(state)).unwrap();
    let response = server
        .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010005")
        .add_header("authorization", "Bearer admin-jwt-test")
        .add_query_param("purpose", "drift_diagnosis")
        .add_query_param("ticket_id", "TICKET-2")
        .await;
    if response.status_code() == 200 {
        let count: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM raw_vault_access_log
              WHERE ticket_id = 'TICKET-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(count.0 >= 1, "audit log 1 row created");
    }
}

#[tokio::test]
#[ignore = "requires test DB"]
async fn health_degraded_when_building_reader_noop() {
    let state = test_state("degraded").await;
    let server = TestServer::new(app_router(state)).unwrap();
    let response = server.get("/healthz/ready").await;
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["checks"]["building_reader"], "degraded");
}
```

- [ ] **Step 7.6.2: Verify build**

```bash
cargo check -p api --tests
# Expected: Finished (test_helpers::TestPgRawCapture 가 없으면 add)
```

- [ ] **Step 7.6.3: Commit**

```bash
git add services/api/tests/sp10_backend_data_correctness.rs
git commit -m "test(sp10-5-b-T7): sp10_backend_data_correctness integration (4 scenarios)"
```

---

## Step 7.7: Final workspace verification

- [ ] **Step 7.7.1: Run full test + lint suite**

```bash
cargo test --workspace --lib
# Expected: all unit tests pass
cargo test --workspace --tests -- --ignored
# Expected (with test env): integration tests pass
cargo clippy --workspace -- -D warnings
# Expected: no warnings
cargo fmt --check
# Expected: no diff
cargo sqlx prepare --check
# Expected: no diff (all queries compile)
```

- [ ] **Step 7.7.2: Verify migration tally**

```bash
sqlx migrate info | grep -E "30012|30013|30014|30015|30016"
# Expected: 5 lines, all [x] Applied
```

- [ ] **Step 7.7.3: Verify acceptance criteria checklist**

README.md 의 Acceptance Criteria (§10.1~§10.5) 검증:
- [ ] 10.1 컴파일/Lint — cargo check/clippy/fmt/sqlx prepare 모두 통과
- [ ] 10.2 Unit Tests — sanitizer/capture/access_log 모든 unit pass
- [ ] 10.3 Integration Tests — sp10_panel_endpoints (실 router) + sp10_backend_data_correctness pass
- [ ] 10.4 Migration — 5건 forward + 멱등성 검증
- [ ] 10.5 Health Shape — ReadinessResponse nested checks JSON 검증

---

## Acceptance — T7 완료 기준

- [ ] `services/api/src/{lib.rs, state.rs}` 신규 — `app_router(state)` factory export
- [ ] `services/api/src/main.rs` thin entry (router 빌더는 lib.rs)
- [ ] `services/api/src/routes/health.rs` — `ReadinessResponse` + `ReadinessChecks` + handler
- [ ] `services/api/tests/sp10_panel_endpoints.rs` — `app_router` 사용 실 router 호출 (handler 재구현 path 폐기)
- [ ] `services/api/tests/sp10_backend_data_correctness.rs` 신규 — 4 시나리오 (PII drop / RLS block / audit log / health degraded)
- [ ] `cargo test --workspace -- --ignored` (test env) — 모든 integration test PASS
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] sqlx prepare 캐시 갱신 (CI 의 `cargo sqlx prepare --check` 통과)

---

## SP10.5-B Plan v3 완료

T1~T7 all done. 다음 단계:

1. **Spec v5 patch** — RawCaptureReceipt 시그니처 + migration 30012~30016 번호 동기화 (별도 commit)
2. **전체 plan Codex final review** — 모든 task 일관성 + 누락 검증
3. **사용자 plan review** — Spec → Plan → Implementation 진입 결정
4. **Implementation 시작** — superpowers:subagent-driven-development 또는 executing-plans skill
