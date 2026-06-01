# Sub-project 5-iii: Audit + Pipeline + Operations BC RDS Repository SQLx + 트랜잭션 Outbox 패턴 (Spec)

| | |
|---|---|
| 작성일 | 2026-05-03 |
| 상태 | Approved |
| 선행 | SP1, SP2 (a/b-i/b-ii/c), SP3 (Auth), SP5-i (Core BC RDS Repository) |
| 후속 | SP5-iv (SP5-i refactor — User/Listing/ListingPhoto 도 `MutationContext` 받기), SP4 (외부 API + R2 Readers), SP5-ii (Insights BC RDS) |
| 관련 ADR | [ADR-0008](../../adr/0008-spatial-postgis.md) (PostGIS), [ADR-0011](../../adr/0011-embedding-gemini-pgvector.md) |

---

## 1. 개요

Audit BC (`AuditLog`, `OutboxEvent`), Pipeline BC (`PipelineSchedule`, `PipelineRun`), Operations BC (5 Aggregate) 의 `Postgres` 저장소 8 개 구현 + **트랜잭션 audit/outbox 패턴 도입**.

본 sub-project 는 SSS 7 기둥 중 **추적성·일관성·안전성** 의 핵심 결함을 닫는 게 목표예요:

- **추적성**: 모든 mutation 이 `audit_log` INSERT 와 *같은 트랜잭션*에 기록
- **일관성**: `OutboxEvent` 패턴이 실제로 작동 (Aggregate save + outbox INSERT 같은 tx)
- **안전성**: tx 중간 실패 시 audit/outbox 자동 rollback — 부분 상태 0

`MutationContext` 라는 새 타입을 도입해 호출자가 actor / correlation / events 를 명시적으로 전달. PgRepository 가 tx 안에서 자동으로 모든 INSERT 처리.

---

## 2. 범위 (Scope)

### 포함
- **8 신규 PgRepository**:
  - `PgAuditLogRepository` — insert-only, `find_by_resource` / `find_by_actor` / `find_by_correlation_id`
  - `PgOutboxRepository` — `save` / `fetch_unpublished` / `mark_published`
  - `PgPipelineRepository` — 2 aggregates (PipelineSchedule + PipelineRun)
  - `PgAdminActionRepository` — insert-only
  - `PgBvqRepository` — OCC
  - `PgLrqRepository` — OCC
  - `PgListingReportRepository`
  - `PgOperationsMetaRepository` — 2 aggregates (FeaturedContent + SystemAlert)
- **`MutationContext` 신규** — `crates/domain/core/shared-kernel/src/mutation.rs`
- **트랜잭션 save 패턴** — 모든 8 repo 적용 (`AuditLog` / `Outbox` 자체 제외)
- **Repository trait 시그니처 변경** — 8 repo 의 `save` (또는 `insert`) 가 `ctx: MutationContext` 인자 추가
- **CI 통합 테스트 ~30** — tx atomicity, rollback on failure, no-actor system actions
- **AuditLog/Outbox 자체** — single-row insert (transactional 패턴 대상 아님 — recursion 방지)
- **`Cargo.toml` 의존성 추가** — `audit-log-domain`, `outbox-event-domain`, `data-pipeline-control`, `admin-action-domain`, `business-verification-queue-domain`, `lrq-domain`, `listing-report-domain`, `operations-meta-domain` (모두 `crates/db` deps)
- **`MutationContext` 단위 테스트** — helper constructors

### 미포함 (후속 sub-project)
- SP5-iv: SP5-i 의 PgUserRepository/PgListingRepository/PgListingPhotoRepository `save` 시그니처에 `MutationContext` 추가
- SP4: 외부 API ingestion + R2 Reader 6 (Parcel/Building/IC/Mfr/RealTransaction/CourtAuction)
- SP5-ii: Insights BC RDS Repository (Bookmark/SearchHistory/AnalysisReport/Notification)
- Outbox **publisher worker** (실제로 outbox row 를 외부 시스템에 발행) — 별도 sub-project (SP4 와 묶거나 후속)
- HTTP 응답 매핑 (`RepoError → IntoResponse`) — 별도
- `sqlx::query!()` macro 채택 — 별도 ADR
- AuditLog `client_ip` / `user_agent` 자동 수집 (Axum middleware 통합) — 별도 sub-project (관측성)

---

## 3. 아키텍처

```
┌─────────────────────────────────────────────────────┐
│  Application layer (handler / service / middleware) │
│  → MutationContext::new_user_action(actor, corr_id, │
│       "approve").with_events(vec![BvqApproved{...}])│
└──────────┬──────────────────────────────────────────┘
           │ Repository.save(aggregate, ctx)
           ▼
┌─────────────────────────────────────────────────────┐
│  PgBvqRepository (예시)                             │
│  ┌───────────────────────────────────────────────┐  │
│  │ tx = pool.begin()                             │  │
│  │ ┌─────────────────────────────────────────┐   │  │
│  │ │ 1. UPDATE bvq SET ... WHERE version = $X│   │  │
│  │ │ 2. INSERT audit_log (actor, action,     │   │  │
│  │ │      resource_kind='bvq', resource_id,  │   │  │
│  │ │      after_state, correlation_id,       │   │  │
│  │ │      ip_address, user_agent, created_at)│   │  │
│  │ │ 3. INSERT outbox_event for each event   │   │  │
│  │ └─────────────────────────────────────────┘   │  │
│  │ tx.commit()  // 실패 시 모두 rollback         │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
           │
           ▼
       Postgres
       (audit_log immutable trigger 가 UPDATE/DELETE 차단)
```

`AuditLog` / `Outbox` 자체 repo 는 위 패턴 **사용 안 함** — recursion 방지.

---

## 4. 컴포넌트 정의

### 4.1 `crates/domain/core/shared-kernel/src/mutation.rs` (신규)

```rust
//! `MutationContext` — 모든 audit/outbox transactional save 의 입력.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::domain_event::DomainEvent;
use crate::id::{Id, UserMarker};

/// 모든 mutation 의 audit/outbox 컨텍스트.
///
/// 호출자 (application layer) 가 누가/왜/무엇을 명시. PgRepository 가 tx 안에서
/// `audit_log` / `outbox_event` INSERT 를 자동 수행.
#[derive(Debug, Clone)]
pub struct MutationContext {
    /// 누가. None = system action (pipeline scheduler, cron job 등).
    pub actor_id: Option<Id<UserMarker>>,
    /// HTTP 요청 ID 또는 pipeline run ID (구조적 로그 / Tempo 연결).
    pub correlation_id: String,
    /// 도메인 의미 (예: "create", "update", "approve", "reject", "acknowledge").
    /// "save" 같은 무의미 값 금지 — 도메인 의도 보존.
    pub action: String,
    /// 추가 메타데이터 (`audit_log.after_state` JSONB 컬럼으로 저장).
    pub metadata: Option<Value>,
    /// 본 mutation 이 발행하는 도메인 이벤트들 (Outbox 로 전파).
    pub events: Vec<Arc<dyn DomainEvent>>,
    /// 클라이언트 IP (HTTP 요청 시).
    pub client_ip: Option<String>,
    /// 클라이언트 User-Agent.
    pub user_agent: Option<String>,
    /// mutation 발생 시각. None 이면 PgRepository 가 `Utc::now()` 사용.
    pub occurred_at: Option<DateTime<Utc>>,
}

impl MutationContext {
    /// 인증된 사용자가 일으킨 mutation. action 은 도메인 의미 String.
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

    /// builder pattern.
    #[must_use]
    pub fn with_metadata(mut self, m: Value) -> Self {
        self.metadata = Some(m);
        self
    }

    #[must_use]
    pub fn with_events(mut self, events: Vec<Arc<dyn DomainEvent>>) -> Self {
        self.events = events;
        self
    }

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

    #[must_use]
    pub const fn with_occurred_at(mut self, at: DateTime<Utc>) -> Self {
        self.occurred_at = Some(at);
        self
    }
}
```

> `Arc<dyn DomainEvent>` 사용 — `Box<dyn ...>` 대신 clone 가능 (테스트 편의).
>
> shared-kernel 의존: `serde_json` (workspace dep, `crates/domain/audit/outbox-event` 가 이미 사용).

### 4.2 Repository trait 시그니처 변경

8 신규 repo 의 trait `save` (또는 `insert`) 가 `MutationContext` 인자 추가. *기존 trait 정의를 변경*하므로 도메인 crate 가 함께 변경됨 (선행 commit 필수).

#### `audit-log-domain` (`crates/domain/audit/audit-log/src/repository.rs`)
```rust
pub trait AuditLogRepository: Send + Sync {
    /// `INSERT` only — `audit_log` 는 자체 audit 안 함 (`V002` immutable trigger 와 동일 정책).
    async fn insert(&self, log: &AuditLog) -> Result<(), RepoError>;

    /// 기존 find 메서드들 — 변경 없음
    async fn find_by_resource(...);
    async fn find_by_actor(...);
    async fn find_by_correlation_id(...);
}
```
**변경 없음** — AuditLog 자체는 transactional 패턴 대상 아님.

#### `outbox-event-domain`
```rust
pub trait OutboxRepository: Send + Sync {
    /// 단순 INSERT — outbox 자체는 audit 안 함.
    async fn save(&self, event: &OutboxEvent) -> Result<(), RepoError>;
    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<OutboxEvent>, RepoError>;
    async fn mark_published(...) -> Result<(), RepoError>;
}
```
**변경 없음**.

#### Pipeline / AdminAction / BVQ / LRQ / ListingReport / OperationsMeta — 모두 `MutationContext` 추가
```rust
// 예: BvqRepository
pub trait BvqRepository: Send + Sync {
    async fn save(&self, bvq: &Bvq, ctx: MutationContext) -> Result<(), RepoError>;
    async fn find_by_id(&self, id: &Id<BvqMarker>) -> Result<Option<Bvq>, RepoError>;
    // ... 그 외 finds (변경 없음)
}
```

`save` / `insert` 메서드만 `ctx` 추가. find/fetch 메서드는 변경 없음.

`AdminAction` 의 경우 `insert(&self, action: &AdminAction, ctx: MutationContext)` — admin action 도 audit 됨 (admin 의 admin action 이라는 메타-audit).

`PipelineRepository` 의 메서드들 (예: `save_schedule`, `save_run`) 모두 ctx 추가.

`OperationsMetaRepository` 의 `save_featured` / `save_alert` 모두 ctx 추가.

### 4.3 PgRepository 구현 패턴

```rust
// crates/db/src/bvq.rs (예시)

pub struct PgBvqRepository {
    pool: PgPool,
}

#[async_trait]
impl BvqRepository for PgBvqRepository {
    #[instrument(skip(self, bvq, ctx), fields(bvq_id = %bvq.id.as_str(), action = %ctx.action))]
    async fn save(&self, bvq: &Bvq, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. Aggregate UPSERT with OCC
        let result = sqlx::query(
            r#"
            INSERT INTO business_verification_queue (...) VALUES (...)
            ON CONFLICT (id) DO UPDATE SET ...,
                version = business_verification_queue.version + 1
            WHERE business_verification_queue.version = $N
            "#,
        )
        .bind(...)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }

        // 2. AuditLog INSERT — 같은 tx
        // 실제 schema (`migrations/10003_system_tables.sql`) 매핑:
        //   ctx.metadata    → after_state (JSONB)
        //   ctx.client_ip   → ip_address  (inet, $N::inet 캐스팅)
        //   ctx.occurred_at → created_at  (timestamptz; None → DB default now())
        let created_at = ctx.occurred_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r#"
            INSERT INTO audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state,
                correlation_id, ip_address, user_agent, created_at
            )
            VALUES ($1, $2, $3, 'bvq', $4, NULL, $5, $6, $7::inet, $8, $9)
            "#,
        )
        .bind(/* new audit log id */)
        .bind(ctx.actor_id.as_ref().map(|id| id.as_str()))
        .bind(&ctx.action)
        .bind(bvq.id.as_str())
        .bind(&ctx.metadata)        // → after_state
        .bind(&ctx.correlation_id)
        .bind(&ctx.client_ip)       // → ip_address (inet)
        .bind(&ctx.user_agent)
        .bind(created_at)           // → created_at
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. Outbox INSERT for each event — 같은 tx
        for event in &ctx.events {
            sqlx::query(
                r#"
                INSERT INTO outbox_event (
                    id, event_type, aggregate_kind, aggregate_id,
                    payload, occurred_at, correlation_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(/* new outbox id */)
            .bind(event.event_type())
            .bind("bvq")
            .bind(event.aggregate_id())
            .bind(event.payload())
            .bind(event.occurred_at())
            .bind(&ctx.correlation_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    // find_by_id / find_pending / etc — 변경 없음 (read-only)
}
```

이 패턴이 8 repo 에 반복.

### 4.4 OperationsMeta 처리 — 단일 trait 유지

`OperationsMetaRepository` 는 `save_featured` + `save_alert` 두 메서드 가짐. 둘 다 `ctx` 추가:
```rust
async fn save_featured(&self, fc: &FeaturedContent, ctx: MutationContext) -> Result<...>;
async fn save_alert(&self, alert: &SystemAlert, ctx: MutationContext) -> Result<...>;
```

각 메서드가 자체 tx 시작 + audit_log INSERT (resource_kind = "featured_content" 또는 "system_alert").

### 4.5 PipelineRepository 처리 — 단일 trait 유지

`save_schedule(s, ctx)` + `save_run(r, ctx)`. PipelineRun.save() 시 actor_id = None (시스템), action = "create"/"update", correlation_id = run.id.

---
