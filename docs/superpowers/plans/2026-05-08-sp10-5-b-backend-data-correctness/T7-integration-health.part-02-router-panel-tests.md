# T7 Integration Health - Part 02: Router Factory And Panel Endpoint Tests

Parent index: [T7 Integration Health](./T7-integration-health.md).


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
