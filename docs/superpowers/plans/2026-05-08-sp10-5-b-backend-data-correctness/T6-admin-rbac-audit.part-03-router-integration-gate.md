# T6 Vault Admin RBAC Audit - Part 03: Router Mount And Integration Gate

Parent index: [T6 Vault Admin RBAC Audit](./T6-admin-rbac-audit.md).

## Step 6.4: Mount admin router in main.rs

- [ ] **Step 6.4.1: Modify `services/api/src/main.rs` — add admin route**

main.rs 의 axum router 빌더에 추가:

```rust
mod routes;

// ... (기존 router 빌더 위치)
let admin_state = routes::admin::raw_vault::AdminState {
    pool: pool.clone(),
    kms: kms_client.clone(),
    access_log: gongzzang_db::PgVaultAccessLog::new(pool.clone()),
};
let admin_router = axum::Router::new()
    .route(
        "/api/admin/raw_vault/:source/:pnu",
        axum::routing::get(routes::admin::raw_vault::get_vault_handler),
    )
    .with_state(admin_state);
let app = app.merge(admin_router);
```

- [ ] **Step 6.4.2: Verify build**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 6.4.3: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-5-b-T6): mount admin raw_vault router on /api/admin/*"
```

---

## Step 6.5: Integration test (TDD with axum-test)

- [ ] **Step 6.5.1: Append axum-test based integration tests to `raw_vault.rs`**

```rust
    use axum::Router;
    use axum_test::TestServer;

    fn test_app(pool: PgPool, kms: Arc<aws_sdk_kms::Client>) -> Router {
        let state = AdminState {
            pool: pool.clone(),
            kms,
            access_log: PgVaultAccessLog::new(pool),
        };
        Router::new()
            .route(
                "/api/admin/raw_vault/:source/:pnu",
                axum::routing::get(get_vault_handler),
            )
            .with_state(state)
    }

    fn admin_jwt() -> String {
        // 테스트용 JWT — production 은 ZITADEL 발급
        "Bearer test-admin-jwt-with-admin-role".to_string()
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS"]
    async fn missing_purpose_returns_400() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let kms = Arc::new(aws_sdk_kms::Client::new(&kms_config));
        let server = TestServer::new(test_app(pool, kms)).unwrap();
        let response = server
            .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010003")
            .add_header("authorization", admin_jwt())
            .add_query_param("ticket_id", "TICKET-1")
            // purpose 누락
            .await;
        assert_eq!(response.status_code(), 400);
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS + admin JWT"]
    async fn non_admin_returns_403() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let kms = Arc::new(aws_sdk_kms::Client::new(&kms_config));
        let server = TestServer::new(test_app(pool, kms)).unwrap();
        let response = server
            .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010003")
            .add_header("authorization", "Bearer test-user-jwt-no-admin-role")
            .add_query_param("purpose", "drift_diagnosis")
            .add_query_param("ticket_id", "TICKET-1")
            .await;
        assert_eq!(response.status_code(), 403);
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS + admin JWT + pre-populated vault"]
    async fn successful_access_creates_audit_log() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        // ... (vault row 사전 INSERT, KMS Decrypt 가능한 key_id 사용)
        // 응답 200 + raw_vault_access_log 새 row 1건 검증
    }
```

- [ ] **Step 6.5.2: Run tests (ignored unless test env)**

```bash
cargo test -p api --lib routes::admin -- --ignored
# Expected (with env): 3 passed
# Expected (without env): 3 ignored
```

- [ ] **Step 6.5.3: Commit**

```bash
git add services/api/src/routes/admin/raw_vault.rs
git commit -m "test(sp10-5-b-T6): admin raw_vault integration tests (axum-test)"
```

---

## Acceptance — T6 완료 기준

- [ ] `migrations/30015_raw_vault_access_log.sql` 적용 (7 columns + 3 indexes)
- [ ] `crates/db/src/access_log.rs` — `PgVaultAccessLog::record` impl + unit test PASS
- [ ] `services/api/src/routes/admin/raw_vault.rs` — handler impl (parse_purpose, AdminAuth extractor, KMS decrypt)
- [ ] `cargo test -p api --lib routes::admin -- --ignored` (test env 있을 시) — 3 passed (400/403/200 + audit)
- [ ] Admin route `/api/admin/raw_vault/:source/:pnu` mounted in main.rs
- [ ] audit INSERT 실패 시 vault SELECT 차단 (fail-fast) — 코드 path 검증

**다음 task:** [T7-integration-health.md](T7-integration-health.md) — services/api state.rs/lib.rs 분리 + ReadinessResponse + sp10_panel_endpoints.rs 재작성.
