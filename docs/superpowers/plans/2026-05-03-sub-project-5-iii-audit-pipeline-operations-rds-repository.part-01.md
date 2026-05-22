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

