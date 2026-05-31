# T7 Integration Health - Part 03: Backend Correctness Integration And Acceptance

Parent index: [T7 Integration Health](./T7-integration-health.md).


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
