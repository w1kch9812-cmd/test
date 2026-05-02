# Sub-project 2c: Market BC + Insights BC + Operations BC + Pipeline + Audit + Outbox — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.
>
> **CRITICAL:** Read [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) before each task.

**Goal:** Sub-project 2의 *마지막* plan. Market BC (Real Transaction + Court Auction Reader) + Insights BC (Bookmark/SearchHistory/AnalysisReport/Notification Aggregate) + Operations BC (6 Aggregate) + Pipeline + AuditLog + Outbox 도메인 이벤트 패턴 정의. 모두 *port-only* (구현은 sub-project 4-5).

**Architecture:** 6 phase, ~17 task.
- **Phase A (T1-T2):** Market BC R2 Readers (RealTransaction + CourtAuction)
- **Phase B (T3-T6):** Insights BC RDS Aggregates (Bookmark / SearchHistory / AnalysisReport / Notification)
- **Phase C (T7-T8):** AuditLog + DomainEvent trait
- **Phase D (T9-T10):** OutboxEvent + Outbox Repository
- **Phase E (T11-T12):** Pipeline (PipelineSchedule + PipelineRun)
- **Phase F (T13-T17):** Operations BC 6개 Aggregate
- **Phase G (T18):** 통합 검증 + project_progress 갱신

**Patterns from 2a/2b-i/2b-ii (강제):**
- spec § 5.X 직접 인용 (paraphrase 신뢰 X)
- 값 객체 표준 패턴 (`#[serde(transparent)]`, try_new, Display, FromStr)
- Aggregate 패턴 (struct + try_new + 도메인 메서드 + Repository trait)
- Reader 패턴 (read-only, R2 정적)
- CI 그린 게이트 (3 workflows)
- 466 누적 테스트 → +200 추가 목표
- `clippy::derive_partial_eq_without_eq` 사전 대응 (no float fields → `Eq` 추가)

**알려진 위험 (Sub-project 2a + 2b-ii lessons):**
- Aggregate 필드는 spec § 5/§ 4 verbatim — paraphrase 금지
- `entity_tests.rs` `#[path]` 사용 시 `super::Parcel` 같은 import 주의 (T4 lesson)
- 모든 식별자 백틱 (clippy::doc_markdown 사전 대응)

---

## Phase A: Market BC R2 Readers

### Task 1: RealTransaction Reader

**Spec 근거:** spec § 4 line 113 — Market BC R2 정적, 실거래가 이력.

**Files:**
- `crates/domain/market/real-transaction/{Cargo.toml, README.md, src/{lib,entity,errors,reader}.rs}`
- 워크스페이스 member 추가

**Aggregate (~10 fields, V-World/data.go.kr 추정):**

```rust
pub struct RealTransaction {
    pub id: String,                                   // 거래 식별자 (정부 표준)
    pub pnu: Pnu,                                     // 거래 필지
    pub building_id: Option<String>,                  // 거래 대상 건물 (있으면)
    pub transaction_kind: TransactionKind,            // sale/lease enum
    pub price_krw: MoneyKrw,                          // 거래 금액
    pub area_m2: AreaM2,                              // 거래 면적
    pub floor: Option<i16>,                           // 층 (음수 = 지하)
    pub transaction_date: NaiveDate,                  // 거래일 (정부 발표는 월 단위)
    pub fetched_at: DateTime<Utc>,
}

pub enum TransactionKind {
    Sale,            // 매매
    Jeonse,          // 전세
    MonthlyRent,     // 월세
}
```

**Reader trait:**

```rust
#[async_trait]
pub trait RealTransactionReader: Send + Sync {
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Vec<RealTransaction>, ReaderError>;
    async fn fetch_in_bbox(&self, bbox: &BoundingBox, since: NaiveDate)
        -> Result<Vec<RealTransaction>, ReaderError>;
}
```

`since` 파라미터는 *시점 필터* — R2 PMTiles는 거래일 단위 인덱스로 추정.

- [ ] BC crate 생성 + entity + reader trait + ≥10 tests + CI green

```bash
git commit -m "feat(real-transaction-domain): RealTransaction Aggregate + Reader (sale/jeonse/monthly_rent)"
```

### Task 2: CourtAuction Reader

**Spec 근거:** spec § 4 line 114 — 경매 정보 (활성 + 이력).

**Aggregate (~12 fields):**

```rust
pub struct CourtAuction {
    pub case_number: String,                          // 사건번호 (예: "2024타경12345")
    pub pnu: Pnu,
    pub kind: CourtAuctionKind,                       // forced/voluntary/...
    pub status: CourtAuctionStatus,                   // upcoming/in_progress/sold/cancelled
    pub appraisal_value: MoneyKrw,                    // 감정가
    pub minimum_bid: MoneyKrw,                        // 최저입찰가
    pub bid_count: u8,                                // 유찰 횟수
    pub auction_date: Option<NaiveDate>,              // 매각기일
    pub sold_price: Option<MoneyKrw>,                 // 낙찰가 (sold일 때만)
    pub sold_at: Option<NaiveDate>,                   // 낙찰일
    pub geom: Option<PointSrid>,                      // 위치 (있으면)
    pub fetched_at: DateTime<Utc>,
}

pub enum CourtAuctionKind {
    Forced,        // 강제경매
    Voluntary,     // 임의경매
    Other,
}

pub enum CourtAuctionStatus {
    Upcoming,      // 예정
    InProgress,    // 진행중 (입찰 가능)
    Sold,          // 낙찰
    Cancelled,     // 취하
    Failed,        // 유찰 (next round 대기)
}
```

**Reader trait:** fetch_by_case_number, fetch_active (status in upcoming/in_progress), fetch_in_bbox.

- [ ] BC crate + entity + reader + ≥12 tests + CI green

```bash
git commit -m "feat(court-auction-domain): CourtAuction Aggregate + Reader (kind/status enums)"
```

---

## Phase B: Insights BC RDS Aggregates

### Task 3: Bookmark Aggregates (BookmarkListing + BookmarkExternal)

**Spec 근거:** spec § 5.2 lines 246-275.

**Files:**
- `crates/domain/insights/bookmark/{Cargo.toml, src/{lib, listing, external, errors, repository}.rs}`

**BookmarkListing** (composite PK):

```rust
pub struct BookmarkListing {
    pub user_id: Id<UserMarker>,
    pub listing_id: Id<ListingMarker>,
    pub note: Option<String>,                         // ≤500자
    pub created_at: DateTime<Utc>,
}

impl BookmarkListing {
    pub fn try_new(user_id, listing_id, note: Option<String>, now) -> Result<Self, BookmarkError>
}
```

**BookmarkExternal** (polymorphic to R2 entities):

```rust
pub struct BookmarkExternal {
    pub id: Id<BookmarkExternalMarker>,               // bme_<26 ULID>
    pub user_id: Id<UserMarker>,
    pub target_kind: BookmarkExternalKind,            // 4-variant per V003 spec
    pub target_id: String,                            // PNU 또는 R2 식별자
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub enum BookmarkExternalKind {
    Parcel,
    CourtAuction,
    Manufacturer,
    IndustrialComplex,
}
```

**Marker 추가:** shared-kernel id.rs에 `BookmarkExternalMarker` (`PREFIX = "bme"`).

**Repository trait:**

```rust
#[async_trait]
pub trait BookmarkRepository: Send + Sync {
    async fn find_listing_bookmarks(&self, user_id: &Id<UserMarker>) -> Result<Vec<BookmarkListing>, RepoError>;
    async fn find_external_bookmarks(&self, user_id: &Id<UserMarker>) -> Result<Vec<BookmarkExternal>, RepoError>;
    async fn save_listing_bookmark(&self, bm: &BookmarkListing) -> Result<(), RepoError>;
    async fn save_external_bookmark(&self, bm: &BookmarkExternal) -> Result<(), RepoError>;
    async fn delete_listing_bookmark(&self, user_id: &Id<UserMarker>, listing_id: &Id<ListingMarker>) -> Result<(), RepoError>;
    async fn delete_external_bookmark(&self, id: &Id<BookmarkExternalMarker>) -> Result<(), RepoError>;
}
```

- [ ] 2 Aggregate + Marker + Repository + ≥15 tests + CI green

### Task 4: SearchHistory Aggregate

**Spec 근거:** spec § 5.2 lines 277-292.

```rust
pub struct SearchHistory {
    pub id: Id<SearchHistoryMarker>,                  // srh_<26 ULID>
    pub user_id: Option<Id<UserMarker>>,              // nullable (비로그인)
    pub query: String,                                // ≤500자
    pub filters: serde_json::Value,                   // jsonb
    pub result_count: u32,
    pub correlation_id: String,                       // ≤30자, 트레이싱 stitch
    pub created_at: DateTime<Utc>,
}
```

`SearchHistoryMarker` (PREFIX `"srh"`) shared-kernel id.rs 추가.

**Repository trait:** find_by_user (가명화 retention 90일 후), insert (대량 — 매 검색마다).

- [ ] Aggregate + Marker + Repository + ≥10 tests + CI green

### Task 5: AnalysisReport Aggregate

**Spec § 5.2** lines 294-308.

```rust
pub struct AnalysisReport {
    pub id: Id<AnalysisReportMarker>,                 // rpt_<26 ULID>
    pub user_id: Id<UserMarker>,
    pub title: String,                                // ≤200
    pub target_pnus: Vec<Pnu>,                        // ≥1, ≤50 (응답 크기 제한)
    pub snapshot: serde_json::Value,                  // R2 데이터 시점 캐시
    pub created_at: DateTime<Utc>,
    pub version: i64,                                 // optimistic locking
}

impl AnalysisReport {
    pub fn try_new(...) -> Result<Self, AnalysisReportError>;
    pub fn rename(&mut self, new_title: String, at: DateTime<Utc>);  // version bump
}
```

`AnalysisReportError`: EmptyTitle, TitleTooLong, EmptyTargetPnus, TooManyTargetPnus.

`AnalysisReportMarker` (PREFIX `"rpt"`).

**Repository:** find_by_id, find_by_user, save (optimistic lock).

- [ ] Aggregate + Marker + try_new + rename + Repository + ≥12 tests + CI green

### Task 6: Notification Aggregate

**Spec § 5.2** lines 310-320.

```rust
pub struct Notification {
    pub id: Id<NotificationMarker>,                   // ntf_<26 ULID>
    pub user_id: Id<UserMarker>,
    pub kind: String,                                 // 'bookmark_listing_changed' 등 (≤50자)
    pub payload: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn try_new(...) -> Result<Self, NotificationError>;
    pub fn mark_read(&mut self, at: DateTime<Utc>);   // idempotent
}
```

`NotificationMarker` (PREFIX `"ntf"`).

**Repository:** find_unread_by_user, find_by_id, save, mark_read_by_user_and_kind (batch).

- [ ] Aggregate + Marker + Repository + ≥10 tests + CI green

---

## Phase C: AuditLog + DomainEvent trait

### Task 7: DomainEvent trait (shared-kernel)

**Goal:** Aggregate 도메인 메서드가 *이벤트를 emit*할 수 있게. Outbox publisher가 emit된 이벤트를 RDS `outbox_event`에 저장. Sub-project 4의 publisher가 외부 시스템에 발행.

**Files:** `crates/domain/core/shared-kernel/src/domain_event.rs`

```rust
//! 도메인 이벤트 trait — Aggregate 도메인 메서드가 emit하는 이벤트의 공통 형태.

/// 도메인 이벤트 (`Outbox` 패턴의 첫 단계).
///
/// Aggregate 도메인 메서드 (`Listing::approve`, `User::verify_business` 등)가
/// 상태 변경 후 `Vec<Box<dyn DomainEvent>>` 또는 `Vec<EnumWrapper>`를 반환할 수 있어요.
/// Application layer가 이를 받아 `OutboxRepository`에 저장.
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// 이벤트 종류 (`<aggregate>.<verb>` 패턴, 예: `listing.approved`, `user.business_verified`).
    fn event_type(&self) -> &'static str;
    /// 이벤트 발생 시각.
    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc>;
    /// 관련 Aggregate 식별자 (string 표현).
    fn aggregate_id(&self) -> String;
    /// 이벤트 페이로드 (JSON serialize 가능).
    fn payload(&self) -> serde_json::Value;
}
```

**Note:** trait object pattern이라 *모든 Aggregate event struct*는 `Box<dyn DomainEvent>`로 반환 가능. 또는 BC별 enum wrapper 사용. 본 plan은 trait만 정의 — emit하는 메서드는 후속 task에서 case-by-case.

- [ ] trait + ≥3 stub event impl로 trait 동작 확인 + CI green

### Task 8: AuditLog Aggregate

**Spec § 5.3** lines 326-345.

```rust
pub struct AuditLog {
    pub id: Id<AuditLogMarker>,                       // aud_<26 ULID>
    pub actor_id: Option<Id<UserMarker>>,             // None = system
    pub action: String,                               // ≤100자
    pub resource_kind: String,                        // ≤50자
    pub resource_id: String,                          // ≤50자
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,                   // ≤500자
    pub correlation_id: String,                       // ≤30자
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    pub fn try_new(...) -> Result<Self, AuditLogError>;
    // *no mutation methods* — append-only invariant
}
```

`AuditLogMarker` (PREFIX `"aud"`).

**Repository (insert-only — V002 트리거가 UPDATE/DELETE 차단):**

```rust
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn insert(&self, log: &AuditLog) -> Result<(), RepoError>;
    async fn find_by_resource(&self, kind: &str, id: &str)
        -> Result<Vec<AuditLog>, RepoError>;
    async fn find_by_actor(&self, actor_id: &Id<UserMarker>, since: DateTime<Utc>)
        -> Result<Vec<AuditLog>, RepoError>;
    // *no save/update/delete* — V002 immutable trigger
}
```

- [ ] Aggregate + Marker + insert-only Repository + ≥10 tests + CI green

```bash
git commit -m "feat(audit-domain): AuditLog Aggregate + insert-only Repository (immutable per V002)"
```

---

## Phase D: OutboxEvent + Outbox Repository

### Task 9: OutboxEvent Aggregate

**Spec § 5.3** lines 347-365.

```rust
pub struct OutboxEvent {
    pub id: Id<OutboxEventMarker>,                    // oev_<26 ULID>
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

`OutboxEventMarker` (PREFIX `"oev"`).

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
