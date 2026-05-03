# Sub-project 5-iii: Audit + Pipeline + Operations BC RDS Repo + 트랜잭션 Outbox — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.
>
> **CRITICAL pre-read:** [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) + [docs/superpowers/specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md](../specs/2026-05-03-sub-project-5-iii-audit-pipeline-operations-rds-design.md)

**Goal:** 8 신규 PgRepository 구현 + `MutationContext` + Aggregate save / `audit_log` INSERT / `outbox_event` INSERT 가 같은 트랜잭션 안에서 atomic 실행되는 패턴 도입.

**Architecture:** `crates/domain/core/shared-kernel/src/mutation.rs` 에 `MutationContext` 정의. 6 도메인 trait 의 mutation 메서드 시그니처에 `ctx: MutationContext` 추가. `crates/db/src/{audit_log,outbox,pipeline,admin_action,bvq,lrq,listing_report,operations_meta}.rs` 8 파일 신규. 각 PgImpl 가 `pool.begin()` → 3 INSERT/UPDATE → `tx.commit()` 패턴.

**Tech Stack:** Rust 1.88, sqlx 0.8 (Transaction + Executor), Postgres 17 + audit_log V002 immutable trigger, async-trait, tracing, serde_json.

**환경**: 로컬 cargo 작동 (MSVC). 통합 테스트는 CI walking-skeleton 의 PG 컨테이너에서 실행. SP5-i T5 lesson — `--test-threads=1` + 통합 테스트 후 `truncate cascade` reset.

**Repo**: `https://github.com/w1kch9812-cmd/test` (public, Actions free).

---

## Schema 정정 — Spec § 4.3 와 실제 스키마 차이

Spec mock 의 `audit_log` INSERT 컬럼이 실제 `migrations/10003_system_tables.sql` 스키마와 다름:

| Spec mock | 실제 스키마 |
|---|---|
| `metadata` JSONB | `before_state` JSONB + `after_state` JSONB |
| `client_ip` (text?) | `ip_address` (`inet`) |
| `occurred_at` | `created_at` |

본 plan 의 SQL 은 *실제 스키마* 따름. `MutationContext.metadata` 는 `after_state` 로 매핑. `before_state` 는 SP5-iii 에서는 `NULL` (full diff 캡처는 후속 — Spec FU 13).

`outbox_event` 컬럼: `id, aggregate_kind, aggregate_id, event_type, payload, correlation_id, created_at, published_at`. Spec 본문과 일치.

---

## Task 분해 (11 task)

- **Phase A (T1):** `MutationContext` + 6 도메인 trait 시그니처 변경
- **Phase B (T2):** `error_map.rs` 에 8 신규 도메인 `MapFromSqlx` impl + `db` Cargo.toml deps
- **Phase C (T3-T4):** Audit infrastructure — `PgAuditLogRepository`, `PgOutboxRepository`
- **Phase D (T5-T9):** Operations BC 5 repos
- **Phase E (T10):** `PgPipelineRepository` (2 aggregates)
- **Phase F (T11):** 통합 검증 + memory 갱신

각 task: 로컬 `cargo check / clippy / test --lib` → push → CI 통합 테스트.

---

## File Structure

신규:
```
crates/domain/core/shared-kernel/src/mutation.rs      (MutationContext)

crates/db/src/
├── audit_log.rs          (~200줄)
├── outbox.rs             (~180줄)
├── admin_action.rs       (~180줄)
├── bvq.rs                (~250줄)
├── lrq.rs                (~250줄)
├── listing_report.rs     (~200줄)
├── operations_meta.rs    (~280줄, 2 aggregates)
└── pipeline.rs           (~280줄, 2 aggregates)

crates/db/tests/
├── audit_log_integration.rs           (~4 tests)
├── outbox_integration.rs              (~4 tests)
├── admin_action_integration.rs        (~4 tests)
├── bvq_integration.rs                 (~5 tests)
├── lrq_integration.rs                 (~4 tests)
├── listing_report_integration.rs      (~4 tests)
├── operations_meta_integration.rs     (~5 tests)
└── pipeline_integration.rs            (~5 tests)
```

수정:
```
crates/domain/core/shared-kernel/src/lib.rs                         (pub mod mutation 추가)
crates/domain/core/shared-kernel/Cargo.toml                         (serde_json workspace dep — already)
crates/data-pipeline-control/src/repository.rs                      (save_schedule/save_run + ctx)
crates/operations/admin-action/src/repository.rs                    (insert + ctx)
crates/operations/business-verification-queue/src/repository.rs     (save + ctx)
crates/operations/listing-review-queue/src/repository.rs            (save + ctx)
crates/operations/listing-report/src/repository.rs                  (save + ctx)
crates/operations/operations-meta/src/repository.rs                 (save_featured/save_alert + ctx)
crates/db/src/error_map.rs                                          (8 MapFromSqlx impl)
crates/db/src/lib.rs                                                (pub mod 8개)
crates/db/Cargo.toml                                                (8 도메인 deps)
```

---

## Phase A: 핵심 추상화

### Task 1: `MutationContext` + 6 도메인 trait 시그니처 변경

**Files:**
- Create: `crates/domain/core/shared-kernel/src/mutation.rs`
- Modify: `crates/domain/core/shared-kernel/src/lib.rs`
- Modify: 6 도메인 `repository.rs` 파일 (위 File Structure 참조)

- [ ] **Step 1: `crates/domain/core/shared-kernel/src/mutation.rs` 작성**

```rust
//! `MutationContext` — 모든 audit/outbox transactional save 의 입력.

#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::domain_event::DomainEvent;
use crate::id::{Id, UserMarker};

/// 모든 mutation 의 audit/outbox 컨텍스트.
///
/// 호출자 (application layer) 가 누가/왜/무엇을 명시. `PgRepository` 가 트랜잭션
/// 안에서 `audit_log` / `outbox_event` `INSERT` 를 자동 수행해요.
///
/// 시스템 mutation (pipeline/scheduler) 은 [`Self::new_system_action`] 사용 —
/// `actor_id = None`.
#[derive(Debug, Clone)]
pub struct MutationContext {
    /// 누가 (`None` = system action — pipeline scheduler 등).
    pub actor_id: Option<Id<UserMarker>>,
    /// `HTTP` 요청 `ID` 또는 pipeline run `ID` (구조적 로그 / `Tempo` 연결).
    pub correlation_id: String,
    /// 도메인 의미 (예: `"create"`, `"update"`, `"approve"`, `"reject"`,
    /// `"acknowledge"`). `"save"` 같은 무의미 값 금지.
    pub action: String,
    /// 추가 메타데이터 — `audit_log.after_state` `JSONB` 로 매핑.
    pub metadata: Option<Value>,
    /// 본 mutation 이 발행하는 도메인 이벤트들 (`Outbox` 로 전파).
    pub events: Vec<Arc<dyn DomainEvent>>,
    /// 클라이언트 `IP` (`HTTP` 요청 시).
    pub client_ip: Option<String>,
    /// 클라이언트 `User-Agent`.
    pub user_agent: Option<String>,
    /// mutation 발생 시각. `None` 이면 `PgRepository` 가 `Utc::now()` 사용.
    pub occurred_at: Option<DateTime<Utc>>,
}

impl MutationContext {
    /// 인증된 사용자가 일으킨 mutation.
    #[must_use]
    pub fn new_user_action(
        actor_id: Id<UserMarker>,
        correlation_id: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            actor_id: Some(actor_id),
            correlation_id: correlation_id.into(),
            action: action.into(),
            metadata: None,
            events: Vec::new(),
            client_ip: None,
            user_agent: None,
            occurred_at: None,
        }
    }

    /// 시스템 (pipeline / scheduler / cron) 이 일으킨 mutation.
    #[must_use]
    pub fn new_system_action(
        correlation_id: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            actor_id: None,
            correlation_id: correlation_id.into(),
            action: action.into(),
            metadata: None,
            events: Vec::new(),
            client_ip: None,
            user_agent: None,
            occurred_at: None,
        }
    }

    /// 추가 메타데이터 부여 (builder).
    #[must_use]
    pub fn with_metadata(mut self, m: Value) -> Self {
        self.metadata = Some(m);
        self
    }

    /// 도메인 이벤트 부여 (builder).
    #[must_use]
    pub fn with_events(mut self, events: Vec<Arc<dyn DomainEvent>>) -> Self {
        self.events = events;
        self
    }

    /// 클라이언트 정보 부여 (builder).
    #[must_use]
    pub fn with_client_info(
        mut self,
        ip: impl Into<String>,
        ua: impl Into<String>,
    ) -> Self {
        self.client_ip = Some(ip.into());
        self.user_agent = Some(ua.into());
        self
    }

    /// 발생 시각 부여 (builder, 테스트 결정성용).
    #[must_use]
    pub const fn with_occurred_at(mut self, at: DateTime<Utc>) -> Self {
        self.occurred_at = Some(at);
        self
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::id::Id;

    #[test]
    fn new_user_action_sets_actor() {
        let actor: Id<UserMarker> = Id::new();
        let ctx = MutationContext::new_user_action(actor.clone(), "req-1", "approve");
        assert_eq!(ctx.actor_id.as_ref().map(|i| i.as_str()), Some(actor.as_str()));
        assert_eq!(ctx.correlation_id, "req-1");
        assert_eq!(ctx.action, "approve");
        assert!(ctx.events.is_empty());
        assert!(ctx.metadata.is_none());
    }

    #[test]
    fn new_system_action_no_actor() {
        let ctx = MutationContext::new_system_action("plr-1", "create");
        assert!(ctx.actor_id.is_none());
        assert_eq!(ctx.action, "create");
    }

    #[test]
    fn with_metadata_chainable() {
        let ctx = MutationContext::new_system_action("c", "update")
            .with_metadata(serde_json::json!({"reason": "test"}));
        assert!(ctx.metadata.is_some());
    }

    #[test]
    fn with_client_info_sets_both() {
        let ctx = MutationContext::new_system_action("c", "create")
            .with_client_info("10.0.0.1", "Mozilla/5.0");
        assert_eq!(ctx.client_ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(ctx.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    #[test]
    fn with_occurred_at_sets_time() {
        let now = Utc::now();
        let ctx = MutationContext::new_system_action("c", "create").with_occurred_at(now);
        assert_eq!(ctx.occurred_at, Some(now));
    }

    #[test]
    fn with_events_replaces_vec() {
        let ctx = MutationContext::new_system_action("c", "create")
            .with_events(vec![]); // empty replacement to verify chainable
        assert!(ctx.events.is_empty());
    }
}
```

- [ ] **Step 2: `crates/domain/core/shared-kernel/src/lib.rs` 갱신**

기존 `pub mod` 선언들 사이에 추가 (알파벳 순서):
```rust
pub mod mutation;
```

- [ ] **Step 3: 6 도메인 `repository.rs` 시그니처 변경**

각 파일 import 추가 + save 시그니처 변경.

**`crates/data-pipeline-control/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn save_schedule(
    &self,
    schedule: &PipelineSchedule,
    ctx: MutationContext,
) -> Result<(), RepoError>;

async fn save_run(
    &self,
    run: &PipelineRun,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

**`crates/operations/admin-action/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn insert(
    &self,
    action: &AdminAction,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

**`crates/operations/business-verification-queue/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn save(
    &self,
    bvq: &BusinessVerificationQueue,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

**`crates/operations/listing-review-queue/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn save(
    &self,
    lrq: &ListingReviewQueue,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

**`crates/operations/listing-report/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn save(
    &self,
    report: &ListingReport,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

**`crates/operations/operations-meta/src/repository.rs`**:
```rust
use shared_kernel::mutation::MutationContext;
// ...
async fn save_featured(
    &self,
    fc: &FeaturedContent,
    ctx: MutationContext,
) -> Result<(), RepoError>;

async fn save_alert(
    &self,
    alert: &SystemAlert,
    ctx: MutationContext,
) -> Result<(), RepoError>;
```

`AuditLogRepository` (`crates/domain/audit/audit-log/src/repository.rs`): **변경 없음** — insert-only, audit 자체.
`OutboxRepository` (`crates/domain/audit/outbox-event/src/repository.rs`): **변경 없음** — outbox 자체.

- [ ] **Step 4: 로컬 검증**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo check --workspace
cargo clippy --workspace --all-features -- -D warnings
cargo test -p shared-kernel --lib  # MutationContext 6 unit tests
```

Expected: 컴파일 통과 (PgImpl 가 아직 새 시그니처 따라가지 않음 — Walking Skeleton 의 PgUserRepository 만 있고 본 6 도메인은 PgImpl 없음 — 컴파일 깨질 게 없음).

- [ ] **Step 5: Commit + push**

```bash
git add crates/domain/core/shared-kernel/src/mutation.rs \
        crates/domain/core/shared-kernel/src/lib.rs \
        crates/data-pipeline-control/src/repository.rs \
        crates/operations/admin-action/src/repository.rs \
        crates/operations/business-verification-queue/src/repository.rs \
        crates/operations/listing-review-queue/src/repository.rs \
        crates/operations/listing-report/src/repository.rs \
        crates/operations/operations-meta/src/repository.rs
git commit -m "feat(shared-kernel): MutationContext + 6 domain trait save signatures (SP5-iii T1)

- shared-kernel/src/mutation.rs: MutationContext + 4 builders + 6 unit tests
  · new_user_action / new_system_action / with_metadata / with_events / with_client_info / with_occurred_at
  · actor_id (Option), correlation_id, action, metadata, events (Vec<Arc<dyn DomainEvent>>),
    client_ip, user_agent, occurred_at
- 6 domain Repository trait save/insert 메서드 시그니처 변경 (ctx: MutationContext 추가)
  · pipeline (save_schedule, save_run)
  · admin-action (insert)
  · bvq, lrq, listing-report (save)
  · operations-meta (save_featured, save_alert)
- AuditLogRepository / OutboxRepository 는 변경 없음 (audit/outbox 자체는 transactional 패턴 대상 아님)

PgImpl 들은 T3-T10 에서 추가 — 본 task 는 trait 정의만"
git push
gh run list --branch main --limit 3
gh run watch <CI-run-id> --exit-status
```

3 워크플로우 그린 확인.

---

## Phase B: 공통 인프라

### Task 2: `error_map.rs` 8 신규 `MapFromSqlx` impl + `db` Cargo.toml deps

**Files:**
- Modify: `crates/db/Cargo.toml`
- Modify: `crates/db/src/error_map.rs`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: `crates/db/Cargo.toml` 8 도메인 deps 추가**

기존 deps 다음에 (alphabetic):
```toml
[dependencies]
# ... 기존 ...
admin-action-domain = { path = "../operations/admin-action", version = "0.1.0" }
audit-log-domain = { path = "../domain/audit/audit-log", version = "0.1.0" }
bvq-domain = { path = "../operations/business-verification-queue", version = "0.1.0" }
data-pipeline-control = { path = "../data-pipeline-control", version = "0.1.0" }
listing-domain = { path = "../domain/core/listing", version = "0.1.0" }      # 이미 있음
listing-photo-domain = { path = "../domain/core/listing-photo", version = "0.1.0" }  # 이미 있음
listing-report-domain = { path = "../operations/listing-report", version = "0.1.0" }
lrq-domain = { path = "../operations/listing-review-queue", version = "0.1.0" }
operations-meta-domain = { path = "../operations/operations-meta", version = "0.1.0" }
outbox-event-domain = { path = "../domain/audit/outbox-event", version = "0.1.0" }
shared-kernel = { path = "../domain/core/shared-kernel", version = "0.1.0" }  # 이미 있음
user-domain = { path = "../domain/core/user", version = "0.1.0" }              # 이미 있음
```

각 crate 의 실제 `[package].name` 확인 후 정정:
```bash
grep -A 1 '\[package\]' crates/operations/admin-action/Cargo.toml | head -3
# ...
```

- [ ] **Step 2: `crates/db/src/error_map.rs` 8 impl 추가**

기존 3 impl (user / listing / listing-photo) 끝에 추가:

```rust
impl MapFromSqlx for audit_log_domain::repository::RepoError {
    fn conflict() -> Self {
        // audit_log 는 immutable, OCC 없음. unique violation 도 ULID 자동 생성으로 발생 안 함.
        // 여기 도달했다면 비정상 — Database 로 fallback.
        Self::Database("unexpected conflict in audit_log".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for outbox_event_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in outbox_event".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for data_pipeline_control::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for admin_action_domain::repository::RepoError {
    fn conflict() -> Self {
        // AdminAction 은 insert-only. id 중복 시 Conflict.
        Self::Database("unexpected conflict in admin_action".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for bvq_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for lrq_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for listing_report_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Conflict
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}

impl MapFromSqlx for operations_meta_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in operations_meta".to_owned())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}
```

> 실제 도메인 crate 이름은 `Cargo.toml [package].name` 따라 — 위는 추정. 첫 cargo check 에서 확인.
> 각 도메인의 RepoError variant 명도 확인 — `Conflict` 가 있는지 / `NotFound` 만 있는지. 없는 도메인은 fallback 으로 `Database`.

- [ ] **Step 3: `crates/db/src/lib.rs` 8 신규 `pub mod` 선언**

```rust
//! `SQLx` `Postgres` `Repository` 구현체.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin_action;
pub mod audit_log;
pub mod bvq;
pub mod error_map;
pub mod listing;
pub mod listing_photo;
pub mod listing_report;
pub mod lrq;
pub mod operations_meta;
pub mod outbox;
pub mod pipeline;
pub mod user;
```

- [ ] **Step 4: 8 stub 파일 생성** (T3-T10 에서 채울)

```bash
for f in audit_log outbox admin_action bvq lrq listing_report operations_meta pipeline; do
  cat > "crates/db/src/${f}.rs" <<EOF
//! \`Pg${f}Repository\` (placeholder, 후속 task 에서 구현).
EOF
done
```

각 stub 파일 그냥 doc-comment 하나만:
```rust
//! `PgAuditLogRepository` (placeholder, T3 에서 구현).
```

- [ ] **Step 5: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features --all-targets -- -D warnings
```

Expected: 8 도메인 dep 등록 + module 선언 통과. 기존 user/listing/listing_photo unit + 2 error_map unit 통과.

- [ ] **Step 6: Commit + push**

```bash
git add crates/db/Cargo.toml crates/db/src/error_map.rs crates/db/src/lib.rs \
        crates/db/src/audit_log.rs crates/db/src/outbox.rs \
        crates/db/src/admin_action.rs crates/db/src/bvq.rs crates/db/src/lrq.rs \
        crates/db/src/listing_report.rs crates/db/src/operations_meta.rs crates/db/src/pipeline.rs
git commit -m "feat(db): 8 신규 도메인 deps + MapFromSqlx impls + module stubs (SP5-iii T2)

- db Cargo.toml: 8 도메인 (audit-log, outbox-event, pipeline, admin-action, bvq,
  lrq, listing-report, operations-meta) deps 추가
- error_map.rs: 8 신규 RepoError MapFromSqlx impl (Conflict 없는 도메인은 Database fallback)
- lib.rs: 8 신규 module 선언 (audit_log, outbox, admin_action, bvq, lrq,
  listing_report, operations_meta, pipeline) — 본 task 는 stub 만"
git push
```

3 워크플로우 그린 확인.

---

## Phase C: Audit Infrastructure

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

### Task 6: `PgBvqRepository` (OCC + transactional)

**Files:**
- Modify: `crates/db/src/bvq.rs`
- Create: `crates/db/tests/bvq_integration.rs`

`BVQ` 의 OCC + UPSERT + audit + outbox 패턴. 핵심 시나리오:
1. `save` 첫 호출 → INSERT
2. `save` 재호출 → UPDATE WHERE version = $ → version + 1
3. version mismatch → tx rollback (audit/outbox 도 안 들어감)
4. tx 안 events → outbox INSERT 동기

**Files (구체 코드)**: 위 T5 패턴 + OCC. 도메인 시그니처는 `crates/operations/business-verification-queue/src/entity.rs` 참고:
```bash
grep -A 20 "pub struct BusinessVerificationQueue" crates/operations/business-verification-queue/src/entity.rs
```

도메인 12 컬럼:
- id, applicant_id, business_number, business_kind, status (enum), submitted_at, reviewer_id, reviewer_note, reviewed_at, sla_due_at, version, submitted_documents (jsonb)

코드 구조 (T5 패턴 따라):
```rust
#[instrument(skip(self, bvq, ctx), fields(bvq_id = %bvq.id.as_str(), action = %ctx.action, version = bvq.version))]
async fn save(&self, bvq: &BusinessVerificationQueue, ctx: MutationContext) -> Result<(), RepoError> {
    let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

    // 1. INSERT or UPDATE BVQ (OCC)
    let result = sqlx::query(r#"
        insert into business_verification_queue (...)
        values (...)
        on conflict (id) do update set
            status = excluded.status,
            reviewer_id = excluded.reviewer_id,
            reviewer_note = excluded.reviewer_note,
            reviewed_at = excluded.reviewed_at,
            submitted_documents = excluded.submitted_documents,
            version = business_verification_queue.version + 1
        where business_verification_queue.version = $11
    "#)
    // ... binds (12 columns)
    .execute(&mut *tx).await.map_err(map_sqlx_err)?;
    if result.rows_affected() == 0 {
        return Err(RepoError::Conflict);
    }

    // 2. audit_log INSERT (resource_kind='bvq')
    // 3. outbox_event INSERT for each event
    // 4. tx.commit()
}
```

5 통합 테스트:
1. `save_inserts_bvq_audit_outbox_in_one_tx`
2. `save_with_events_inserts_each_outbox`
3. `occ_version_mismatch_rolls_back_audit` — version 안 맞으면 audit_log 도 안 들어감
4. `save_user_action_records_actor_id`
5. `save_with_metadata_serializes_after_state`

상세 코드는 T5 패턴과 거의 동일. (생략 — implementer subagent 가 T5 + entity 정보로 구성)

```bash
git commit -m "feat(db): PgBvqRepository — OCC + transactional audit/outbox (SP5-iii T6)

- INSERT or UPDATE WHERE version = $ (OCC)
- 0 rows_affected → Conflict (tx auto-rollback, audit_log 도 안 들어감)
- ctx.events → outbox_event for each
- 5 통합 테스트 (insert+audit / events→outbox / OCC rollback / actor_id / metadata→after_state)"
git push
```

---

### Task 7: `PgLrqRepository` (OCC + transactional)

T6 와 동일 패턴. LRQ 도메인 컬럼 (12) + decision Option<LrqDecision>. 4 통합 테스트.

```bash
grep -A 25 "pub struct ListingReviewQueue" crates/operations/listing-review-queue/src/entity.rs
```

상세 코드는 T6 미러. 4 tests:
1. save_inserts_lrq_audit_outbox
2. occ_version_mismatch_rolls_back_audit
3. save_decision_approve
4. save_with_no_events_no_outbox

```bash
git commit -m "feat(db): PgLrqRepository — OCC + transactional audit/outbox (SP5-iii T7)"
git push
```

---

### Task 8: `PgListingReportRepository` (no OCC, transactional)

T5 패턴 (insert-only-ish — 단, 상태 update 가능). 4 통합 테스트.

```bash
grep -A 20 "pub struct ListingReport" crates/operations/listing-report/src/entity.rs
```

ListingReport 컬럼: id, listing_id, reporter_id (Option), reason (enum), description, status (enum), reviewer_id (Option), reviewer_note (Option), created_at, updated_at, resolved_at (Option) — OCC 없음.

```bash
git commit -m "feat(db): PgListingReportRepository — transactional audit/outbox (no OCC) (SP5-iii T8)"
git push
```

---

### Task 9: `PgOperationsMetaRepository` (2 aggregates)

`save_featured` + `save_alert` 두 메서드 모두 ctx 받음. 각자 audit/outbox 처리.

**중요**: `OperationsMetaRepository` trait 의 finds 메서드도 함께 구현 (`find_featured_by_id`, `find_active_featured`, `find_alert_by_id`, `find_unacknowledged_alerts`). 이들은 read-only, ctx 없음.

5 통합 테스트:
1. save_featured_inserts_with_audit_and_outbox
2. find_active_featured_filters_by_time
3. save_alert_with_metadata
4. find_unacknowledged_alerts_excludes_acked
5. save_alert_with_no_events_no_outbox

```bash
git commit -m "feat(db): PgOperationsMetaRepository — 2 aggregates + transactional (SP5-iii T9)"
git push
```

---

## Phase E: Pipeline

### Task 10: `PgPipelineRepository` (2 aggregates)

`save_schedule(s, ctx)` + `save_run(r, ctx)`. `PipelineSchedule` 은 schedule (cron + lock), `PipelineRun` 은 1번 실행. 둘 다 OCC 없음 (`version` 필드 없음).

도메인 컬럼:
- pipeline_schedule: id, name, cron, enabled, last_run_at, next_run_at, lock_until, lock_owner, created_at, updated_at, config (jsonb)
- pipeline_run: id, schedule_id, status (enum), started_at, finished_at, error_message, steps (jsonb), trigger_kind

5 통합 테스트:
1. save_schedule_with_audit
2. save_run_with_audit_and_outbox
3. find_schedule_by_id
4. find_active_schedules
5. system_action_no_actor

```bash
grep -A 20 "pub struct PipelineSchedule\|pub struct PipelineRun" crates/data-pipeline-control/src/

git commit -m "feat(db): PgPipelineRepository — 2 aggregates + system actions (SP5-iii T10)"
git push
```

---

## Phase F: 종료

### Task 11: 통합 검증 + project_progress 갱신

**Files:**
- Modify: `MEMORY.md`
- Modify: `memory/project_progress.md`

- [ ] **Step 1: 누적 카운트**

```bash
grep -rE '#\[(tokio::)?test\]' crates/ services/ --include="*.rs" | wc -l
# 통합 테스트만
grep -rE '#\[(tokio::)?test\]' crates/db/tests/ --include="*.rs" | wc -l
```

목표: 1075 (SP5-i 종료) + 6 unit (MutationContext) + ~30 integration = ~1110+.

- [ ] **Step 2: `MEMORY.md` 갱신**

```diff
- - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3+5-i 완료 (25 crate, ~1075 tests)...
+ - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3+5-i+5-iii 완료 (25 crate, ~1110 tests)...
```

- [ ] **Step 3: `memory/project_progress.md` 에 SP5-iii 절 추가**

기존 SP5-i 절 *직후* 에:

```markdown
### Sub-project 5-iii: Audit + Pipeline + Operations BC RDS Repo + 트랜잭션 Outbox (완료, T1-T11)

- 신규: `MutationContext` (`crates/domain/core/shared-kernel/src/mutation.rs`) + 6 단위 테스트
- 신규: 8 PgRepository (`crates/db/src/{audit_log,outbox,admin_action,bvq,lrq,listing_report,operations_meta,pipeline}.rs`)
- 6 도메인 trait 시그니처 변경 — `save`/`insert` 메서드에 `ctx: MutationContext` 추가
- `error_map.rs` 8 신규 도메인 `MapFromSqlx` impl
- **트랜잭션 패턴**: PgRepository.save() 가 tx 안에서 [INSERT/UPDATE Aggregate + INSERT audit_log + INSERT outbox_event for each event] 모두 atomic. 부분 실패 → 모두 rollback
- AuditLog/Outbox 자체 repo 는 transactional 패턴 대상 아님 (recursion 방지)
- 통합 테스트 ~30 + 단위 6 → 누적 ~1110

**SSS 7기둥 결함 닫음**:
- 추적성: 모든 mutation 이 audit_log 자동 + correlation_id 추적
- 일관성: OutboxEvent 패턴 작동 (이전엔 trait 정의만)
- 안전성: tx atomic — audit 실패 = 전체 실패

**SP5-iii 미포함 (후속)**:
- SP5-iv: User/Listing/ListingPhoto save() 에 MutationContext 추가 (SP5-i 의 3 repo)
- SP4: 외부 API ingestion + R2 Reader + Outbox publisher worker
- SP5-ii: Insights BC RDS (Bookmark/SearchHistory/AnalysisReport/Notification)
- AuditLog full diff capture (before_state + after_state) — 별도
```

- [ ] **Step 4: Commit + push + 최종 CI 확인**

```bash
git add MEMORY.md memory/project_progress.md
git commit -m "chore(sp5-iii-t11): integration validation — Sub-project 5-iii complete (25 crates, ~1110 tests)

3 CI workflow 그린:
- CI 7 jobs (clippy / fmt / cargo-deny / tarpaulin ≥90% / secret / file-size / markdown)
- db-migrations: V001-V003_05
- walking-skeleton: integration tests ~53 (SP5-i 23 + SP5-iii 30) + E2E 6/6 + DB reset

SP5-iii 산출물:
- MutationContext + 6 도메인 trait 시그니처 변경
- 8 신규 PgRepository (트랜잭션 audit/outbox 패턴)
- AuditLog V002 immutable trigger 검증

다음: SP5-iv (SP5-i refactor) / SP5-ii (Insights) / SP4 (외부 API) — 사용자 결정"
git push
gh run list --branch main --limit 3
```

3 워크플로우 그린 최종 확인.

---

## 검증 기준 매핑 (Spec § 10)

| Spec § 10 항목 | 본 plan task |
|---|---|
| 1. `MutationContext` 신규 + 5-7 unit tests | T1 (6 tests) |
| 2. 8 도메인 `RepoError` 동일 3 variants 패턴 | T2 (`MapFromSqlx` impls) |
| 3. 6 도메인 `Repository` trait save signature 변경 | T1 |
| 4. 8 신규 PgImpl | T3-T10 |
| 5. 모든 신규 repo 메서드 `#[tracing::instrument]` | T3-T10 매 task |
| 6. 통합 테스트 ~30 신규 | T3-T10 합산 |
| 7. `error_map.rs` 8 도메인 impl | T2 |
| 8. 3 CI 워크플로우 그린 | T11 |
| 9. 누적 ≥1110 | T11 |
| 10. tarpaulin ≥90% | T1-T10 매 commit |
| 11. clippy `-D warnings` | T1-T10 매 commit |
| 12. 파일 ≤500 권장 / ≤1500 강제 | T1-T10 매 commit |

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-13 모든 절 반영
- [x] 11 task 모두 fresh subagent dispatch 가능 단위
- [x] schema 정정 (audit_log 실제 컬럼: before_state/after_state/ip_address/created_at)
- [x] 도메인 시그니처 검증 명시 (`grep -A` 명령어 포함)
- [x] tx 패턴 일관성 (8 repo 모두 `pool.begin → INSERT/UPDATE → audit_log INSERT → outbox INSERT for each → commit`)

## 알려진 위험

1. **도메인 entity 시그니처 가정** — `AuditLog::try_new` 11-arg, `OutboxEvent::try_new` 8-arg, AdminAction/BVQ/LRQ/ListingReport/FeaturedContent/SystemAlert 모두 가정. 첫 cargo check 컴파일 에러 시 수정.
2. **before_state/after_state 매핑** — 본 plan 은 `before_state = NULL` (full diff 캡처는 후속). `after_state = ctx.metadata`. 이 매핑이 application 사용 시 설계 의도와 맞는지 검증 필요.
3. **8 신규 repo 통합 테스트 시간** — walking-skeleton CI 워크플로우 시간 ~5-7분 으로 늘 수 있음. `--test-threads=1` 직렬 실행 + 30 통합 테스트 = 추가 ~1분.
4. **AuditLog `ip_address` `inet` 타입** — sqlx Postgres `inet` 매핑은 String 으로 가능하지만 검증 필요. 실패 시 `host(ip_address)` 캐스팅으로 우회.
5. **AuditLog/Outbox 자체 RepoError 의 Conflict** — Conflict variant 가 있는지 확인. 없으면 spec 처럼 Database 로 fallback.

## 완료 후 다음

**Sub-project 5-iii 종료** → 사용자 결정:
- **SP5-iv**: User/Listing/ListingPhoto save() 에 `MutationContext` 추가 (SSS 약속 완전 닫음)
- **SP5-ii**: Insights BC RDS (4 repo)
- **SP4**: 외부 API ingestion + R2 Reader + Outbox publisher worker
