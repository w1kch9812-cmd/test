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

## 5. 데이터 흐름 (시퀀스)

### 5.1 사용자 요청 — BVQ 승인

```
[1] HTTP POST /admin/bvq/:id/approve
[2] Auth middleware → AuthenticatedUser { actor_id, ... }
[3] Handler:
    a. fetch BVQ from PgBvqRepository.find_by_id(...)
    b. domain method bvq.approve(reviewer_id, note, now)
    c. ctx = MutationContext::new_user_action(
            actor_id, request_id, "approve")
        .with_events(vec![Arc::new(BvqApprovedEvent {...})])
        .with_client_info(req.ip, req.ua)
    d. repo.save(&bvq, ctx).await
[4] PgBvqRepository.save (tx 안에서):
    a. UPDATE bvq SET status='approved', reviewed_at=now, version=version+1
       WHERE id=$ AND version=$
    b. INSERT audit_log (actor=admin, action='approve', resource_kind='bvq',
       resource_id=bvq.id, correlation_id=request_id)
    c. INSERT outbox_event (event_type='bvq.approved', aggregate_id=bvq.id,
       payload=event.payload(), correlation_id=request_id)
    d. tx.commit()
[5] (별도 워커) outbox publisher 가 unpublished 가져와 외부 발행 (SP4 후속)
```

### 5.2 시스템 — Pipeline run 시작

```
[1] Pipeline scheduler tick (cron 또는 manual trigger)
[2] PipelineRun::start(...) 도메인 메서드 → 새 run 생성
[3] ctx = MutationContext::new_system_action(run.id.as_str(), "create")
[4] PgPipelineRepository.save_run(&run, ctx).await
[5] tx 안:
    a. INSERT pipeline_run
    b. INSERT audit_log (actor=NULL, action='create', resource_kind='pipeline_run')
    c. (events 없음)
    d. commit
```

---

## 6. 에러 매핑 정책

### 6.1 도메인 RepoError 일관성

8 repo 의 `RepoError` 모두 3 variants 패턴 따름:
- `NotFound`
- `Conflict` (OCC 또는 unique violation)
- `Database(String)`

`AuditLogRepository` `RepoError` 는 `Conflict` 없음 (insert-only, OCC 없음, unique violation 도 없음 — id 자동 생성 ULID). SP5-i T1 의 `MapFromSqlx` impl 에 audit-log-domain 추가:
```rust
impl MapFromSqlx for audit_log_domain::repository::RepoError {
    fn conflict() -> Self {
        Self::Database("unexpected conflict in audit_log".into())
    }
    fn database(msg: String) -> Self {
        Self::Database(msg)
    }
}
```

같은 식으로 `OutboxRepository` `RepoError` — `Conflict` 정의 (추가 필요 시) 또는 fallback.

### 6.2 트랜잭션 실패

- `tx.commit()` 실패 → `RepoError::Database(msg)` 반환
- 중간 INSERT 실패 → `?` 로 자동 rollback (sqlx Drop 시 rollback)
- audit_log INSERT 실패 → 전체 mutation 실패 (commit 안 됨, aggregate 업데이트도 안 됨)

이 동작이 SSS 안전성 핵심: *audit 가 안 들어가면 mutation 도 안 들어감*.

### 6.3 SQL injection 방어

모든 사용자 입력 `bind()` 통해 parameterized query. `MutationContext.action` / `metadata` / `correlation_id` 등 모두 bind. `format!` 안에 사용자 입력 금지.

---

## 7. 가시성 — `tracing::instrument`

모든 repo 메서드에 `#[instrument(skip(self, ctx, aggregate), fields(<ids>, action = %ctx.action))]`:

```rust
#[instrument(skip(self, bvq, ctx), fields(bvq_id = %bvq.id.as_str(), action = %ctx.action, correlation_id = %ctx.correlation_id))]
async fn save(&self, bvq: &Bvq, ctx: MutationContext) -> Result<(), RepoError>;
```

PII 미노출:
- `actor_id` 는 ID 만 (이름/이메일 노출 X)
- `metadata` / `client_ip` / `user_agent` 는 `skip` (운영 시 별도 dashboard 에서 audit_log 직접 쿼리)
- `events` 의 payload 도 `skip`

---

## 8. 통합 테스트 전략

### 8.1 새 repo 별 4-5 tests

**예: `bvq_integration.rs` (5 tests)**:
1. `save_inserts_aggregate_audit_outbox_in_one_tx` — 성공 후 3 row 모두 존재 확인
2. `save_failure_rolls_back_audit_log` — OCC mismatch 일으키고 audit_log 도 안 들어감 확인
3. `save_user_action_records_actor_id` — actor_id 정확히 audit_log 에 기록
4. `save_system_action_with_no_actor` — actor_id NULL 허용
5. `save_with_multiple_events_inserts_each_outbox` — events 2개 → outbox 2 row

**`audit_log_integration.rs` (3-4 tests)**:
1. `insert_persists_all_fields`
2. `find_by_resource_returns_correct_logs`
3. `find_by_actor_filters_correctly`
4. `immutable_trigger_blocks_update` — V002 trigger 가 UPDATE/DELETE rejection 확인

**`outbox_integration.rs` (3-4 tests)**:
1. `save_persists_event`
2. `fetch_unpublished_returns_only_unpublished`
3. `mark_published_updates_published_at`

총 ~30 통합 테스트 (8 repo × 3-5 tests + audit/outbox 각 3-4).

### 8.2 단위 테스트

`MutationContext` helpers — `new_user_action` / `new_system_action` / builders:
- 5-7 unit tests in `mutation.rs`

---

## 9. CI 통합

기존 `walking-skeleton.yml` 의 `cargo test --features integration` 단계가 새 통합 테스트도 자동 실행. 별도 CI 변경 없음.

`db-migrations.yml` 변경 없음 (스키마 변화 없음 — 8 repo 는 이미 V001 에 정의된 테이블 사용).

---

## 10. 검증 기준 (DoD)

본 sub-project 종료 조건:

1. `crates/domain/core/shared-kernel/src/mutation.rs` 신규 (`MutationContext` + 5-7 unit tests)
2. 8 도메인 crate 의 `RepoError` 가 모두 동일 3 variants (`NotFound` / `Conflict` / `Database`) 패턴
3. 6 도메인 crate (pipeline / admin-action / bvq / lrq / listing-report / operations-meta) 의 `Repository` trait 의 `save` (또는 `insert`) 메서드 시그니처가 `ctx: MutationContext` 추가
4. 8 신규 PgImpl in `crates/db/src/{audit_log,outbox,pipeline,admin_action,bvq,lrq,listing_report,operations_meta}.rs`
5. 모든 신규 repo 메서드 `#[tracing::instrument]` (PII 미노출)
6. 통합 테스트 ~30 신규 (`crates/db/tests/*_integration.rs`)
7. `error_map.rs` 에 6 신규 도메인 `RepoError` 의 `MapFromSqlx` impl
8. 3 CI 워크플로우 (CI / db-migrations / walking-skeleton) 모두 그린
9. 누적 단위 + 통합 테스트 ≥1110 (1075 + 30 통합 + 5-7 단위)
10. tarpaulin ≥90% 유지
11. clippy `-D warnings` 통과
12. 모든 파일 ≤500 권장 / ≤1500 강제

---

## 11. SSS 7 기둥 매핑 (정직 평가)

| 기둥 | SP5-iii 적용 |
|---|---|
| 1 일관성 | 8 repo 모두 동일 transactional save 패턴. `MutationContext` 단일 인터페이스. SP5-i 의 3 repo 만 미정렬 (SP5-iv 가 닫음) |
| 2 자동 강제 | tx 안에서 audit_log + outbox INSERT — 못 잊음. 통합 테스트 CI 게이트로 *그 동작* 검증. integration test 빨강 = walking-skeleton 빨강 |
| 3 추적성 | **모든 mutation** (8 repo 의 save) 이 audit_log 자동 INSERT. correlation_id 추적 + actor_id 기록. `find_by_correlation_id` 로 1 request 의 모든 mutation 추적 가능 |
| 4 안전성 | tx atomic — audit 실패 = 전체 실패. parameterized SQL only. `RepoError` sealed, no panics, no unsafe. immutable trigger (V002) 가 audit_log UPDATE/DELETE 차단 |
| 5 가시성 | 모든 메서드 `tracing::instrument`. PII (actor 이름/email/IP/UA/metadata) 미노출 — skip 활용 |
| 6 SSOT | DB schema = SSOT. 8 repo 가 그걸 따름. AuditLog 의 `resource_kind` 문자열 ("bvq", "pipeline_run" 등) 은 *코드의 PgRepository 마다* hard-coded — 도메인 의미 보존 |
| 7 명확성 | `MutationContext.action` 도메인 의미 보존 ("approve" / "create" / "acknowledge"). "save" 같은 무의미 값 spec 에 명시 금지. error variants 한국어 해요체 (도메인 측에서 정의) |

---

## 12. Follow-up items (production 배포 전)

1. **SP5-iv** — User/Listing/ListingPhoto save() 가 `MutationContext` 받기. AuthMiddleware 의 first-sign-in 자동 생성 ctx 구성.
2. **Outbox publisher worker** — `outbox_event` row 를 fetch_unpublished 후 외부 시스템에 발행 + `mark_published`. SP4 또는 별도 sub-project.
3. **Axum middleware 통합** — HTTP 요청에서 `correlation_id` (X-Request-ID 헤더) + `client_ip` + `user_agent` 자동 추출 → handler 가 받는 `MutationContext` 에 자동 주입. 관측성 sub-project 와 묶음.
4. **AuditLog metadata 표준화** — 어떤 도메인 변경이 어떤 metadata 형식으로 들어가는지 *컨벤션* 문서. 현재는 application layer 자유.
5. **resource_kind 상수화** — 현재 PgRepository 마다 `"bvq"` 등 hard-coded. enum 으로 빼서 type-safe 화 가능. 별도 작업.

---

## 13. 후속 sub-project 시드

- **SP5-iv**: SP5-i refactor (User/Listing/ListingPhoto save → MutationContext)
- **SP5-ii**: Insights BC RDS Repository (Bookmark/SearchHistory/AnalysisReport/Notification)
- **SP4**: 외부 API ingestion + R2 Reader 6
- **SP4 ext**: Outbox publisher worker

---

## 14. FU 13 closed by SP-FU-i (2026-05-04)

본 spec § 4.3 의 `audit_log` INSERT mock 컬럼이 실제 schema (`migrations/10003_system_tables.sql`)
와 일치하도록 정정. plan 단계에서 발견된 schema mismatch 회복:

- `metadata`     → `after_state` (JSONB; `before_state` 는 update 시 사용, 본 mock 은 `NULL`)
- `client_ip`    → `ip_address` (`inet` 타입; bind 시 `$N::inet` 캐스팅)
- `occurred_at`  → `created_at` (`timestamptz`; `None` 이면 DB default `now()`)

`MutationContext` Rust 필드명 (`metadata`/`client_ip`/`occurred_at`) 은 application 측 의미 보존을
위해 그대로 유지 — SQL 레이어에서만 매핑.
