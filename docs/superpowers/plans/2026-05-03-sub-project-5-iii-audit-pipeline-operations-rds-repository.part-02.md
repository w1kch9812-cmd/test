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

### Task 4: `PgOutboxRepository`

**Files:**
- Modify: `crates/db/src/outbox.rs` (stub → full impl)
- Create: `crates/db/tests/outbox_integration.rs`

- [ ] **Step 1: 통합 테스트 작성**

```rust
//! `PgOutboxRepository` 통합 테스트 — save + fetch_unpublished + mark_published.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::outbox::PgOutboxRepository;
use outbox_event_domain::entity::OutboxEvent;
use outbox_event_domain::repository::{OutboxRepository, RepoError};
use shared_kernel::id::{Id, OutboxEventMarker};

use common::{setup_test_pool, truncate_all};

fn make_event(event_type: &str) -> OutboxEvent {
    OutboxEvent::try_new(
        Id::<OutboxEventMarker>::new(),
        "test_aggregate",
        "agg-id-1",
        event_type,
        serde_json::json!({"sample": "payload"}),
        "test-corr-id",
        Utc::now(),
        None, // published_at
    )
    .expect("outbox event")
}

#[tokio::test]
async fn save_persists_event() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let event = make_event("test.created");
    repo.save(&event).await.expect("save");

    let unpublished = repo.fetch_unpublished(10).await.unwrap();
    assert_eq!(unpublished.len(), 1);
}

#[tokio::test]
async fn fetch_unpublished_excludes_published() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool.clone());

    let e1 = make_event("a.created");
    let e2 = make_event("b.created");
    repo.save(&e1).await.unwrap();
    repo.save(&e2).await.unwrap();

    // 직접 SQL 로 e1 publish 표시
    sqlx::query("update outbox_event set published_at = now() where id = $1")
        .bind(e1.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let unpublished = repo.fetch_unpublished(10).await.unwrap();
    assert_eq!(unpublished.len(), 1);
    assert_eq!(unpublished[0].id.as_str(), e2.id.as_str());
}

#[tokio::test]
async fn mark_published_updates_timestamp() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let event = make_event("test.created");
    repo.save(&event).await.unwrap();

    let now = Utc::now();
    repo.mark_published(&event.id, now).await.expect("mark");

    let unpublished = repo.fetch_unpublished(10).await.unwrap();
    assert_eq!(unpublished.len(), 0);
}

#[tokio::test]
async fn mark_published_nonexistent_returns_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let id: Id<OutboxEventMarker> = Id::new();
    let err = repo.mark_published(&id, Utc::now()).await.unwrap_err();
    assert!(matches!(err, RepoError::NotFound));
}
```

- [ ] **Step 2: `crates/db/src/outbox.rs` 작성**

```rust
//! `OutboxRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use outbox_event_domain::entity::OutboxEvent;
use outbox_event_domain::repository::{OutboxRepository, RepoError};
use shared_kernel::id::{Id, OutboxEventMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `OutboxEvent` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgOutboxRepository {
    pool: PgPool,
}

impl PgOutboxRepository {
    /// 새 저장소 생성.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const OUTBOX_COLUMNS: &str = r#"
    id, aggregate_kind, aggregate_id, event_type, payload,
    correlation_id, created_at, published_at
"#;

fn row_to_outbox(row: &PgRow) -> Result<OutboxEvent, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let aggregate_kind: String = row.try_get("aggregate_kind").map_err(|e| RepoError::Database(e.to_string()))?;
    let aggregate_id: String = row.try_get("aggregate_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let event_type: String = row.try_get("event_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let payload: serde_json::Value = row.try_get("payload").map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row.try_get("correlation_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let published_at: Option<DateTime<Utc>> = row.try_get("published_at").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<OutboxEventMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed outbox id: {e}")))?;

    OutboxEvent::try_new(
        id,
        &aggregate_kind,
        &aggregate_id,
        &event_type,
        payload,
        &correlation_id,
        created_at,
        published_at,
    )
    .map_err(|e| RepoError::Database(format!("invalid outbox row: {e}")))
}

#[async_trait]
impl OutboxRepository for PgOutboxRepository {
    #[instrument(skip(self, event), fields(event_id = %event.id.as_str(), event_type = %event.event_type))]
    async fn save(&self, event: &OutboxEvent) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            insert into outbox_event (
                id, aggregate_kind, aggregate_id, event_type, payload,
                correlation_id, created_at, published_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(event.id.as_str())
        .bind(&event.aggregate_kind)
        .bind(&event.aggregate_id)
        .bind(&event.event_type)
        .bind(&event.payload)
        .bind(&event.correlation_id)
        .bind(event.created_at)
        .bind(event.published_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(limit))]
    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<OutboxEvent>, RepoError> {
        let sql = format!(
            "select {OUTBOX_COLUMNS} from outbox_event where published_at is null order by created_at asc limit $1"
        );
        let rows = sqlx::query(&sql)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_outbox).collect()
    }

    #[instrument(skip(self), fields(event_id = %id.as_str()))]
    async fn mark_published(
        &self,
        id: &Id<OutboxEventMarker>,
        published_at: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        let result = sqlx::query("update outbox_event set published_at = $1 where id = $2 and published_at is null")
            .bind(published_at)
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
```

> `OutboxEvent::try_new` 시그니처 확인 필요. `mark_published` 의 trait 시그니처도 — 실제 trait 에 `published_at: DateTime<Utc>` 인자 있는지 확인.

- [ ] **Step 3: 로컬 검증 + Commit**

```bash
cargo check -p db --all-features
cargo clippy -p db --all-features --all-targets -- -D warnings

git add crates/db/src/outbox.rs crates/db/tests/outbox_integration.rs
git commit -m "feat(db): PgOutboxRepository — save + fetch_unpublished + mark_published (SP5-iii T4)

- row_to_outbox: 8 필드 round-trip (payload jsonb)
- save: 단순 INSERT (transactional 패턴 대상 아님 — 자기 자신이 outbox)
- fetch_unpublished: WHERE published_at IS NULL ORDER BY created_at ASC LIMIT
- mark_published: UPDATE published_at = $ WHERE id = $ AND published_at IS NULL (idempotent)
- 모든 메서드 #[tracing::instrument]
- 4 통합 테스트 (save / fetch_unpublished / mark_published / NotFound)"
git push
```

---

## Phase D: Operations BC 5 repos

### Task 5: `PgAdminActionRepository` (insert-only with audit)

**Files:**
- Modify: `crates/db/src/admin_action.rs`
- Create: `crates/db/tests/admin_action_integration.rs`

**핵심 패턴**: AdminAction 자체가 admin 의 audit 성격 — 그 INSERT 도 audit_log 에 추가 기록 (메타-audit). insert-only 라 OCC 없음.

먼저 도메인 시그니처 확인:
```bash
grep -A 15 "pub fn try_new" crates/operations/admin-action/src/entity.rs
grep -nE "pub fn|pub const fn|pub struct" crates/operations/admin-action/src/entity.rs | head -10
```

- [ ] **Step 1: 통합 테스트 (`crates/db/tests/admin_action_integration.rs`)**

```rust
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use admin_action_domain::entity::AdminAction;
use admin_action_domain::repository::{AdminActionRepository, RepoError};
use chrono::Utc;
use db::admin_action::PgAdminActionRepository;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{AdminActionMarker, Id};
use shared_kernel::mutation::MutationContext;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

async fn seed_admin(pool: &sqlx::PgPool) -> Id<shared_kernel::id::UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let admin = User::try_new(
        Id::new(),
        "admin-zsub",
        Email::try_new("admin@x.com").unwrap(),
        "Admin",
        UserKind::Individual,
        now,
    )
    .unwrap();
    repo.save(&admin).await.unwrap();
    admin.id
}

#[tokio::test]
async fn insert_creates_admin_action_and_audit_log() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool).await;
    let repo = PgAdminActionRepository::new(pool.clone());

    // AdminAction 도메인 생성자 (실제 시그니처에 맞춰 — 본 plan 은 8-arg 가정)
    let action = AdminAction::try_new(
        Id::<AdminActionMarker>::new(),
        admin_id.clone(),
        "user_role_grant",
        Some("user".to_owned()),
        Some("usr_target".to_owned()),
        "role_added: Operator",
        serde_json::json!({"role": "Operator"}),
        Utc::now(),
    )
    .unwrap();

    let ctx = MutationContext::new_user_action(admin_id.clone(), "test-corr", "create");
    repo.insert(&action, ctx).await.expect("insert");

    // AdminAction row 존재 + audit_log row 존재 (transactional)
    let action_count: (i64,) = sqlx::query_as("select count(*) from admin_action where id = $1")
        .bind(action.id.as_str())
        .fetch_one(&pool).await.unwrap();
    assert_eq!(action_count.0, 1);

    let audit_count: (i64,) = sqlx::query_as("select count(*) from audit_log where resource_kind = 'admin_action' and resource_id = $1")
        .bind(action.id.as_str())
        .fetch_one(&pool).await.unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn insert_with_no_events_inserts_no_outbox() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool).await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = AdminAction::try_new(
        Id::<AdminActionMarker>::new(), admin_id.clone(),
        "feature_flag_toggle", None, None,
        "flag x toggled", serde_json::json!({}), Utc::now(),
    ).unwrap();
    let ctx = MutationContext::new_user_action(admin_id, "corr", "update"); // events 비어있음
    repo.insert(&action, ctx).await.unwrap();

    let outbox_count: (i64,) = sqlx::query_as("select count(*) from outbox_event")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(outbox_count.0, 0);
}

#[tokio::test]
async fn insert_system_action_with_no_actor_id() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let pool_for_seed = pool.clone();
    let admin_id = seed_admin(&pool_for_seed).await; // admin user 가 도메인 actor
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = AdminAction::try_new(
        Id::<AdminActionMarker>::new(), admin_id,
        "system_purge", None, None,
        "scheduled cleanup", serde_json::json!({}), Utc::now(),
    ).unwrap();
    let ctx = MutationContext::new_system_action("scheduler-run-1", "create");
    repo.insert(&action, ctx).await.unwrap();

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'admin_action' and actor_id is null"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn insert_with_metadata_serialized_to_after_state() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool).await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = AdminAction::try_new(
        Id::<AdminActionMarker>::new(), admin_id.clone(),
        "verify_business", Some("user".to_owned()), Some("usr_x".to_owned()),
        "verified", serde_json::json!({}), Utc::now(),
    ).unwrap();
    let ctx = MutationContext::new_user_action(admin_id, "corr", "create")
        .with_metadata(serde_json::json!({"verification_id": "v-123"}));
    repo.insert(&action, ctx).await.unwrap();

    // audit_log.after_state = ctx.metadata
    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log where resource_kind = 'admin_action'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(after_state, Some(serde_json::json!({"verification_id": "v-123"})));
}
```

- [ ] **Step 2: `crates/db/src/admin_action.rs` 작성**

핵심 패턴 (반복적):
```rust
#[async_trait]
impl AdminActionRepository for PgAdminActionRepository {
    #[instrument(skip(self, action, ctx), fields(action_id = %action.id.as_str(), kind = %action.action_kind, ctx_action = %ctx.action))]
    async fn insert(
        &self,
        action: &AdminAction,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;
        let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);

        // 1. INSERT admin_action (실제 컬럼명/순서 — entity 확인)
        sqlx::query(
            r#"
            insert into admin_action (
                id, admin_id, action_kind, target_kind, target_id,
                description, metadata, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(action.id.as_str())
        .bind(action.admin_id.as_str())
        .bind(&action.action_kind)
        .bind(&action.target_kind)
        .bind(&action.target_id)
        .bind(&action.description)
        .bind(&action.metadata)
        .bind(action.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2. INSERT audit_log
        let audit_id_str = format!("aud_{}", ulid::Ulid::new());
        sqlx::query(
            r#"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state, correlation_id,
                ip_address, user_agent, created_at
            )
            values ($1, $2, $3, 'admin_action', $4, NULL, $5, $6, $7::inet, $8, $9)
            "#,
        )
        .bind(&audit_id_str)
        .bind(ctx.actor_id.as_ref().map(|i| i.as_str()))
        .bind(&ctx.action)
        .bind(action.id.as_str())
        .bind(&ctx.metadata)
        .bind(&ctx.correlation_id)
        .bind(&ctx.client_ip)
        .bind(&ctx.user_agent)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. INSERT outbox_event for each event in ctx.events
        for event in &ctx.events {
            let outbox_id_str = format!("evt_{}", ulid::Ulid::new());
            sqlx::query(
                r#"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'admin_action', $2, $3, $4, $5, $6, NULL)
                "#,
            )
            .bind(&outbox_id_str)
            .bind(action.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    // find_by_admin / find_by_target — read-only, ctx 없음
    // (도메인 trait 의 finds 메서드들 그대로)
}
```

> `ulid` crate dep 필요할 수 있음. `Cargo.toml workspace deps` 에 이미 `ulid = "1.1"` 있음 — 본 db crate 에 dep 추가:
> ```toml
> ulid = { workspace = true }
> ```
> 또는 도메인의 `Id::new()` 활용:
> ```rust
> let audit_id = Id::<AuditLogMarker>::new();
> .bind(audit_id.as_str())
> ```
> 후자가 도메인 패턴 따름 — 권장. shared_kernel deps 에 `AuditLogMarker` import.

- [ ] **Step 3: 로컬 검증 + Commit**

```bash
cargo check -p db --all-features
cargo clippy -p db --all-features --all-targets -- -D warnings

git add crates/db/src/admin_action.rs crates/db/tests/admin_action_integration.rs
git commit -m "feat(db): PgAdminActionRepository — insert-only + transactional audit/outbox (SP5-iii T5)

- insert: tx 안에서 INSERT admin_action + INSERT audit_log + INSERT outbox_event for each event
- ctx.metadata → audit_log.after_state, ctx.actor_id → audit_log.actor_id
- system action 시 actor_id NULL (cron / scheduler 호출)
- 모든 메서드 #[tracing::instrument]
- 4 통합 테스트 (insert + audit / no events → no outbox / system action / metadata → after_state)"
git push
```

---

