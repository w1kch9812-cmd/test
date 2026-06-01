---
name: 프로젝트 진행 현황 (2026-05-04)
description: SP1+2+3+5-i+5-iii+5-iv+4-i+5-ii+4-ii+FU34+4-iii-d+4-iii-a 완료 (31 crate, ~1230 tests). FU 27/34 closed. SP4-iii-a = data.go.kr 건축물대장.
type: project
---

## ⚠️ 환경 변경 (2026-05-04)

- **로컬 머신 변경**: 어제 (2026-05-03) MSVC Build Tools 설치된 머신과 다른 환경에서 fresh `git clone` 진행 → cargo / rustup 미설치
- **결과**: SP5-iv (T1-T10) 작업이 로컬 cargo 검증 없이 진행됨 → **CI 그린 검증 필수**
- **검증 경로**: 사용자가 push 결정 시 GitHub Actions 3 workflow (CI / db-migrations / walking-skeleton) 가 진실 — 빨강 시 즉시 fix commit
- 다음 SP 시작 전 rustup 설치 권장 (1.88.0 + rustfmt + clippy + rust-analyzer)

## ⚠️ 인프라 변경 (2026-05-03)

- **Repo rename + visibility**: `w1kch9812-cmd/gongzzang3` (private) → `w1kch9812-cmd/test` (public)
- **이유**: GH Actions free-tier 빌링 소진 (5월 31일까지 reset 대기) → 무료 CI 위해 임시 public
- **새 origin**: `https://github.com/w1kch9812-cmd/test.git`
- **MSVC Build Tools 2022 설치 완료** — 로컬 cargo check/clippy/test/fmt 모두 작동, 더 이상 CI 단독 게이트 아님
- **로컬 검증 1050 tests 그린** (`cargo test --workspace`), `cargo clippy --workspace --all-features -- -D warnings` 5초 만에 통과 (CI 동일 명령)
- 후속: production 운영 단계 직전에 다시 private 전환 — `docs/auth/staging-zitadel-integration.md` 와 동일한 deferred infra 처리 항목

## 완료된 Sub-projects

### Sub-project 1: 헌법 + 모노레포 (완료)
- 132 파일 (헌법 + ADR 11개 + 컨벤션 9개 + 모노레포 골격)
- 자동 강제 도구: lefthook, gitleaks, biome, clippy, cargo-deny, markdownlint

### Sub-project 2a: DB + shared-kernel (완료, 31 task)
- 18 RDS 테이블 (V001 5분할) + V002 (3 role + audit immutable trigger)
- shared-kernel crate, 14 값 객체 (Pnu, Money, Email, BusinessNumber + checksum 등)
- tarpaulin ≥90% CI 게이트

### Sub-project 2a-fixup: spec 결함 5건 보강 (완료)
- V003_01: listing transaction_type cross-field CHECK
- V003_02: BVQ + LRQ optimistic locking version
- V003_03: featured_content ends_at > starts_at CHECK
- BusinessNumber 000xxxxxxx prefix 거부
- PhoneKr +82 명시적 prefix만 strip

### Walking Skeleton (완료, T1-T5)
- User Aggregate minimal (`crates/domain/core/user`)
- PgUserRepository (SQLx, `crates/db`)
- Axum HTTP server (`services/api`, 3 endpoint)
- CI smoke test workflow — POST /users + GET /users/:id round-trip

### Sub-project 2b-i: Core BC RDS Aggregates (완료, T1-T13)
- 워크스페이스 재구조 (shared-kernel → `crates/domain/core/`)
- 6 신규 값 객체 (ListingType, TransactionType, ListingStatus + 상태 머신, ContactVisibility, ListingTitle, Description)
- User Aggregate full (18 필드, 13 도메인 메서드, soft-delete)
- Listing Aggregate full (20 필드, 9 도메인 메서드, 상태 머신, V003_01 invariant)
- ListingPhoto Aggregate (12 필드, soft-delete + reorder)
- Repository trait 3개 (User/Listing/ListingPhoto), 모두 port-only

### Sub-project 2b-ii: Core BC R2 정적 Reader (완료, T1-T8)
- shared-kernel 추가: LandUseType, Zoning, PolygonSrid, BoundingBox, AdminDivision composite
- 4 R2 정적 BC 신규 crate: Parcel, Building, IndustrialComplex, Manufacturer
- Reader trait 4개, 모두 read-only port (구현은 sub-project 4)

### Sub-project 2c: Market + Insights + Audit + Pipeline + Operations BC (완료, 14 task — T1-T18)
- T1 RealTransaction Aggregate (Market BC)
- T2 CourtAuction Aggregate (Market BC)
- T3 Bookmark Aggregate (Insights BC)
- T4 SearchHistory Aggregate (Insights BC)
- T5 AnalysisReport Aggregate (Insights BC)
- T6 Notification Aggregate (Insights BC)
- T7 shared-kernel `DomainEvent` trait + ULID id 표준 (4 tests 추가)
- T8 AuditLog (Audit BC, immutable)
- T9-T10 OutboxEvent + Outbox 패턴 (Audit BC)
- T11-T12 PipelineSchedule + PipelineRun + steps JSONB (data-pipeline-control)
- T13 AdminAction (Operations BC)
- T14 BusinessVerificationQueue (Operations BC, optimistic locking)
- T15 ListingReviewQueue (Operations BC, optimistic locking)
- T16 ListingReport (Operations BC)
- T17 OperationsMeta (FeaturedContent + AlertHistory, 단일 crate)
- T18 통합 검증 + memory 갱신

**누적**: 14 신규 crate (Market 2 + Insights 4 + Audit 2 + Pipeline 1 + Operations 5),
1017 단위 테스트, Rust 1.88, 24 workspace member.

### Sub-project 3: Auth — Zitadel JWT 핵심 게이트 (완료, T1-T10)

- 신규 crate: `crates/auth` (`Verifier` enum + `JwksCache` + middleware + extractor + `require_role`)
- `Verifier::Real(JwtVerifier)` — Zitadel `RS256` + `JWKS` 검증
- `Verifier::Dev` — Mock JWT (`DEV.<sub>` 형식, `AUTH_DEV_MODE=true` 시 사용)
- `services/api`: 미들웨어 적용 (`/healthz` public, `/users/me`/`/users/:id` 인증 보호), `POST /users` 제거
- migration `30005`: `user.roles` CHECK 제약 (7 enum 값)
- walking-skeleton.yml: Zitadel 컨테이너 대신 Mock JWT 6단계 e2e (healthz / 401-no-token / 401-bad / first-sign-in / no-dup / different-sub)
- T9 첫 시도 (Zitadel 컨테이너) 7 iter 실패 → docs/auth/staging-zitadel-integration.md 에 deferred 기록
- 누적 테스트: 1017 → **1050** (auth crate +33), 25 crate

**SP3 미포함 (후속)**:
- 진짜 Zitadel staging 통합 테스트 (deferred)
- 소셜 로그인 (Google/Kakao/Naver/Apple)
- NICE 본인인증
- 2FA / WebAuthn
- endpoint 별 RBAC 매트릭스

### Sub-project 5-i: Core BC RDS Repository SQLx (완료, T1-T6)

- 신규: `crates/db/src/error_map.rs` (`MapFromSqlx` trait + `map_sqlx_err` helper)
- 신규: `crates/db/src/listing.rs` (`PgListingRepository` — 21 필드, `PostGIS` `ST_X`/`ST_Y` round-trip, `OCC`, `ListingMarker` projection)
- 신규: `crates/db/src/listing_photo.rs` (`PgListingPhotoRepository` — 12 필드, soft-delete, hard delete with `NotFound`, `cascade` 검증)
- 보강: `crates/db/src/user.rs` 8 필드 → 18 필드 (roles/business_number/broker_license/*_verified_at 모두)
- 보강: `listing-photo-domain` `RepoError` 에 `Conflict` variant 추가 (T1 fallback 해소, SSS 일관성)
- 모든 repo 메서드 `#[tracing::instrument]` (`skip(self)` PII 미노출 패턴)
- `Cargo.toml` `[features] integration = []` + `walking-skeleton.yml` 에 `cargo test --features integration` 단계 + 통합 테스트 후 `truncate cascade` reset
- `bigdecimal` dep 추가 (`numeric(12,2)` ↔ `f64` bridge)
- 통합 테스트 23 (User 6 + Listing 9 + ListingPhoto 6 + error_map 2) + 단위 2 (error_map) → 누적 ~1075

**SP5-i 미포함 (후속)**:
- `Outbox` 트랜잭션 → SP5-iii
- `audit_log` 자동 INSERT → SP5-iii
- `R2` Reader 6 (Parcel/Building/IC/Mfr/RealTransaction/CourtAuction) → SP4 (외부 API ingestion)
- `sqlx::query!()` macro 채택 → 별도 ADR
- HTTP 응답 매핑 (`RepoError → IntoResponse`) → 별도 sub-project

**SP5-i 발견 사항 (lessons)**:
- T1: listing-photo `RepoError` 에 `Conflict` variant 부재 — T4 에서 추가하며 해소
- T3: spec plan 의 `PointSrid::new(Point<f64>)` 가정 틀림 — 실제 `PointSrid::try_new_wgs84(lng, lat) + pub fields lng/lat`
- T3: `MoneyKrw::as_i64()` 사용 (plan 의 `i64::from(...)` 가정 틀림)
- T3: `AreaM2::as_f64()` + `BigDecimal` bridge (plan 의 `Decimal` 가정 틀림)
- T5: 통합 테스트가 `psql truncate cascade` 로 격리 + `--test-threads=1` 직렬 실행 + reset step 으로 후속 E2E 보호

### Sub-project 5-iii: Audit + Pipeline + Operations BC RDS Repo + 트랜잭션 Outbox (완료, T1-T11)

- 신규: `MutationContext` (`crates/domain/core/shared-kernel/src/mutation.rs`) + 6 단위 테스트
  · `actor_id` (Option), `correlation_id`, `action`, `metadata`, `events: Vec<Arc<dyn DomainEvent>>`, `client_ip`, `user_agent`, `occurred_at`
  · `new_user_action` / `new_system_action` / `with_metadata` / `with_events` / `with_client_info` / `with_occurred_at`
- 6 도메인 trait 시그니처 변경 — `save`/`insert` 메서드에 `ctx: MutationContext` 추가 (pipeline / admin-action / bvq / lrq / listing-report / operations-meta)
- 8 신규 PgRepository (`crates/db/src/{audit_log,outbox,admin_action,bvq,lrq,listing_report,operations_meta,pipeline}.rs`)
- `error_map.rs` 8 신규 도메인 `MapFromSqlx` impl
- **트랜잭션 패턴**: PgRepository.save() 가 tx 안에서 `[INSERT/UPDATE Aggregate + INSERT audit_log + INSERT outbox_event for each event]` 모두 atomic. 부분 실패 → 모두 rollback (자동)
- AuditLog/Outbox 자체 repo 는 transactional 패턴 *대상 아님* (recursion 방지)
- `audit_log` V002 immutable trigger 동작 검증 (UPDATE/DELETE 차단)
- 통합 테스트 39 신규 (audit 5 + outbox 6 + admin_action 4 + bvq 5 + lrq 5 + listing_report 4 + operations_meta 5 + pipeline 5) + 단위 6 → 누적 ~1120

**SSS 7기둥 결함 닫음**:
- 추적성: 모든 mutation 이 audit_log 자동 + correlation_id + actor_id 추적
- 일관성: OutboxEvent 패턴 작동 (이전엔 trait 정의만, 실제 INSERT 0)
- 안전성: tx atomic — audit 실패 = 전체 실패

**SP5-iii 발견 사항 (lessons)**:
- Trait doc stale 다수: `find_by_resource(limit)` `find_by_actor(since)` 등 spec 문서가 실제 trait 보다 뒤짐 → 코드가 SSOT
- Entity-DB asymmetry: BVQ/LRQ entity 의 `updated_at` 필드는 DB 미존재 → SELECT 시 합성 (`reviewed_at.unwrap_or(submitted_at)`) → spec FU 14 후보
- OCC API 한계: `RepoRepo::save(aggregate, ctx)` 가 caller 의 read-시점 version 을 묵시 의존 → `expected_version` 명시 인자가 더 명확 → spec FU 15 후보
- AuditLog 컬럼 mismatch: spec § 4.3 mock 의 `metadata` 컬럼 → 실제 schema 는 `before_state`/`after_state`/`ip_address` (plan 에서 정정해 따름)
- LRQ `find_by_listing` 은 multi-row corruption 시 silent shadow → `UNIQUE INDEX listing_review_queue(listing_id) WHERE decision IS NULL` 추가 검토 → spec FU 16 후보
- AuthCrate clippy 빚: `crates/auth/src/verifier.rs` 의 pre-existing `clippy::panic` + `clippy::manual_let_else` — SP3 잔재, 별도 정리 필요

**SP5-iii 미포함 (후속)**:
- SP5-iv: SP5-i 의 User/Listing/ListingPhoto save() 에 MutationContext 추가 → ✅ **완료** (2026-05-04)
- SP5-ii: Insights BC RDS (Bookmark/SearchHistory/AnalysisReport/Notification)
- SP4: 외부 API ingestion + R2 Reader + Outbox publisher worker
- AuditLog full diff capture (before_state + after_state) — 별도
- OperationsMeta `find_unacknowledged_alerts` trait doc 갱신 (created_at ASC → severity DESC + created_at DESC)

### Sub-project 5-iv: Core BC `MutationContext` 일원화 (완료, T1-T10)

- 3 도메인 trait 시그니처 변경: `UserRepository::save` / `ListingRepository::save` /
  `ListingPhotoRepository::save` 모두 `(agg, ctx: MutationContext)`. `ListingPhotoRepository::delete`
  도 `(id, ctx)` 로 — hard delete 도 audit 대상.
- 3 PgImpl 트랜잭션화 (`crates/db/src/{user,listing,listing_photo}.rs`):
  · 1) `pool.begin()` → 2) Aggregate UPSERT (또는 hard delete) → 3) `audit_log` INSERT
  (`resource_kind = 'user' / 'listing' / 'listing_photo'`) → 4) `ctx.events` 마다 `outbox_event`
  INSERT (`aggregate_kind` 동일) → 5) `tx.commit()`. 부분 실패 → 자동 rollback (tx Drop)
  · `PgListingPhotoRepository` 만 `write_audit_log` + `write_outbox_events` 모듈-private
  헬퍼로 추출 (save / delete 공유)
- `crates/auth/src/middleware.rs`: first-sign-in 이 `MutationContext::new_system_action(claims.sub,
  "first_sign_in").with_metadata({"zitadel_sub": ...})` 호출 후 `repo.save(&user, ctx)`. race
  재시도는 find 재호출만 — 추가 ctx 불필요
- `crates/db/tests/common.rs` 신규 helper `pub fn test_ctx() -> MutationContext` →
  `new_system_action("test-seed", "create")`. seed 호출 일괄 통일
- 통합 테스트 신규 10 (User 4 + Listing 3 + ListingPhoto 3):
  · `save_inserts_<kind>_audit_log_in_one_tx` — audit_log 1 row 검증
  · `save_<kind>_with_events_inserts_outbox_per_event` — outbox row 수 검증
  · `save_<kind>_system_action_records_null_actor` — actor_id NULL
  · User 만: `save_user_with_metadata_writes_to_after_state` — metadata → after_state JSON
  · ListingPhoto 만: `delete_photo_audit_logs_with_action_delete` — hard delete 도 audit
- 기존 통합 테스트 9 파일 (user / listing / listing_photo / error_map / bvq / lrq /
  listing_report / operations_meta / admin_action) seed 호출 모두 `test_ctx()` 사용. PgOutboxRepository
  call sites 는 의도적으로 미변경 (Outbox 자체는 transactional 패턴 대상 아님)
- 누적 테스트: 1120 → ~1130 (단위 1058 + 통합 72)
- 환경 한계: 본 작업 머신에 cargo / rustup 미설치 → 로컬 검증 0. CI 푸시로 검증 필요

**SSS 7 기둥 결함 닫음 (SP5-iii 가 6 BC 에서 닫은 것 + Core BC 3 BC 추가)**:
- 1 일관성: 9 BC 모두 동일 `save(agg, ctx)` 시그니처 + transactional 패턴. SP5 시리즈 종료
- 3 추적성: User/Listing/ListingPhoto 의 save (+ photo delete) 모두 audit_log row 자동 INSERT
- 6 SSOT: Repository trait 시그니처 단일화 — 신규 BC 도 같은 패턴 채택 강제

**SP5-iv 미포함 (후속)**:
- SP6 시작 시: `MutationContext` 가 application layer 에서 자주 쓰이므로 `services/api` 에
  `http_user_action(req, action)` 류 helper 추가 검토 (FU 19)
- HTTP X-Request-ID → `correlation_id` 자동 주입 (Axum middleware) → SP7 관측성과 묶음
- AuditLog full diff (`before_state` + `after_state`) — 9 BC 공통 후속

### Sub-project 4-i: Outbox Publisher Worker (완료, T1-T7)

- 신규 lib crate `crates/outbox-publisher`:
  · `Sink` trait — outbox event 발행 추상화 (멱등성 의무 명시)
  · `LoggingSink` default — `tracing::info!` target=`outbox.publish` 구조화 발행
  · `CountingSink` 테스트용 `AtomicU64` 카운터
  · `tick(repo, sink, limit) -> TickReport { fetched, published, failed }` — 1 사이클 단위
  · `SinkError`, `PublisherError` enum
- 신규 binary crate `services/outbox-publisher` (binary name: `outbox-publisher`):
  · env: DATABASE_URL, OUTBOX_POLL_INTERVAL_MS=1000, OUTBOX_BATCH_SIZE=100
  · `tracing-subscriber` JSON output (Loki 친화)
  · `tokio::time::interval (MissedTickBehavior::Skip)` → 매 tick `outbox_publisher::tick`
  · 빈 tick silent (운영 spam 방지)
  · `SIGTERM` (Unix) / `Ctrl+C` graceful shutdown — Windows 는 `#[cfg(not(unix))] pending`
- 통합 테스트 신규 4 (`crates/db/tests/outbox_publisher_integration.rs`):
  · `tick_publishes_unpublished_rows` — 3 row INSERT → tick → published_at 모두 NOT NULL
  · `tick_skips_already_published` — 이미 mark 된 row 는 fetch 안 잡힘
  · `tick_returns_zero_when_no_rows` — 빈 테이블 → 0
  · `tick_failure_leaves_row_unpublished` — `FailingSink` (file inline) → row 미발행 유지
- 단위 테스트 신규 6 (sink: 3 + publisher: 3)

**SSS 7 기둥 결함 닫음**:
- 1 일관성: Outbox 약속의 read side 채움. SP5-iii/iv 의 outbox INSERT 가 publisher 로 발행됨
- 4 안전성: at-least-once 발행 + 멱등성 의무 명시. tick 단위 격리 — 개별 sink 실패가 batch 막지 않음
- 5 가시성: tick report (fetched/published/failed) + LoggingSink target 으로 운영 dashboard 즉시 활용

**SP4-i 발견 사항 (lessons)**:
- T1 코드 자체는 단순 — SP5-iii 패턴 답습, 1시간 분량
- T6 CI 검증에서 clippy 빨강 4 iter:
  · iter 1: `clippy::module_name_repetitions` (Sink/LoggingSink/SinkError 가 module sink 내에) → file-level allow
  · iter 2: `clippy::match_wildcard_for_single_variants` (`PublisherError` single-variant) → 명시적 패턴
  · iter 3: `clippy::ignored_unit_patterns` (services/outbox-publisher main.rs `_ = term => {}`) → `() = term => {}`
  · iter 3 동시 적용: `redundant_async_block`, `redundant_pub_crate` allow
- **로컬 cargo 검증 한계**: rustup x86_64-pc-windows-gnu 설치 + bundled rust-mingw 만 으론 cc1 부재 → sqlx/ring 같은 C-dep crate 빌드 불가. 시스템 MSVC Build Tools 또는 portable WinLibs MinGW 다운로드는 모두 hook 차단. 결과: services/outbox-publisher 빌드는 push 후 CI 가 진실
- **CI clippy 범위**: `cargo clippy --workspace --all-features -- -D warnings` 는 `--all-targets` 없이 lib + bin 만 lint. tests/examples/benches 미포함. 실제 빨강 위치 좁히는 데 시간 절약 가능

**SP4-i 미포함 (후속)**:
- 분산 락 (`SELECT FOR UPDATE SKIP LOCKED` 또는 advisory lock) — 멀티 인스턴스 시
- 외부 sink 구현체 (Kafka / Webhook HTTP POST / SQS / NATS) — SP4-iii+
- 재시도 정책 (`attempt_count` 컬럼 + DLQ)
- Circuit breaker 통합 — 외부 sink 도입 시
- Prometheus metrics (`outbox_published_total{aggregate_kind, sink}`) — SP7

### Sub-project 5-ii: Insights BC RDS Repository (완료, T1-T9)

- 4 도메인 trait 시그니처 변경 — mutation 메서드에 `MutationContext` 추가:
  · `BookmarkRepository`: save_listing/external + delete_listing/external
  · `SearchHistoryRepository`: insert + pseudonymize_older_than
  · `AnalysisReportRepository`: save (OCC) + delete
  · `NotificationRepository`: insert + mark_read + mark_all_read_by_kind
- 4 신규 PgRepository (`crates/db/src/`):
  · `bookmark.rs` (~370 lines) — composite PK `(user_id, listing_id)` UPSERT +
    polymorphic external (id PK + UNIQUE `(user_id, target_kind, target_id)`).
    write_audit_log/outbox 헬퍼 가 `resource_kind` / `aggregate_kind` 인자화로
    listing/external 두 Aggregate 처리
  · `search_history.rs` (~250 lines) — append-mostly + bulk pseudonymize.
    `pseudonymize_older_than` 단일 audit row + override_metadata
    `{cutoff_iso, rows_pseudonymized}` 보존
  · `analysis_report.rs` (~280 lines) — OCC + `target_pnus char(19)[]` ↔
    `Vec<Pnu>` round-trip via `Vec<&str>` bind. SP5-iv 와 동일 OCC 패턴
  · `notification.rs` (~310 lines) — append + 멱등 `mark_read` + bulk
    `mark_all_read_by_kind`. mark_read 는 rows_affected 검증 없이 멱등 (이미
    읽음 / row 미존재 모두 OK). bulk metadata `{kind, rows_marked, marked_at_iso}` 보존
- 4 도메인 `MapFromSqlx` impl in `error_map.rs` (Bookmark/SearchHistory/Notification
  은 fallback `Database`, AnalysisReport 는 OCC `Conflict`)
- `crates/db/Cargo.toml` 4 도메인 dep 추가
- `crates/db/tests/common.rs` `truncate_all` 에 5 테이블 추가
- 통합 테스트 신규 22 (`crates/db/tests/`):
  · `bookmark_integration.rs` (6): listing/external round-trip + delete audit +
    delete NotFound + outbox events + UPSERT updates note
  · `search_history_integration.rs` (4): insert audit + anonymous null
    user_id + bulk pseudonymize (rows + bulk audit) + metadata 검증
  · `analysis_report_integration.rs` (6): target_pnus[] round-trip + version
    bump + OCC conflict + delete audit + delete NotFound + find_by_user
  · `notification_integration.rs` (4): insert audit + mark_read + 멱등
    mark_read + bulk mark_all_read_by_kind (rows + audit metadata)
- 누적 테스트: 1142 → ~1166 (단위 1063 + 통합 103)

**SSS 7기둥 결함 닫음**:
- 1 일관성: 13 BC 모두 동일 `save(agg, ctx)` / `insert(agg, ctx)` 패턴. SP5
  시리즈 완전 종료
- 3 추적성: Insights BC mutation 도 audit_log 자동 기록. bulk operation
  (pseudonymize / mark_all_read_by_kind) 도 단일 audit row + metadata 로 추적
- 6 SSOT: Repository trait 시그니처 일원화

**SP5-ii 발견 사항 (lessons)**:
- T1-T7 모두 한 번에 push 후 CI 그린 (clippy 빨강 0). SP4-i 의 4 iter 빨강
  (module_name_repetitions, match_wildcard, ignored_unit_patterns) 학습 효과
  — 미리 차단
- 로컬 cargo clippy 4 도메인 crate (proc-macro 만 의존) 는 link 가능 →
  `cargo +1.88.0-x86_64-pc-windows-gnu clippy -p bookmark-domain
  -p search-history-domain -p analysis-report-domain -p notification-domain
  --all-features --all-targets -- -D warnings` 그린 검증 후 push (PgImpl 은
  여전히 sqlx/ring 으로 link 불가 — CI 가 진실)
- bookmark composite PK delete 의 `audit_log.resource_id` 는 listing_id (30
  chars) 만 — varchar(50) 안전. user_id 는 actor_id 가 별도 capture
- bulk operation audit metadata 패턴 검증 — `pseudonymize_older_than` 의
  `{cutoff_iso, rows_pseudonymized}` + `mark_all_read_by_kind` 의 `{kind,
  rows_marked, marked_at_iso}`
- target_pnus char(19)[] round-trip: write 는 `Vec<&str>`, read 는
  `Vec<String>` → `Pnu::try_new` 도메인 검증. sqlx 가 text[] 호환 처리

**SP5-ii 미포함 (후속)**:
- FU 21: Bookmark count denormalization (`listing.bookmark_count` 동기) —
  outbox consumer 또는 trigger
- FU 22: AnalysisReport `target_pnus` GIN 인덱스 — 사용자 통계 쿼리 시
- FU 23: Notification push delivery (FCM/APNS/WebPush) — Outbox sink 추가
- FU 24: SearchHistory NLP / 임베딩 (Phase 3+)
- FU 25: 365일 알림 retention 워커 (`services/worker/notification_retention`)

### Sub-project 4-ii: V-World 외부 API + Circuit Breaker (완료, T1-T7)

- 신규 lib `crates/circuit-breaker` (~600 lines):
  · `Policy` (Copy) — `timeout_ms`, `max_retries`, `retry_base_ms`,
    `open_threshold`, `open_window_ms`, `open_cooldown_ms`. `vworld_default()` 상수
  · `Breaker` — `std::sync::Mutex<Inner>` + 3-state machine (Closed/Open/HalfOpen),
    sliding window failure 카운터 (VecDeque<Instant>) + cooldown 자동 전이
  · `prune_window` 헬퍼 추출 (cognitive_complexity 분해)
  · `execute(breaker, policy, op_name, op)` — timeout(`tokio::time::timeout`) +
    attempt loop (지수 백오프 `retry_base_ms * 2^attempt`) + state 추적
  · `BreakerError<E: Display>` (where 절) — Open/Timeout/MaxRetriesExceeded/Inner
  · 23 단위 테스트 (state transitions / retries / timeouts)
- 신규 lib `crates/data-clients/vworld` (~700 lines):
  · `VWorldConfig` + `from_env()` (VWORLD_API_KEY/DOMAIN/BASE_URL)
  · `VWorldClient` (reqwest::Client + Breaker + Policy) — `fetch_feature_by_pnu`
    이 WFS GetFeature URL build + `circuit_breaker::execute` 통과
  · `parser::parse_parcel` — V-World JSON → 도메인 `Parcel` ACL:
    pnu/admin(시도/시군구/읍면동)/jibun_address/lndcgr_nm 한글 지목 매핑/
    uq_nm 한글 용도지역 매핑/lndpcl_ar/geometry.coordinates → PolygonSrid
  · `RawCapture` trait + `NoOpRawCapture` (tracing event "vworld.raw"). DB
    저장 구현체는 SP4-iii (FU 27)
  · `VWorldParcelReader` impl `ParcelReader::fetch_by_pnu` —
    client → raw_capture (best-effort) → parser → Parcel
  · `fetch_markers_in_bbox` 는 honest failure (`Err(Fetch("deferred to SP4-iii"))`)
  · 12 단위 테스트 + 6 wiremock 통합 (happy path / empty / 5xx / malformed /
    circuit open / deferred bbox)
- workspace.members + wiremock dev-dep 추가
- 누적 테스트: 1166 → ~1195 (단위 +29 / 통합 +6)

**SSS 7기둥 결함 닫음**:
- 1 일관성: 모든 외부 API 호출이 `circuit_breaker::execute` 통과 — 후속 API 도
  같은 패턴 강제 (SP4-iii data.go.kr/법제처 도입 시)
- 4 안전성: timeout + retry + circuit open — 외부 장애 격리. Honest failure
  (5xx 를 mock 으로 덮지 않음)
- 5 가시성: 모든 호출 `tracing::instrument` + state 전이 시 warn/info

**SP4-ii 발견 사항 (lessons)**:
- **Iteration 비용**: CI 4번 (193/194/195/196) clippy 빨강 → MSVC 부재로 로컬
  진단 불가 → 추측 fix → 또 빨강. 결국 사용자 승인으로 winget 으로 MSVC
  Build Tools 2022 silent install 후 정확 진단 1회 → 1d4c9be 그린.
  **로컬 빌드 환경이 SP 시작의 사전조건**.
- 실제 lint 1건: `Breaker::record_failure` cognitive_complexity 20/15
  (clippy.toml threshold 15). `prune_window` helper 분리 후 18, 추가 분리는
  readability 손해 → `#[allow]` + 사유 doc
- vworld-client `lib.rs` 의 `clippy::doc_markdown` 1건 (raw_response 등
  lowercase identifier) → file-level allow
- CI 가 `--all-targets` 미사용 — 통합 테스트는 lint 안 됨. 로컬 `--all-targets`
  시 기존 crate 들 (shared-kernel float_cmp, user-domain redundant_clone) 의
  잠복 부채 발견 — FU 34
- wiremock 6.0 통합: `MockServer::start()` + `Mock::given(method+path).respond_with`,
  `.mount(&server)`. base_url override 로 클라이언트가 mock 가리킴 — clean

**SP4-ii 미포함 (후속)**:
- FU 26: `clippy::disallowed_types` 로 reqwest::Client 직접 호출 차단 (data-clients
  외 다른 crate 가 우회 못 하게)
- FU 27: ✅ **완료 (SP4-iii-d)** — `parcel_external_data` 마이그 + `PgRawCapture`
- FU 28: Redis 캐시 레이어 (TTL 24h)
- FU 29: Sentry alert on Breaker open
- FU 30: `fetch_markers_in_bbox` PMTiles 또는 WFS BBOX
- FU 31: Distributed circuit breaker (Redis 공유 state)
- FU 32: `governor` rate limit (V-World 일일 쿼터)
- FU 33: vworld-client clippy specific allow 분해 (현재는 doc_markdown 만)
- FU 34: ✅ **완료** — workspace `--all-targets` 부채 일괄 정리 + CI 강화

### Sub-project 4-iii-d: RawCapture trait 분리 + PgRawCapture (완료, T1-T7)

- 신규 lib `crates/data-clients/raw-capture` — `RawCapture` trait, `RawCaptureError`,
  `NoOpRawCapture` (target=`raw.capture`). vworld-client 이 re-export 유지
- vworld-client 의존 갱신 — `raw-capture-client` crate dep 추가, 로컬
  `raw_capture.rs` 삭제, error.rs 의 `RawCaptureError` 도 이동
- 마이그 `V003_06` (`migrations/30006_parcel_external_data.sql`) — 단일 테이블:
  `(pnu char(19), source varchar(50), raw_response jsonb, fetched_at timestamptz,
  expires_at timestamptz)`. PK `(pnu, source)`. CHECK source IN
  (`vworld`, `data_go_kr_building`, `data_go_kr_land`, `data_go_kr_tx`, `lawmaking`).
  BRIN idx `parcel_external_data_fetched_brin_idx` (cold storage 친화)
- 신규 `crates/db/src/raw_capture.rs` — `PgRawCapture` impl `RawCapture`,
  UPSERT `(pnu, source)` 갱신 시 raw_response + fetched_at 덮어쓰기
- 통합 테스트 신규 3 (`crates/db/tests/raw_capture_integration.rs`):
  capture_inserts_new_row / capture_upserts_existing_row /
  capture_persists_complex_json
- `tests/migrations/test_v001_full.sh` — EXPECTED_18 → 19, V003_06 검증 추가
  (PK / source CHECK / BRIN idx)
- `truncate_all` 에 `parcel_external_data` 추가
- 누적 테스트: 1198 → ~1201 (통합 +3)

**SSS 7기둥 결함 닫음**:
- 1 일관성: `RawCapture` trait 가 vworld 외 data-go-kr / 법제처 등 후속 source
  까지 동일 인터페이스로 통합
- 3 추적성: raw_response 가 DB 영구 보존 → 외부 API 디버깅 / 재처리 가능

**SP4-iii-d 발견 사항 (lessons)**:
- migration number 충돌: 처음 `30005_parcel_external_data` 로 작성했으나 SP3 의
  `30005_user_roles_check` 와 충돌 → V003_06 으로 rename
- markdownlint MD004/MD007: `memory/project_progress.md` 의 indented `+` 가
  unordered-list 로 해석되어 빨강 → comma-joined text 로 정리

### Sub-project 4-iii-a: data.go.kr 건축물대장 + BuildingReader (완료, T1-T7)

- 신규 lib `crates/data-clients/data-go-kr` (~1200 lines):
  · `DataGoKrConfig` + `from_env` (ODP_SERVICE_KEY/ODP_BASE_URL)
  · `DataGoKrClient` (reqwest + Breaker + `Policy::data_go_kr_default`)
  · `pnu_split` — PNU 19자리 → sigunguCd(5) / bjdongCd(5) / platGbCd(1) /
    bun(4) / ji(4) 무할당 슬라이스 분해 (Pnu invariant 가 panic-free 보장)
  · `building_register::BuildingRegisterClient::fetch_title_info(parts)` —
    `getBrTitleInfo` URL build + `circuit_breaker::execute` 통과
  · `building_register::parser::parse_building_title` — data.go.kr JSON →
    `Vec<Building>` ACL. `mainPurpsCdNm` / `strctCdNm` 한글 라벨 → enum 매핑
    (Other fallback), `totArea` 문자열 → `AreaM2`, `useAprDay` YYYYMMDD →
    `NaiveDate`, `items.item` 단일/배열/빈 문자열 다형 처리,
    `resultCode != "00"` → `ApiError(code, msg)`
  · `building_register::DataGoKrBuildingReader` impl `BuildingReader::fetch_by_pnu`
    — 1) `getBrTitleInfo` raw → 2) `raw_capture(source="data_go_kr_building")` →
    3) V-World `LT_C_UQ111` 폴리곤 fetch (geom 합성, FU 40 까지 approximation) →
    4) parse → `Vec<Building>`. 빈 items 분기는 V-World 호출 회피.
    `fetch_by_id` → FU 42 (mgmBldrgstPk 별도 endpoint)
  · `circuit-breaker::Policy::data_go_kr_default` — 15s timeout, retry 2회
    (1s/2s/4s), threshold 5, window 5s, cooldown 30s. V-World 보다 timeout 길고
    retry 더 (응답 본문 무거움)
- 통합 테스트 6 (`tests/building_register_integration.rs`) — 한 `MockServer` 가
  data.go.kr (`/1613000/...`) + V-World (`/req/data`) path 동시 처리:
  happy_path / multi_buildings / empty_returns_empty_vec /
  5xx_retries_then_fails / malformed_returns_parse_error /
  circuit_opens_after_threshold
- 단위 테스트 25 (parser 13 + pnu_split 4 + client 3 + error 3 + reader 1 +
  br_client 1) + Policy::data_go_kr_default 단위 2
- workspace.members + `crates/data-clients/data-go-kr` 추가
- 누적 테스트: ~1201 → ~1232 (단위 +27 / 통합 +6, +1 crate)

**SSS 7기둥 결함 닫음**:
- 1 일관성: data.go.kr 도 V-World 와 동일 패턴 (Config + Client + Breaker +
  ACL parser + Reader). 후속 외부 API (법제처/실거래가) 도 같은 골격
- 3 추적성: `parcel_external_data.source = "data_go_kr_building"` 행 자동
  INSERT (PgRawCapture 1차 실사용)
- 4 안전성: timeout 15s + retry 2회 + circuit. `Building.geom` 폴리곤 합성은
  명시적 trade-off 로 spec/README 에 기록 (정확한 footprint 는 FU 40)
- 7 명확성: `Building.geom` 합성 (V-World 필지 폴리곤 = 건물 폴리곤 가정) 가
  *추측이 아닌 결정* — spec § 3.3 / 위험 요소 / FU 40 모두 일관 명시

**SP4-iii-a 발견 사항 (lessons)**:
- `Building.geom: PolygonSrid` required → data.go.kr 응답에 폴리곤 0건 →
  V-World 합성 강제. 도메인 변경 (Option<PolygonSrid>) 보다 합성이 SSS-등급
  명확함 (정확한 footprint 가 누락이 아니라 *근사값으로 채워졌음* 이 명시)
- 한글 라벨 → enum 매핑은 `Other` fallback 정책으로 외부 스키마 확장에
  견고. 명시 매핑 외 라벨도 `ParseError` 가 아니라 흡수 (FU 41 매핑표 확장
  여지)
- items.item 다형 (단일 객체 vs 배열 vs 빈 문자열) 처리는 `serde_json::Value`
  match 로 type-safe 분기
- 통합 테스트 한 `MockServer` 가 두 외부 API path 동시 처리 → spec 의
  *분리 또는 path-based dispatch* 중 후자 채택 (간결)
- 빈 items 분기는 `polygon` fetch 회피 — V-World 호출 비용 절약 (parse_building_title
  signature 가 받지만 사용 안 됨, 호출 측이 미리 분기)

**SP4-iii-a 미포함 (후속)**:
- FU 40: `Building.geom` 정확한 footprint — V-World AL_D194 건물 레이어 또는
  R2 PMTiles (SP4-iii-e)
- FU 41: `mainPurpsCdNm` / `strctCdNm` 한글 매핑표 확장 (28+ 케이스)
- FU 42: `fetch_by_id` 구현 — `mgmBldrgstPk` 문자열 키 별도 endpoint
- FU 43: 캐시 정책 (`expires_at = fetched_at + 30 days` per data.go.kr docs)
- FU 44: 토지대장 (`getLandRegInfo` 등) — SP4-iii-a-2 또는 SP4-iii-b 와 묶음
