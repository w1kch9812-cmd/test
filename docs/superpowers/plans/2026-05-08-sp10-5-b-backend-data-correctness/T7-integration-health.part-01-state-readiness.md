# T7 Integration Health - Part 01: AppState And Readiness Response

Parent index: [T7 Integration Health](./T7-integration-health.md).

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
