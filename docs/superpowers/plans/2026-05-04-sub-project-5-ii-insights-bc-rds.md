# Sub-project 5-ii: Insights BC RDS — 구현 계획

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 spec | [`2026-05-04-sub-project-5-ii-insights-bc-rds-design.md`](../specs/2026-05-04-sub-project-5-ii-insights-bc-rds-design.md) |
| 추정 | 9 task (T1..T9), 1-2일 |

---

## 작업 흐름 원칙

1. **순서**: 4 trait 시그니처 → 4 PgImpl → 4 통합 테스트 → 검증 + 푸시 → SSOT
2. **SP4-i 교훈 적용**:
   - 각 PgImpl 작성 시 SP5-iv `user.rs` / `listing.rs` 패턴 그대로 답습 — 새 결정 0
   - **clippy::ignored_unit_patterns** + `clippy::module_name_repetitions` 미리 차단
   - file-level allow: `#![allow(clippy::module_name_repetitions)]`
3. **각 task = 1 commit**

---

## T1 — spec + plan 커밋 (이미 작성됨)

**commit**: `docs(sp5-ii): spec + plan — Insights BC RDS Repository`

---

## T2 — 4 도메인 trait 시그니처 변경 + error_map MapFromSqlx

**대상**:
- `crates/domain/insights/bookmark/src/repository.rs` — 4 mutation methods + ctx
- `crates/domain/insights/search-history/src/repository.rs` — `insert(&self, history, ctx)` + `pseudonymize_older_than(&self, cutoff, ctx)`
- `crates/domain/insights/analysis-report/src/repository.rs` — `save(report, ctx)` + `delete(id, ctx)`
- `crates/domain/insights/notification/src/repository.rs` — `insert/mark_read/mark_all_read_by_kind` + ctx
- `crates/db/src/error_map.rs` — 4 신규 `impl MapFromSqlx for <crate>_domain::repository::RepoError`

**Cargo.toml deps**:
- 각 도메인 crate 의 `Cargo.toml` 에 `shared-kernel` 이미 있는지 확인 (4 도메인 모두 이미 있음 — verify only)
- `crates/db/Cargo.toml` `[dependencies]` 에 4 도메인 추가:
  - `bookmark-domain = { path = "../domain/insights/bookmark", version = "0.1.0" }`
  - `search-history-domain = { path = "../domain/insights/search-history", version = "0.1.0" }`
  - `analysis-report-domain = { path = "../domain/insights/analysis-report", version = "0.1.0" }`
  - `notification-domain = { path = "../domain/insights/notification", version = "0.1.0" }`

**검증**: 로컬 `cargo fmt --all --check` + `cargo metadata --no-deps`

**commit**: `feat(sp5-ii-t2): 4 Insights BC trait signatures accept MutationContext + error_map`

---

## T3 — `PgBookmarkRepository`

**대상**: `crates/db/src/bookmark.rs` (신규) + `crates/db/src/lib.rs` (`pub mod bookmark;`)

**구조**:
- `find_listing_bookmarks(&user_id)` — read-only
- `find_external_bookmarks(&user_id)` — read-only
- `save_listing_bookmark(&bm, ctx)` — UPSERT (composite PK ON CONFLICT) + audit + outbox
- `save_external_bookmark(&bm, ctx)` — UPSERT (id PK ON CONFLICT) + audit + outbox
- `delete_listing_bookmark(&user_id, &listing_id, ctx)` — DELETE composite + audit + outbox
- `delete_external_bookmark(&id, ctx)` — DELETE by id + audit + outbox

**audit_log resource_kind**:
- listing bookmark: `'bookmark_listing'`, resource_id = `listing_id` (varchar(50) 안 30자)
- external bookmark: `'bookmark_external'`, resource_id = `bm.id` 또는 `id` 인자

**aggregate_kind for outbox**: 동일 (`'bookmark_listing'` / `'bookmark_external'`)

**SQL**:
- listing bookmark: `INSERT ... ON CONFLICT (user_id, listing_id) DO UPDATE SET note = EXCLUDED.note`
- external bookmark: `INSERT ... ON CONFLICT (id) DO UPDATE SET note = EXCLUDED.note, target_kind = EXCLUDED.target_kind, target_id = EXCLUDED.target_id`

**commit**: `feat(sp5-ii-t3): PgBookmarkRepository — composite PK + polymorphic external + audit/outbox`

---

## T4 — `PgSearchHistoryRepository`

**대상**: `crates/db/src/search_history.rs` + lib.rs 갱신

**구조**:
- `find_recent_by_user(&user_id, limit)` — `WHERE user_id = $1 AND created_at > NOW() - INTERVAL '90 days' ORDER BY created_at DESC LIMIT $2`
- `insert(&history, ctx)` — INSERT + audit + outbox
- `pseudonymize_older_than(cutoff, ctx)` — bulk UPDATE + 단일 audit row (`resource_id = format!("cutoff_{}", cutoff.timestamp())`)

**audit_log resource_kind**: `'search_history'`

**user_id round-trip**: `Option<Id<UserMarker>>` ↔ `Option<&str>` (NULL allowed)

**commit**: `feat(sp5-ii-t4): PgSearchHistoryRepository — append + bulk pseudonymize + audit`

---

## T5 — `PgAnalysisReportRepository`

**대상**: `crates/db/src/analysis_report.rs` + lib.rs 갱신

**구조**:
- `find_by_id(&id)` — read-only
- `find_by_user(&user_id, limit)` — read-only
- `save(report, ctx)` — UPSERT with OCC (`WHERE version = $N`)
- `delete(&id, ctx)` — DELETE + audit + outbox (NotFound 시 rollback)

**target_pnus 처리**:
- write: `Vec<Pnu>` → `Vec<&str>` → `bind` 으로 `text[]` 호환
- read: `Vec<String>` → 각 `Pnu::try_new` 으로 round-trip

**audit_log resource_kind**: `'analysis_report'`

**commit**: `feat(sp5-ii-t5): PgAnalysisReportRepository — OCC + R2 snapshot JSONB + target_pnus[]`

---

## T6 — `PgNotificationRepository`

**대상**: `crates/db/src/notification.rs` + lib.rs 갱신

**구조**:
- `find_unread_by_user(&user_id)` — `WHERE user_id = $1 AND read_at IS NULL ORDER BY created_at DESC`
- `find_recent_by_user(&user_id, limit)` — `WHERE user_id = $1 AND created_at > NOW() - INTERVAL '365 days' ORDER BY created_at DESC LIMIT $2`
- `insert(notification, ctx)` — INSERT + audit + outbox
- `mark_read(&id, at, ctx)` — `UPDATE notification SET read_at = $2 WHERE id = $1 AND read_at IS NULL` + audit + outbox
  - 멱등: `rows_affected == 0` → `Ok(())` (이미 읽음 또는 row 미존재 모두 OK)
- `mark_all_read_by_kind(&user_id, kind, at, ctx) -> u64` — bulk + 단일 audit (rows_affected count)

**audit_log resource_kind**: `'notification'`

**commit**: `feat(sp5-ii-t6): PgNotificationRepository — insert + idempotent mark_read + bulk by_kind`

---

## T7 — 4 통합 테스트 파일

**대상** (각 ~5 tests):
- `crates/db/tests/bookmark_integration.rs` — round-trip (listing + external) + composite PK delete + audit row + outbox events
- `crates/db/tests/search_history_integration.rs` — anonymous insert + pseudonymize bulk + audit
- `crates/db/tests/analysis_report_integration.rs` — round-trip + OCC + target_pnus[] + delete
- `crates/db/tests/notification_integration.rs` — insert + mark_read 멱등 + mark_all_read_by_kind bulk

**시드 헬퍼 재사용**: `common::test_ctx()` (system action ctx, SP5-iv 도입). 각 시나리오마다 owner/admin user 시드 (PgUserRepository 통해 — `test_ctx()` 그대로).

**`truncate_all` 갱신** — `crates/db/tests/common.rs`: 4 신규 테이블 추가:
```sql
truncate "user", listing, listing_photo, audit_log, outbox_event,
    admin_action, business_verification_queue, listing_review_queue,
    listing_report, featured_content, system_alert, pipeline_run,
    pipeline_schedule,
    bookmark_listing, bookmark_external, search_history, analysis_report,
    notification cascade
```

**commit**: `feat(sp5-ii-t7): 4 integration tests for Insights BC + truncate_all extended`

---

## T8 — 종합 검증 + push

**로컬**:
- `cargo fmt --all --check` ✓
- `cargo metadata --no-deps` ✓
- `cargo +1.88.0-x86_64-pc-windows-gnu clippy -p bookmark-domain -p search-history-domain -p analysis-report-domain -p notification-domain --all-features --all-targets -- -D warnings` (도메인 crate 만 — proc-macro deps 만 필요, link 가능)

**push**: `git push origin main`

**CI 모니터링**: 3 workflow 그린 확인 (CI / db-migrations / walking-skeleton).

**실패 시**: clippy 에러는 SP4-i 와 동일 패턴으로 fix commit + 재푸시.

**commit (필요 시)**: `fix(sp5-ii): clippy <issue>`

---

## T9 — SSOT 갱신

**대상**:
- `docs/superpowers/roadmap.md`:
  - 완료 표에 SP5-ii 행 추가 (✅, "Insights BC RDS Repository", "4 PgRepository — Bookmark/SearchHistory/AnalysisReport/Notification + 16 통합 테스트")
  - "SP5 시리즈 완전 종료" 표기
  - 다음 SP 후보 갱신 (SP4-ii 추천)
- `memory/project_progress.md`:
  - 새 섹션 `### Sub-project 5-ii: Insights BC RDS (완료, T1-T9)`
  - 누적 카운트 ~1158 tests, 31 crate? — 실제 crate 수 변동 없음 (insights crate 들은 이미 존재). 새 PgImpl 4 모듈만 db crate 안에 추가
- `MEMORY.md` 한 줄 갱신

**commit**: `docs(sp5-ii-t9): SP5-ii 종료 — Insights BC RDS, SP5 시리즈 완전 종료`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| domain trait | 4 파일 | mutation 시그니처 + ctx |
| Pg impl | 4 신규 in `crates/db/src/` | 4 PgRepository |
| db lib | `crates/db/src/lib.rs` | 4 `pub mod` 추가 |
| db deps | `crates/db/Cargo.toml` | 4 dep 추가 |
| error_map | `crates/db/src/error_map.rs` | 4 `MapFromSqlx` impl |
| test common | `crates/db/tests/common.rs` | truncate_all 확장 |
| tests | 4 신규 in `crates/db/tests/` | ~16-20 tests |
| docs | spec + plan | 신규 |
| docs | roadmap.md | SP5-ii 종료 표기 |
| memory | project_progress.md | 새 섹션 |
| memory | MEMORY.md | 한 줄 |

총 ~17 파일.

---

## 위험 요소

- **`target_pnus char(19)[]` round-trip**: sqlx 의 `Vec<String>` 바인딩 호환 검증 필요. text[] 와 char(19)[] 의 sqlx 매핑 차이 확인.
- **`bookmark_listing` composite PK**: ON CONFLICT 절에 두 컬럼 명시 필요. 단순 패턴.
- **`mark_read` 멱등 정의**: 결정대로 `rows_affected == 0` → `Ok(())`. caller 가 row 존재 검증 필요 시 별도 `find_by_id`. 트레이드오프 spec 에 명시.
- **`pseudonymize_older_than` 의 audit row 단일성**: bulk operation 의 단일 audit 행 패턴 — 후속 enrichment (rows_affected metadata) 필수.
- **`insights` 4 crate 가 모두 `serde_json` 필요**: 이미 deps. 변경 없음.
- **CI clippy 가 SP4-i 와 동일 패턴 (ignored_unit_patterns 등) 가능**: 미리 file-level allow 추가.
