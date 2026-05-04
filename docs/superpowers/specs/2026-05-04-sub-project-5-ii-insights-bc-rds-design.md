# Sub-project 5-ii: Insights BC RDS Repository (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP5-i (Core BC RDS), SP5-iii (Audit + Pipeline + Operations + 트랜잭션 Outbox), SP5-iv (Core BC `MutationContext` 일원화), SP4-i (Outbox publisher) |
| 후속 | SP4-ii (V-World 외부 API), SP6 (Frontend) |
| 관련 ADR | — |

---

## 1. 개요

Insights BC 4 도메인 (Bookmark / SearchHistory / AnalysisReport / Notification) 의 `Postgres` 저장소 구현 — SP5-iii/iv 의 **transactional `audit_log` + `outbox_event` 패턴** 답습. 새 기술 0, 패턴 검증된 답습.

본 sub-project 가 닫는 것:
- **SP5 시리즈 완전 종료**: 13 BC 모두 동일 transactional `save(agg, ctx)` / `insert(agg, ctx)` 패턴
- **SSS 일관성**: Insights BC 의 모든 mutation 도 audit_log 자동 기록
- **준비**: SP6 frontend 가 Bookmark/Notification 핸들러 구현 시 즉시 사용 가능

---

## 2. 범위

### 포함

- **4 도메인 trait 시그니처 변경** — mutation 메서드에 `ctx: MutationContext` 추가:
  - `BookmarkRepository`: `save_listing_bookmark`, `save_external_bookmark`, `delete_listing_bookmark`, `delete_external_bookmark`
  - `SearchHistoryRepository`: `insert`, `pseudonymize_older_than`
  - `AnalysisReportRepository`: `save`, `delete`
  - `NotificationRepository`: `insert`, `mark_read`, `mark_all_read_by_kind`
- **4 PgRepository 신규** in `crates/db/src/`:
  - `bookmark.rs` — `PgBookmarkRepository` (composite PK + polymorphic external)
  - `search_history.rs` — `PgSearchHistoryRepository` (append + pseudonymize)
  - `analysis_report.rs` — `PgAnalysisReportRepository` (OCC + R2 snapshot JSONB)
  - `notification.rs` — `PgNotificationRepository` (mark_read 멱등 + bulk mark_all_read_by_kind)
- **`error_map.rs` 4 신규 `MapFromSqlx` impls** (Bookmark/SearchHistory/AnalysisReport/Notification `RepoError`)
- **`crates/db/Cargo.toml` deps 추가** — 4 도메인 crate
- **통합 테스트 4 파일 ~16-20 tests** (`crates/db/tests/`):
  - `bookmark_integration.rs` — 6+ tests (composite PK, external, delete audit)
  - `search_history_integration.rs` — 4-5 tests (insert audit, anonymous user_id, pseudonymize bulk audit)
  - `analysis_report_integration.rs` — 5-6 tests (round-trip + OCC + delete + version bump)
  - `notification_integration.rs` — 4-5 tests (insert/mark_read/bulk mark_all_read_by_kind)

### 미포함

- **AnalysisReport `target_pnus` 배열 round-trip 최적화** — `char(19)[]` ↔ `Vec<Pnu>` 변환만 검증, 인덱스 (GIN 등) 도입은 후속
- **SearchHistory 검색 패턴 분석** — `query` 텍스트 분석 / NLP — 별도 sub-project (Phase 3+ 임베딩)
- **Notification push delivery** — 알림 row INSERT 만, 외부 발송 (FCM / APNS / WebPush) 은 SP4-iii 의 Outbox sink 도입 후
- **Bookmark count cache** — `listing.bookmark_count` 갱신은 SP5-i 의 listing trigger 또는 별도 outbox consumer
- **AnalysisReport snapshot 갱신 워커** — 재분석 자동화 (cron) — SP9+
- **Audit metadata 표준화** — bookmark/notification 의 audit_log.metadata 형식 — 별도 컨벤션 문서

---

## 3. 도메인-DB 맵핑

### 3.1 BookmarkListing (composite PK, no version)
```
DB: (user_id, listing_id) PK, note text, created_at
Domain: BookmarkListing { user_id, listing_id, note, created_at }
```
- `save_listing_bookmark`: `INSERT ... ON CONFLICT (user_id, listing_id) DO UPDATE SET note=EXCLUDED.note` — UPSERT (idempotent for re-bookmark)
- `delete_listing_bookmark(user_id, listing_id, ctx)`: hard delete by composite key. `rows_affected == 0` → `NotFound`

### 3.2 BookmarkExternal (id PK, polymorphic, no version)
```
DB: id char(30), user_id, target_kind varchar(30) CHECK in 4 values, target_id varchar(50),
    note text, UNIQUE(user_id, target_kind, target_id)
Domain: BookmarkExternal { id, user_id, target_kind: BookmarkExternalKind, target_id, note, created_at }
```
- `save_external_bookmark`: INSERT or UPDATE by id
- `delete_external_bookmark(id, ctx)`: hard delete by id

### 3.3 SearchHistory (append-mostly, nullable user_id)
```
DB: id char(30), user_id char(30) NULL, query text, filters jsonb, result_count int,
    correlation_id varchar(30), created_at
Domain: SearchHistory { id, user_id: Option<UserId>, query, filters, result_count, correlation_id, created_at }
```
- `insert(history, ctx)`: 단일 INSERT
- `pseudonymize_older_than(cutoff, ctx)`: `UPDATE search_history SET user_id = NULL WHERE created_at < $1 AND user_id IS NOT NULL` returning rows_affected
  - audit_log 단일 row, `resource_id = format!("cutoff_{}", cutoff.timestamp())`, metadata = `{"cutoff_iso", "rows_pseudonymized"}`

### 3.4 AnalysisReport (OCC + JSONB snapshot)
```
DB: id, user_id, title varchar(200), target_pnus char(19)[], snapshot jsonb,
    created_at, updated_at, version
Domain: AnalysisReport { id, user_id, title, target_pnus: Vec<Pnu>, snapshot, created_at, updated_at, version }
```
- `save`: UPSERT with `WHERE version = $N`. SP5-iv 패턴 (rows_affected == 0 → Conflict)
- `delete(id, ctx)`: hard delete by id
- `target_pnus` round-trip: `Vec<Pnu>` → `Vec<String>` (각 19자) → `bind` as `&[String]` (sqlx 가 `text[]` 호환)

### 3.5 Notification (append-mostly + idempotent mark_read)
```
DB: id, user_id, kind varchar(50), payload jsonb, read_at timestamptz NULL, created_at
Domain: Notification { id, user_id, kind, payload, read_at: Option<DateTime>, created_at }
```
- `insert`: 단일 INSERT
- `mark_read(id, at, ctx)`: `UPDATE notification SET read_at = $2 WHERE id = $1 AND read_at IS NULL`
  - 멱등: 이미 읽음 = `rows_affected == 0` 이지만 NotFound 반환 X (도메인 의미: 멱등이라 OK)
  - 결정: `rows_affected == 0 && row exists` → 이미 읽음 = `Ok(())` 반환. row 미존재 → `NotFound`
  - 실용: `UPDATE ... RETURNING 1` 으로 row 존재 확인 또는 별도 `find_by_id` 추가 호출. 우선 단순화: `rows_affected == 0` → `Ok(())` (멱등). row 미존재 검증은 caller 의 `find_by_id` 책임. 이 trade-off 명시.
- `mark_all_read_by_kind(user_id, kind, at, ctx)`: bulk UPDATE returning count

---

## 4. PgRepository 패턴 (SP5-iv 답습)

모든 mutation 메서드는 다음 5단계:
1. `let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;`
2. Aggregate INSERT/UPDATE/UPSERT/DELETE on `&mut *tx`
3. `audit_log` INSERT (`resource_kind`, `resource_id`, ctx 매핑)
4. `for event in &ctx.events`: `outbox_event` INSERT (`aggregate_kind`)
5. `tx.commit()` — 실패 시 자동 rollback

**ListingPhoto delete 패턴** 답습 (SP5-iv `write_audit_log` + `write_outbox_events` 헬퍼 추출). 단, 4개 repo 가 각자 다른 `resource_kind` 라 헬퍼 추출은 file-level (per-repo) 또는 module-level (`crates/db/src/audit_helpers.rs`).

**결정**: 각 repo file 안에 inline `write_audit_log` / `write_outbox_events` (resource_kind/aggregate_kind 가 다르므로 함수 내부에 hard-coded). DRY 보다 명시성 우선.

---

## 5. 에러 매핑

4 도메인 `RepoError`:
- Bookmark: `NotFound | Database` (no Conflict — 모든 save 가 UPSERT)
- SearchHistory: `NotFound | Database` (insert-only)
- AnalysisReport: `NotFound | Conflict | Database` (OCC)
- Notification: `NotFound | Database` (mark_read 가 멱등)

`error_map.rs` 4 신규 `MapFromSqlx` impl. SP5-iii 패턴 답습.

---

## 6. 가시성

모든 mutation 메서드 `#[tracing::instrument(skip(self, agg, ctx), fields(<id>, ctx_action, correlation_id, events_count))]`. PII 미노출 — `payload`/`snapshot`/`note`/`query` 등은 skip.

---

## 7. 통합 테스트 전략

각 PgRepo 통합 테스트는 SP5-iv 의 user_integration / listing_integration 답습:
1. `<repo>_round_trip` — 모든 필드 보존 검증
2. `save/insert_inserts_audit_log_in_one_tx` — audit_log row count = 1, resource_kind / resource_id 검증
3. `save/insert_with_events_inserts_outbox_per_event` — events 2개 → outbox 2 row
4. `save/insert_system_action_records_null_actor` — actor_id NULL
5. (해당 repo 한정) OCC / mark_read / pseudonymize 등 동작 검증

총 ~16-20 신규 tests.

---

## 8. CI 통합

`walking-skeleton.yml` 의 `cargo test --features integration` 자동 실행. CI workflow 변경 0.

---

## 9. 검증 기준 (DoD)

1. 4 도메인 trait 시그니처에 `MutationContext` 추가
2. 4 PgRepository 신규
3. `error_map.rs` 4 신규 `MapFromSqlx` impl
4. `crates/db/Cargo.toml` 4 도메인 crate dep 추가
5. 통합 테스트 ≥16 신규
6. 3 CI workflow 그린
7. 누적 테스트 ≥1158 (SP4-i 종료 ~1142 + 16)
8. clippy `-D warnings` 통과 (`--all-features`)
9. tarpaulin ≥90% 유지
10. 모든 파일 ≤500 권장 / ≤1500 강제
11. SSOT 갱신: roadmap.md / project_progress.md / MEMORY.md

---

## 10. SSS 7 기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 13 BC 모두 동일 `save(agg, ctx)` 패턴. SP5 시리즈 종료 |
| 2 자동 강제 | tx atomic — audit 실패 = 전체 실패 |
| 3 추적성 | Insights BC mutation 도 audit_log 자동 기록 |
| 4 안전성 | OCC (AnalysisReport), 멱등 (Notification mark_read), parameterized SQL only |
| 5 가시성 | 모든 메서드 `tracing::instrument` |
| 6 SSOT | DB schema = SSOT. Repository trait 시그니처 단일화 |
| 7 명확성 | resource_kind 도메인 의미 보존 (`bookmark_listing` / `bookmark_external` / `search_history` / `analysis_report` / `notification`) |

---

## 11. Follow-up

- **FU 21**: Bookmark count denormalization (`listing.bookmark_count` 동기) — outbox consumer 또는 trigger
- **FU 22**: AnalysisReport `target_pnus` 인덱스 (GIN) — 사용자 리포트 통계 쿼리 시
- **FU 23**: Notification push delivery (FCM/APNS/WebPush) — Outbox sink 추가
- **FU 24**: SearchHistory NLP / 임베딩 (Phase 3+)
- **FU 25**: 365일 알림 retention 워커 (`services/worker/notification_retention`)
