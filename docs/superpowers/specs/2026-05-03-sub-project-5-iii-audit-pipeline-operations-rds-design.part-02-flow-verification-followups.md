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
