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
