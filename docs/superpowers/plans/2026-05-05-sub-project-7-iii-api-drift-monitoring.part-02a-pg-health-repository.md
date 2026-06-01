# SP7-iii API Drift Monitoring - Part 02A: Pg Health Repository

Parent index: [SP7-iii API Drift Monitoring - Part 02](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-02.md).
### Task 2: T2 — PgHealthCheckRepository (DB 인프라) + 통합 테스트

**Files:**
- Create: `crates/db/src/api_health.rs`
- Create: `crates/db/tests/api_health_integration.rs`
- Modify: `crates/db/src/lib.rs` (re-export)
- Modify: `crates/db/Cargo.toml` (`api-health-domain` 의존성 추가)

#### Step 2.1: crates/db/Cargo.toml 의존성 추가

- [ ] **Step**: `crates/db/Cargo.toml` `[dependencies]` 에 추가

```toml
api-health-domain = { path = "../operations/api-health" }
```

#### Step 2.2: lib.rs re-export

- [ ] **Step**: `crates/db/src/lib.rs` 에 추가

```rust
pub mod api_health;
```

(파일 끝 부분의 다른 `pub mod` 들 옆에)

#### Step 2.3: api_health.rs PgHealthCheckRepository 작성

- [ ] **Step**: `crates/db/src/api_health.rs` 작성

```rust
//! `PgHealthCheckRepository` — `api_health_check` 테이블 인프라 구현.
//!
//! SP7-iii 의 SSOT. `crates/operations/api-health` 의 trait 구현.

#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use api_health_domain::{
    HealthCheckRecord, HealthCheckRepository, HealthStatus, NewHealthCheck, RepoError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use tracing::instrument;

/// `api_health_check` 테이블에 대한 Postgres 구현.
#[derive(Clone)]
pub struct PgHealthCheckRepository {
    pool: Arc<PgPool>,
}

impl PgHealthCheckRepository {
    #[must_use]
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

fn map_repo_error(e: sqlx::Error) -> RepoError {
    match &e {
        sqlx::Error::Database(db_err) if db_err.is_check_violation() || db_err.is_unique_violation() => {
            RepoError::Integrity(format!("{e}"))
        }
        _ => RepoError::Database(format!("{e}")),
    }
}

fn row_to_record(row: PgRow) -> Result<HealthCheckRecord, RepoError> {
    let status_str: String = row.try_get("status").map_err(map_repo_error)?;
    let status = HealthStatus::from_str(&status_str)
        .map_err(|e| RepoError::Integrity(format!("invalid status '{status_str}': {e}")))?;
    let http_code: Option<i16> = row.try_get("http_code").map_err(map_repo_error)?;
    let duration_ms: i32 = row.try_get("duration_ms").map_err(map_repo_error)?;

    Ok(HealthCheckRecord {
        id: row.try_get("id").map_err(map_repo_error)?,
        api_name: row.try_get("api_name").map_err(map_repo_error)?,
        checked_at: row.try_get::<DateTime<Utc>, _>("checked_at").map_err(map_repo_error)?,
        status,
        http_code: http_code.map(|c| c as u16),
        error_detail: row.try_get("error_detail").map_err(map_repo_error)?,
        cron_run: row.try_get("cron_run").map_err(map_repo_error)?,
        duration_ms: duration_ms as u32,
    })
}

#[async_trait]
impl HealthCheckRepository for PgHealthCheckRepository {
    #[instrument(skip(self, new), fields(api = %new.api_name, status = %new.status))]
    async fn record(&self, new: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError> {
        let row = sqlx::query(
            r#"
            INSERT INTO api_health_check
                (api_name, status, http_code, error_detail, cron_run, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, api_name, checked_at, status, http_code,
                      error_detail, cron_run, duration_ms
            "#,
        )
        .bind(new.api_name)
        .bind(new.status.as_str())
        .bind(new.http_code.map(|c| c as i16))
        .bind(new.error_detail)
        .bind(new.cron_run)
        .bind(new.duration_ms as i32)
        .fetch_one(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        row_to_record(row)
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn is_n_cron_runs_failed(
        &self,
        api_name: &str,
        n: u32,
    ) -> Result<bool, RepoError> {
        // 최근 N 개 cron run 의 status — fail 가 모두 N 개여야 true.
        let row = sqlx::query(
            r#"
            WITH recent_cron AS (
                SELECT status
                FROM api_health_check
                WHERE api_name = $1 AND cron_run = true
                ORDER BY checked_at DESC
                LIMIT $2
            )
            SELECT
                (SELECT COUNT(*) FROM recent_cron) AS total,
                (SELECT COUNT(*) FROM recent_cron WHERE status != 'success') AS failures
            "#,
        )
        .bind(api_name)
        .bind(n as i64)
        .fetch_one(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        let total: i64 = row.try_get("total").map_err(map_repo_error)?;
        let failures: i64 = row.try_get("failures").map_err(map_repo_error)?;

        // N 개가 모두 모이고, 모두 failure 면 true.
        Ok(total == n as i64 && failures == n as i64)
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn find_latest(
        &self,
        api_name: &str,
    ) -> Result<Option<HealthCheckRecord>, RepoError> {
        let row = sqlx::query(
            r#"
            SELECT id, api_name, checked_at, status, http_code,
                   error_detail, cron_run, duration_ms
            FROM api_health_check
            WHERE api_name = $1
            ORDER BY checked_at DESC
            LIMIT 1
            "#,
        )
        .bind(api_name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_repo_error)?;

        row.map(row_to_record).transpose()
    }
}
```

#### Step 2.4: 통합 테스트 작성

- [ ] **Step**: `crates/db/tests/api_health_integration.rs` 작성

```rust
//! `PgHealthCheckRepository` 통합 테스트 — 실 Postgres 사용.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use api_health_domain::{
    HealthCheckRepository, HealthStatus, NewHealthCheck,
};
use db::api_health::PgHealthCheckRepository;
use sqlx::PgPool;

async fn setup() -> PgHealthCheckRepository {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = PgPool::connect(&url).await.expect("connect");
    sqlx::query("TRUNCATE TABLE api_health_check RESTART IDENTITY")
        .execute(&pool)
        .await
        .expect("truncate");
    PgHealthCheckRepository::new(Arc::new(pool))
}

#[tokio::test]
async fn record_success_inserts_row() {
    let repo = setup().await;
    let new = NewHealthCheck {
        api_name: "data_go_kr.getBrTitleInfo",
        status: HealthStatus::Success,
        http_code: Some(200),
        error_detail: None,
        cron_run: true,
        duration_ms: 1234,
    };
    let record = repo.record(new).await.expect("record");
    assert_eq!(record.api_name, "data_go_kr.getBrTitleInfo");
    assert_eq!(record.status, HealthStatus::Success);
    assert_eq!(record.http_code, Some(200));
    assert!(record.cron_run);
    assert_eq!(record.duration_ms, 1234);
    assert!(record.id > 0);
}

#[tokio::test]
async fn record_invalid_status_violates_check_constraint() {
    let repo = setup().await;
    // HealthStatus enum 우회 — DB CHECK constraint 만 검증
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let result = sqlx::query(
        "INSERT INTO api_health_check (api_name, status, cron_run, duration_ms)
         VALUES ('test', 'invalid_status', true, 1)",
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "CHECK constraint 가 invalid status 거부해야 함");
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_true_when_3_consecutive_failures() {
    let repo = setup().await;
    // 3 cron fail 입력
    for status in [HealthStatus::Http5xx, HealthStatus::Timeout, HealthStatus::Http5xx] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_when_one_success_in_3() {
    let repo = setup().await;
    for status in [HealthStatus::Http5xx, HealthStatus::Success, HealthStatus::Http5xx] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_ignores_manual_dispatch() {
    let repo = setup().await;
    // cron 1 fail, manual 2 fail, cron 1 fail = cron 만 보면 2 fail (2 < 3)
    let mixes = [
        (HealthStatus::Http5xx, true),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, true),
    ];
    for (status, cron) in mixes {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: cron,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap(),
        "cron 만 카운트하면 2 fail < 3");
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_with_insufficient_data() {
    let repo = setup().await;
    repo.record(NewHealthCheck {
        api_name: "test_api",
        status: HealthStatus::Http5xx,
        http_code: None,
        error_detail: None,
        cron_run: true,
        duration_ms: 100,
    })
    .await
    .unwrap();
    // 1개만 있을 때 n=3 = false (총 데이터 < 3)
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn find_latest_returns_most_recent() {
    let repo = setup().await;
    for i in 0..3 {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status: HealthStatus::Success,
            http_code: Some(200),
            error_detail: Some(&format!("call {i}")),
            cron_run: true,
            duration_ms: 100 * (i + 1),
        })
        .await
        .unwrap();
    }
    let latest = repo.find_latest("test_api").await.unwrap().expect("Some");
    assert_eq!(latest.duration_ms, 300);  // 가장 최근 = i=2
    assert_eq!(latest.error_detail, Some("call 2".to_owned()));
}

#[tokio::test]
async fn find_latest_returns_none_for_unknown_api() {
    let repo = setup().await;
    let latest = repo.find_latest("never.recorded").await.unwrap();
    assert!(latest.is_none());
}
```

- [ ] **Step**: 통합 테스트 실행

```bash
cd c:/Users/User/Desktop/gongzzang_2
# DATABASE_URL 가 .env 또는 export 로 설정돼 있어야 함
cargo test -p db --test api_health_integration
```

Expected: 8 tests passed.

#### Step 2.5: cargo check / clippy / fmt 검증

- [ ] **Step**: 전체 검증

```bash
cargo check -p db --all-features
cargo clippy -p db --all-targets -- -D warnings
cargo test -p db --lib
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 2.6: T2 commit

- [ ] **Step**: commit

```bash
git add crates/db/Cargo.toml \
        crates/db/src/lib.rs \
        crates/db/src/api_health.rs \
        crates/db/tests/api_health_integration.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t2): add PgHealthCheckRepository + integration tests

T2 of SP7-iii:
- crates/db/src/api_health.rs — PgHealthCheckRepository (HealthCheckRepository impl)
  - record() — INSERT (api_name, status, http_code, error_detail, cron_run, duration_ms)
  - is_n_cron_runs_failed(api_name, n) — 최근 N cron run 모두 fail 인가? (수동 trigger 무관)
  - find_latest(api_name) — 가장 최근 record (None | Some)
  - row_to_record helper + map_repo_error (sqlx::Error → RepoError)
  - tracing::instrument (api/status/n PII 제외 fields)
- 8 통합 테스트 (TRUNCATE 격리 + 실 Postgres):
  - record_success_inserts_row
  - record_invalid_status_violates_check_constraint
  - is_n_cron_runs_failed_returns_true_when_3_consecutive_failures
  - is_n_cron_runs_failed_returns_false_when_one_success_in_3
  - is_n_cron_runs_failed_ignores_manual_dispatch
  - is_n_cron_runs_failed_returns_false_with_insufficient_data
  - find_latest_returns_most_recent
  - find_latest_returns_none_for_unknown_api
EOF
)"
```

**사용자 체크포인트**: T2 commit 확인 + 다음 진행 여부.

---
