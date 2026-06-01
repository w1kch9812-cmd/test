# Sub-project 2c - Part B: Outbox, Pipeline, Operations, Validation, And Handoff

Parent index: [Sub-project 2c Market Insights Operations](./2026-05-02-sub-project-2c-market-insights-operations.md).

## Phase D: OutboxEvent + Outbox Repository

### Task 9: OutboxEvent Aggregate

**Spec § 5.3** lines 347-365.

```rust
pub struct OutboxEvent {
    pub id: Id<OutboxEventMarker>,                    // evt_<26 ULID> (per spec § 5.3)
    pub event_type: String,                           // 'listing.approved' (≤50자)
    pub aggregate_kind: String,                       // 'listing' (≤30자)
    pub aggregate_id: String,                         // ≤50자
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub correlation_id: String,
}

impl OutboxEvent {
    /// `DomainEvent`로부터 `OutboxEvent` 생성.
    pub fn from_domain<E: DomainEvent>(event: &E, aggregate_kind: &str, correlation_id: String) -> Self;
    pub fn mark_published(&mut self, at: DateTime<Utc>);
}
```

`OutboxEventMarker` (PREFIX `"evt"` per spec § 5.3 inline; earlier plan draft mistakenly used `"oev"`).

### Task 10: OutboxRepository trait

```rust
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// 단일 INSERT (트랜잭션 내 Aggregate save와 함께).
    async fn save(&self, event: &OutboxEvent) -> Result<(), RepoError>;
    /// 미배포 이벤트 폴링 (publisher 워커가 사용).
    async fn fetch_unpublished(&self, limit: usize) -> Result<Vec<OutboxEvent>, RepoError>;
    /// 배포 완료 마킹.
    async fn mark_published(&self, id: &Id<OutboxEventMarker>, at: DateTime<Utc>) -> Result<(), RepoError>;
}
```

- [ ] T9 + T10 합쳐 1 commit (또는 분리). ≥12 tests + CI green

---

## Phase E: Pipeline Aggregates

### Task 11: PipelineSchedule Aggregate

**Spec § 5.4** lines 372-393.

13 필드 (id, pipeline_kind UNIQUE, cron_expression, enabled default true, timezone Asia/Seoul, last_run_at, next_run_at, config jsonb, running_lock_acquired_at, running_worker_id, updated_at, updated_by, version).

`PipelineScheduleMarker` (PREFIX `"pls"`).

**도메인 메서드:**
- `try_new(...)` — invariant: pipeline_kind 비어있지 않음, cron_expression 비어있지 않음 (cron 문법 검증은 sub-project 4에서)
- `enable(&mut self, by: Id<UserMarker>, at)` — disabled → enabled
- `disable(&mut self, by, at)`
- `acquire_lock(&mut self, worker_id: String, at)` — running_lock_acquired_at 설정
- `release_lock(&mut self, at)`
- `record_run(&mut self, run_started_at)` — last_run_at 갱신
- `update_config(&mut self, config: serde_json::Value, by, at)` — version bump

### Task 12: PipelineRun Aggregate

**Spec § 5.4** lines 397-431.

`steps jsonb` — 단계별 진행 + 결과. spec 인용 그대로 모델링.

`PipelineRunMarker` (PREFIX `"plr"`).

**도메인 메서드:**
- `try_new_started(schedule_id, triggered_by, at)` — initial state running, steps=[]
- `add_step(&mut self, step_name, at)` — steps[].push (in-progress)
- `complete_step(&mut self, step_name, items_processed, items_changed, output_hash, at)`
- `fail_step(&mut self, step_name, error, at)`
- `complete_run(&mut self, at)` — status='success', finished_at, *immutable after*
- `fail_run(&mut self, error_message, at)`
- `abort_run(&mut self, at)`

상태 전이는 enum (running/success/failed/skipped_unchanged/aborted).

**PipelineRepository:** 두 Aggregate 합친 1 trait (find_schedule_by_kind, save_schedule, save_run, find_recent_runs, etc.)

- [ ] T11 + T12 + Repository + Markers (2개) + ≥18 tests + CI green

---

## Phase F: Operations BC 6 Aggregate

각 task = 1 Aggregate + Repository trait. spec § 5.5 verbatim.

### Task 13: AdminAction Aggregate

Spec § 5.5 lines 449-462. AdminActionMarker (PREFIX "ada").

**도메인 메서드:** try_new (insert-only, like AuditLog — admin 액션은 immutable).

**Repository:** insert-only.

### Task 14: BusinessVerificationQueue Aggregate

Spec § 5.5 lines 465-485 + V003_02 version. BVQMarker (PREFIX "bvq").

**도메인 메서드:**
- try_new_pending
- approve(&mut self, reviewed_by, at) — status pending → approved
- reject(&mut self, reviewed_by, reason, at)
- request_more_info(&mut self, reviewed_by, message, at) — pending → needs_more_info → user supplies → pending again
- 모두 version bump

### Task 15: ListingReviewQueue Aggregate

Spec § 5.5 lines 488-508 + V003_02 version. LRQMarker (PREFIX "lrq").

**도메인 메서드:**
- try_new_pending
- approve, reject, request_changes (3-state decision)
- 모두 version bump

### Task 16: ListingReport Aggregate

Spec § 5.5 lines 511-528. ListingReportMarker (PREFIX "lrp").

**도메인 메서드:**
- try_new (anonymous reports → reporter_id None)
- mark_investigating, mark_confirmed, mark_dismissed (status workflow)

### Task 17: FeaturedContent + SystemAlert (1 task, 2 Aggregate)

**FeaturedContent** (spec § 5.5 lines 531-549 + V003_03 time bound):
- FeaturedContentMarker (PREFIX "fea")
- try_new with V003_03 invariant (ends_at > starts_at)
- pub fn is_active_at(&self, t: DateTime<Utc>) -> bool

**SystemAlert** (spec § 5.5 lines 552-568):
- SystemAlertMarker (PREFIX "sal")
- try_new (info/warning/error/critical)
- acknowledge(&mut self, by, at)

**Repository (둘 묶음):**

```rust
#[async_trait]
pub trait OperationsRepository: Send + Sync {
    async fn save_featured(&self, fc: &FeaturedContent) -> Result<(), RepoError>;
    async fn find_active_featured(&self, at: DateTime<Utc>) -> Result<Vec<FeaturedContent>, RepoError>;
    async fn save_alert(&self, alert: &SystemAlert) -> Result<(), RepoError>;
    async fn find_unacknowledged_alerts(&self) -> Result<Vec<SystemAlert>, RepoError>;
    // ... 다른 4 Aggregate (admin_action, BVQ, LRQ, listing_report)는 별도 trait 또는 통합
}
```

**대안:** 6 Aggregate × 6 Repository — 너무 많음. *2 Repository로 묶기*: AdminWorkflowRepository (BVQ + LRQ + ListingReport) + AdminMetaRepository (AdminAction + FeaturedContent + SystemAlert). 그룹화는 implementer 재량.

- [ ] T13-T17: 5 task (T17은 묶음). ≥40 tests + CI green per task

---

## Phase G: 통합 검증

### Task 18: 통합 검증 + project_progress 갱신

검증:
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo deny check`
- `cargo tarpaulin --workspace --skip-clean --out Lcov --fail-under 90`

기대 결과:
- 누적 테스트: 466 (2b-ii 종료) + ~200 (2c) ≈ 660+
- crate count: 10 → ~22 (Phase A 2 + B 4 + C 2 + D 1 + E 1 + F 5 ≈ 15 신규)

**MEMORY.md 갱신** + **memory/project_progress.md 갱신** — Sub-project 2 완료 표기.

- [ ] cargo workspace 검증
- [ ] 테스트 카운트 확인
- [ ] memory 파일 갱신
- [ ] Commit + push + CI green

```bash
git commit -m "chore(2c): integration validation — Sub-project 2 complete (~22 crates, ~660 tests)"
```

---

## Self-Review Checklist (plan 작성자 — 끝났음)

- [x] Sub-project 2 잔여 도메인 모두 다룸 (Market 2 + Insights 4 + System 2 + Pipeline 2 + Operations 6 = 16 Aggregate/Reader)
- [x] AuditLog insert-only invariant (V002 트리거 호환)
- [x] DomainEvent trait + Outbox Repository로 이벤트 패턴 정의
- [x] V003_01/02/03 모든 cross-field invariant Aggregate 레벨에서 강제
- [x] spec § 5.2-5.5 직접 인용
- [x] 알려진 lessons (`#[path]` import, doc_markdown, derive_partial_eq_without_eq) 사전 대응

## 알려진 위험

1. **17 task** — 가장 큰 plan. 2-3 day 작업 추정. 각 task 1-3 CI iter (lessons 적용 시 단축).
2. **DomainEvent trait** — Box<dyn> overhead. 대안: BC별 enum wrapper. 최종 결정은 sub-project 4 publisher 통합 시 — 본 plan은 trait 정의만.
3. **OutboxRepository transactional 보장** — Aggregate save와 outbox INSERT는 *같은 트랜잭션*이어야. SQLx 구현은 sub-project 5 — 본 plan은 trait 인터페이스만. 구현체가 transactional 보장 책임.
4. **Operations Repository 그룹화** — 6 Aggregate × 6 Repository는 과한 폴더. 2-3 group으로 묶기 (implementer 재량). 인터페이스 정의 단계라 변경 비용 낮음.

## 완료 후 다음

**Sub-project 2 종료** → **Sub-project 3 (Auth)** 진입:
- Zitadel JWT 검증 미들웨어 (Axum tower)
- User Aggregate와 연결 (`User::find_by_zitadel_sub`)
- 인증된 사용자 → handler에서 `Extension<AuthenticatedUser>` 추출

또는 **Sub-project 4 (외부 API 통합)** — Reader trait 구현체 (V-World, 법제처 등).

순서는 사용자 결정.
