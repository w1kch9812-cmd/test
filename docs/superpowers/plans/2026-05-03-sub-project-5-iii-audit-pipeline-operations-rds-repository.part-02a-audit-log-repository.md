# Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02A: Audit Log Repository

Parent index: [Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02](./2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-repository.part-02.md).
### Task 3: `PgAuditLogRepository`

**Files:**
- Modify: `crates/db/src/audit_log.rs` (stub → full impl)
- Create: `crates/db/tests/audit_log_integration.rs`

**audit_log 컬럼** (실제 V001 schema):
```
id, actor_id, action, resource_kind, resource_id,
before_state (jsonb), after_state (jsonb),
correlation_id, ip_address (inet), user_agent (text),
created_at (default now())
```

**중요**: V002 immutable trigger 가 UPDATE/DELETE 차단. 본 repo 의 메서드는 `insert` + 3 finds 만.

- [ ] **Step 1: `crates/db/tests/audit_log_integration.rs` 신규**

```rust
//! `PgAuditLogRepository` 통합 테스트 — insert + 3 finds + immutable trigger.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use audit_log_domain::entity::AuditLog;
use audit_log_domain::repository::{AuditLogRepository, RepoError};
use chrono::Utc;
use db::audit_log::PgAuditLogRepository;
use shared_kernel::id::{AuditLogMarker, Id};

use common::{setup_test_pool, truncate_all};

fn make_log(action: &str, resource_id: &str) -> AuditLog {
    AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        None, // actor_id — system
        action,
        "test_resource",
        resource_id,
        None, // before_state
        None, // after_state
        "test-correlation-id",
        None, // ip_address
        None, // user_agent
        Utc::now(),
    )
    .expect("audit log")
}

#[tokio::test]
async fn insert_persists_audit_log() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool.clone());

    let log = make_log("create", "res-1");
    repo.insert(&log).await.expect("insert");

    // 직접 SELECT 로 확인 (find_by_resource 다음 테스트에서 검증)
    let count: (i64,) = sqlx::query_as("select count(*) from audit_log where id = $1")
        .bind(log.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn find_by_resource_returns_logs() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool);

    let l1 = make_log("create", "res-A");
    let l2 = make_log("update", "res-A");
    let l3 = make_log("create", "res-B");
    repo.insert(&l1).await.unwrap();
    repo.insert(&l2).await.unwrap();
    repo.insert(&l3).await.unwrap();

    let logs = repo.find_by_resource("test_resource", "res-A").await.expect("ok");
    assert_eq!(logs.len(), 2);
}

#[tokio::test]
async fn find_by_correlation_id_filters_correctly() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool);

    let log = make_log("approve", "res-X");
    repo.insert(&log).await.unwrap();

    let logs = repo.find_by_correlation_id("test-correlation-id").await.expect("ok");
    assert_eq!(logs.len(), 1);

    let none = repo.find_by_correlation_id("nonexistent-corr").await.expect("ok");
    assert_eq!(none.len(), 0);
}

#[tokio::test]
async fn immutable_trigger_blocks_update() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool.clone());

    let log = make_log("create", "res-immut");
    repo.insert(&log).await.unwrap();

    // V002 immutable trigger: UPDATE 시도 → DB 에러
    let result = sqlx::query("update audit_log set action = 'tampered' where id = $1")
        .bind(log.id.as_str())
        .execute(&pool)
        .await;
    assert!(result.is_err(), "audit_log UPDATE 가 trigger 로 차단되어야");
}
```

- [ ] **Step 2: `crates/db/src/audit_log.rs` 작성**

```rust
//! `AuditLogRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use audit_log_domain::entity::AuditLog;
use audit_log_domain::repository::{AuditLogRepository, RepoError};
use chrono::{DateTime, Utc};
use shared_kernel::id::{AuditLogMarker, Id, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `AuditLog` 의 `Postgres` 저장소. `V002` immutable trigger 가 `UPDATE`/`DELETE` 차단.
#[derive(Debug, Clone)]
pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    /// 새 저장소 생성.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const AUDIT_COLUMNS: &str = r#"
    id, actor_id, action, resource_kind, resource_id,
    before_state, after_state, correlation_id,
    host(ip_address) as ip_address_text, user_agent, created_at
"#;

fn row_to_audit_log(row: &PgRow) -> Result<AuditLog, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let actor_id_str: Option<String> = row.try_get("actor_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let action: String = row.try_get("action").map_err(|e| RepoError::Database(e.to_string()))?;
    let resource_kind: String = row.try_get("resource_kind").map_err(|e| RepoError::Database(e.to_string()))?;
    let resource_id: String = row.try_get("resource_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let before_state: Option<serde_json::Value> = row.try_get("before_state").map_err(|e| RepoError::Database(e.to_string()))?;
    let after_state: Option<serde_json::Value> = row.try_get("after_state").map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row.try_get("correlation_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let ip_address_text: Option<String> = row.try_get("ip_address_text").map_err(|e| RepoError::Database(e.to_string()))?;
    let user_agent: Option<String> = row.try_get("user_agent").map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<AuditLogMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed audit_log id: {e}")))?;
    let actor_id = actor_id_str
        .map(|s| Id::<UserMarker>::try_from_str(&s).map_err(|e| RepoError::Database(format!("malformed actor_id: {e}"))))
        .transpose()?;

    AuditLog::try_new(
        id,
        actor_id,
        &action,
        &resource_kind,
        &resource_id,
        before_state,
        after_state,
        &correlation_id,
        ip_address_text.as_deref(),
        user_agent.as_deref(),
        created_at,
    )
    .map_err(|e| RepoError::Database(format!("invalid audit_log row: {e}")))
}

#[async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    #[instrument(skip(self, log), fields(audit_id = %log.id.as_str(), action = %log.action))]
    async fn insert(&self, log: &AuditLog) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state, correlation_id,
                ip_address, user_agent, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10, $11)
            "#,
        )
        .bind(log.id.as_str())
        .bind(log.actor_id.as_ref().map(|i| i.as_str()))
        .bind(&log.action)
        .bind(&log.resource_kind)
        .bind(&log.resource_id)
        .bind(&log.before_state)
        .bind(&log.after_state)
        .bind(&log.correlation_id)
        .bind(&log.ip_address) // String — sqlx 가 inet 으로 cast
        .bind(&log.user_agent)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(resource_kind, resource_id))]
    async fn find_by_resource(
        &self,
        resource_kind: &str,
        resource_id: &str,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log where resource_kind = $1 and resource_id = $2 order by created_at desc"
        );
        let rows = sqlx::query(&sql)
            .bind(resource_kind)
            .bind(resource_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }

    #[instrument(skip(self), fields(actor_id = %actor_id.as_str()))]
    async fn find_by_actor(
        &self,
        actor_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log where actor_id = $1 order by created_at desc limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(actor_id.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }

    #[instrument(skip(self))]
    async fn find_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log where correlation_id = $1 order by created_at asc"
        );
        let rows = sqlx::query(&sql)
            .bind(correlation_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }
}
```

> `AuditLog::try_new` 시그니처 검증 필요 — 실제 도메인 메서드 확인:
> ```bash
> grep -A 15 "pub fn try_new" crates/domain/audit/audit-log/src/entity.rs
> ```
> 본 plan 은 11-arg 가정. 다르면 정정.

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db --all-features
cargo clippy -p db --all-features --all-targets -- -D warnings
cargo test -p db --lib
```

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/src/audit_log.rs crates/db/tests/audit_log_integration.rs
git commit -m "feat(db): PgAuditLogRepository — insert + 3 finds + immutable trigger 검증 (SP5-iii T3)

- row_to_audit_log: 11 필드 round-trip (Option<UserMarker> 처리, ip_address inet → String)
- insert: 단순 INSERT (V002 immutable trigger 가 UPDATE/DELETE 차단)
- find_by_resource / find_by_actor / find_by_correlation_id: 3 인덱스 활용 finds
- 모든 메서드 #[tracing::instrument] (PII 미노출)
- 4 통합 테스트 (insert / find_by_resource / find_by_correlation_id / immutable trigger 차단)"
git push
```

CI 그린 확인. 통합 테스트가 새로 작동.

---
