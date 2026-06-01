# Sub-project 2a - Part 01B: V001 Operations, Verification, V002, And Shared-Kernel Bootstrap

Parent index: [Sub-project 2a Part 01](./2026-05-02-sub-project-2a-infra-migrations-shared-kernel.part-01.md).

## Task 8: V001_05 — Operations 6 테이블

**Files:**
- Create: `migrations/V001_05__operations_tables.sql`
- Create: `tests/migrations/test_v001_05.sh`

**스펙 참조:** spec § 5.5 (lines 444–570). 6개 테이블: `admin_action`, `business_verification_queue`, `listing_review_queue`, `listing_report`, `featured_content`, `system_alert`.

핵심 패턴 (spec 인용):
- `admin_action` — 어드민이 한 모든 행위 기록 (audit_log와 별개 — 운영 컨텍스트)
- `business_verification_queue` — 사업자 등록 대기 큐 (NICE/공정위 검증 후 status 갱신)
- `listing_review_queue` — 매물 사전 심사 큐 (assigned_to FK)
- `listing_report` — 사용자 신고 (reason_code enum + status 4종)
- `featured_content` — 추천 매물/산단/지역 (시작/종료 시각 + check (ends_at > starts_at))
- `system_alert` — 운영 알림 (severity 3종, resolved_at IS NULL 부분 인덱스)

> **참고:** `featured_content.content_type` enum에 `'industrial_complex'`이 포함되지만, *string 값*일 뿐 FK 아님 (IndustrialComplex는 R2 정적). target_id는 R2 식별자.

- [ ] **Step 1-2:** 테스트 작성 + 실패 확인 (EXPECTED 6개)

- [ ] **Step 3:** spec § 5.5 의 SQL 그대로 복사. 파일 머리 주석:

```sql
-- V001_05: Admin/Operations — admin_action, queues, reports, featured, alerts (spec § 5.5)
```

- [ ] **Step 4: 통과** — `PASS: V001_05 Operations 6 tables`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_05 — Operations 6 tables (admin actions, queues, reports, featured, alerts) per spec § 5.5"
```

---

## Task 9: V001 통합 검증 + ER 다이어그램

**Files:**
- Create: `tests/migrations/test_v001_full.sh` (18 테이블 검증)
- Create: `docs/database/er-diagram-v001.md` (Mermaid ERD)

**스펙 참조:** spec § 5.6 (line 571) — 합계 18 테이블 명시.

- [ ] **Step 1: 통합 테스트 작성**

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$SCRIPT_DIR"
if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a; source <(tr -d '\r' < .env); set +a
fi
sqlx database drop -y >/dev/null 2>&1 || true
sqlx database create
sqlx migrate run --source migrations

EXPECTED_18=( "user" listing listing_photo \
  bookmark_listing bookmark_external search_history analysis_report notification \
  audit_log outbox_event \
  pipeline_schedule pipeline_run \
  admin_action business_verification_queue listing_review_queue listing_report featured_content system_alert )

for t in "${EXPECTED_18[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: missing $t" >&2; exit 1
  fi
done

# 추가 검증: 정확히 18 테이블 (sqlx 시스템 테이블 제외)
COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_tables where schemaname='public' and tablename not like '\\_sqlx%';")
if [ "$COUNT" != "18" ]; then
  echo "FAIL: expected exactly 18 public tables (excluding _sqlx_*), got $COUNT" >&2
  exit 1
fi

echo "PASS: V001 18 tables (spec § 5.6)"
```

- [ ] **Step 2: ER 다이어그램** — `docs/database/er-diagram-v001.md` (Mermaid `erDiagram`, ≤300줄). 18 RDS 테이블만. R2 정적 (Parcel/Building/IndustrialComplex/Manufacturer/RealTransaction/CourtAuction/Law)은 *별도 점선 박스*로 표시 + "stored in R2 (see § 4)" 주석.

- [ ] **Step 3: 통과** — `bash tests/migrations/test_v001_full.sh` (또는 정적 검증)

- [ ] **Step 4: Commit**

```bash
git add tests/migrations/test_v001_full.sh docs/database/er-diagram-v001.md
git commit -m "test(db): V001 full validation — exactly 18 RDS tables per spec § 5.6 + ER diagram"
```

---

## Task 10: V002 — DB role 3개 + audit immutable 트리거

**Files:**
- Create: `migrations/V002_01__db_roles.sql`
- Create: `migrations/V002_02__audit_immutable_trigger.sql`
- Create: `tests/migrations/test_v002_audit_immutable.sh`

**스펙 참조:** spec § 7

- [ ] **Step 1: 트리거 테스트 작성**

```bash
#!/usr/bin/env bash
set -euo pipefail
source .env
sqlx database drop -y && sqlx database create
sqlx migrate run --source migrations
psql "$DATABASE_URL" -c "set role gongzzang_app_writer; \
  insert into audit_log(id, action, resource_type, resource_id) values('aud_test', 't', 'r', 'r1');"
# UPDATE 시도 → 실패 기대
if psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "set role gongzzang_app_writer; \
  update audit_log set action='x' where id='aud_test';" 2>&1 | grep -q "audit_log is immutable"; then
  echo "PASS: audit immutable enforced"; exit 0
fi
echo "FAIL: writer was able to UPDATE audit_log"; exit 1
```

- [ ] **Step 2: 실행 — 실패 (V002 없음)**

- [ ] **Step 3: V002_01 — role 정의**

```sql
do $$ begin
  if not exists (select 1 from pg_roles where rolname='gongzzang_app_writer') then
    create role gongzzang_app_writer login password 'changeme_writer';
  end if;
  if not exists (select 1 from pg_roles where rolname='gongzzang_app_reader') then
    create role gongzzang_app_reader login password 'changeme_reader';
  end if;
  if not exists (select 1 from pg_roles where rolname='gongzzang_audit_archiver') then
    create role gongzzang_audit_archiver login password 'changeme_archiver';
  end if;
end $$;

grant select, insert, update, delete on all tables in schema public to gongzzang_app_writer;
grant usage, select on all sequences in schema public to gongzzang_app_writer;
revoke update, delete on audit_log from gongzzang_app_writer;

grant select on all tables in schema public to gongzzang_app_reader;

grant select, delete on audit_log to gongzzang_audit_archiver;
revoke insert, update on audit_log from gongzzang_audit_archiver;
```

- [ ] **Step 4: V002_02 — immutable 트리거**

```sql
create or replace function reject_audit_mutation() returns trigger language plpgsql as $$
begin
    raise exception 'audit_log is immutable: % not allowed (use audit_archiver role to DELETE after retention)', tg_op;
end $$;

create trigger trg_audit_no_update before update on audit_log
    for each row when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();

create trigger trg_audit_no_delete before delete on audit_log
    for each row when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();
```

- [ ] **Step 5: 통과 확인**

- [ ] **Step 6: Commit**

```bash
git add migrations/V002_*.sql tests/migrations/test_v002_audit_immutable.sh
git commit -m "feat(db): V002 — 3 DB roles + audit_log immutable trigger (writer cannot UPDATE/DELETE)"
```

---

## Task 11: shared-kernel crate 부트스트랩 + `_placeholder` 제거

**Files:**
- Modify: `Cargo.toml` (workspace members)
- Delete: `crates/_placeholder/`
- Create: `crates/shared-kernel/Cargo.toml`
- Create: `crates/shared-kernel/src/lib.rs`
- Create: `crates/shared-kernel/README.md`

- [ ] **Step 1: workspace Cargo.toml 갱신**

```toml
[workspace]
resolver = "2"
members = ["crates/shared-kernel"]

[workspace.dependencies]
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls","postgres","macros","uuid","chrono","json","migrate"] }
ulid = "1.1"
chrono = { version = "0.4", default-features = false, features = ["std","clock","serde"] }
serde = { version = "1", features = ["derive"] }
thiserror = "1"
geo-types = "0.7"
once_cell = "1"
regex = "1"
```

- [ ] **Step 2: `crates/_placeholder/` 삭제**

- [ ] **Step 3: shared-kernel/Cargo.toml**

```toml
[package]
name = "shared-kernel"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 도메인 공유 값 객체 — Pnu, Money, Area 등 BC 간 공통 타입."

[dependencies]
ulid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
geo-types = { workspace = true }
once_cell = { workspace = true }
regex = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 4: lib.rs**

```rust
//! Shared kernel — 모든 BC가 의존하는 값 객체.
//!
//! 도메인 간 통용 어휘. *Aggregate*는 각 BC crate에 위치.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin_division;
pub mod area;
pub mod broker_license;
pub mod business_number;
pub mod email;
pub mod geometry;
pub mod id;
pub mod jibun_address;
pub mod ksic_code;
pub mod money;
pub mod phone_kr;
pub mod pnu;
pub mod road_address;
pub mod srid;
pub mod time;
```

> **참고:** lib.rs는 *각 모듈 추가 시점*에 점진 갱신. 처음에는 `id`/`time` 모듈만 선언하고, 이후 task마다 한 줄씩 추가.

- [ ] **Step 5: 검증** — `cargo check -p shared-kernel`

```bash
cargo check -p shared-kernel
```
Expected: 모듈 미존재로 에러 → 처음에는 `pub mod id;`만 두고, Task 12에서 `id.rs` 작성.

- [ ] **Step 6: 초기 커밋** — 빈 모듈 선언만

```rust
// lib.rs (Task 11 시점)
#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! shared-kernel — 값 객체는 후속 task에서 점진 추가.
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/shared-kernel/
git rm -r crates/_placeholder/
git commit -m "feat(shared-kernel): bootstrap crate, remove _placeholder"
```

---
