# Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02B: Outbox Repository

Parent index: [Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02](./2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-repository.part-02.md).

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
