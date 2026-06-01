# Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02C: Admin Action Repository

Parent index: [Sub-project 5-iii Audit Pipeline Operations RDS Repository - Part 02](./2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-repository.part-02.md).

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
