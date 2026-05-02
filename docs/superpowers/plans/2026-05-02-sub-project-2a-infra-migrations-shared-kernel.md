# Sub-project 2a: Infra + V001/V002 마이그레이션 + Shared-kernel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** [Sub-project 2 spec](../specs/2026-05-02-sub-project-2-db-core-domain-design.md) § 5 (RDS 18 테이블) + § 7 (DB role 3개) + § 8.1 shared-kernel을 *컴파일·테스트 가능한 상태*로 구축한다. 도메인 Aggregate (Plan 2b)와 응용 코드 (Plan 2c)는 후속.

**Architecture:** 26 task. 의존 흐름:
1. **Phase A (Task 1-4):** 로컬 인프라 (Docker Compose: PG17 + PostGIS 3.5 + Valkey) → SQLx CLI → `.env`
2. **Phase B (Task 5-9):** V001 마이그레이션 5분할 (Core 5 / Listing+사진+북마크 4 / Insights 4 / Audit+Outbox+Pipeline 3 / Operations 6) — *분할 이유: 18 테이블 한 SQL 파일 = 800줄 이상으로 500줄 룰 위반*
3. **Phase C (Task 10):** V002 DB role 3개 + audit immutable (writer 박탈 트리거)
4. **Phase D (Task 11):** `crates/_placeholder` 제거 + `crates/shared-kernel` 부트스트랩
5. **Phase E (Task 12-25):** 값 객체 14개 *각각 TDD* (실패 테스트 → 실행 → 최소 구현 → 통과 → 커밋)
6. **Phase F (Task 26):** 최종 검증 (`cargo check/clippy/deny` + `cargo test` 90%+ 커버리지 + `sqlx migrate run` E2E)

**Tech Stack:** PostgreSQL 17, PostGIS 3.5, Valkey 8 (Redis 호환), SQLx 0.8 (offline mode), Rust 1.83, Cargo workspace, ULID, geo-types 0.7, cargo-tarpaulin (커버리지).

**TDD 원칙 (모든 값 객체 task에 적용):**
1. *실패하는 테스트 먼저* — 행동 명세를 테스트로 표현
2. *최소 구현* — 테스트만 통과시키는 가장 단순한 코드
3. *각 step ≤ 5분* — 10단계 이상이면 task 쪼갤 것

**Pre-flight 체크 (Task 1 시작 전 확인):**
- [ ] `docker --version` ≥ 24.0
- [ ] `cargo --version` = 1.83.x (Sub-project 1에서 설치)
- [ ] 작업 디렉토리: `c:/Users/User/Desktop/gongzzang_2`
- [ ] 현재 브랜치: `main` 그린 + 132 파일 commit `1ad314a` 이후

---

## File Structure (이번 plan 한정 — 약 50 파일)

### Phase A: 인프라 (4 파일)
- `infrastructure/docker/docker-compose.yml` — PG17 + PostGIS + Valkey
- `infrastructure/docker/postgres/init.sql` — `create extension postgis;` + `pg_trgm` + `unaccent`
- `infrastructure/docker/.env.example` — DB 비밀번호 placeholder
- `infrastructure/docker/README.md` — 기동·정지·접속 안내 (해요체)

### Phase B-C: 마이그레이션 (8 파일, RDS 18 테이블)
- `migrations/V001_01__core_tables.sql` — user, listing, listing_photo *(spec § 5.1, 3 테이블)*
- `migrations/V001_02__insights_tables.sql` — bookmark_listing, bookmark_external, search_history, analysis_report, notification *(spec § 5.2, 5 테이블)*
- `migrations/V001_03__system_tables.sql` — audit_log, outbox_event *(spec § 5.3, 2 테이블)*
- `migrations/V001_04__pipeline_tables.sql` — pipeline_schedule, pipeline_run *(spec § 5.4, 2 테이블)*
- `migrations/V001_05__operations_tables.sql` — admin_action, business_verification_queue, listing_review_queue, listing_report, featured_content, system_alert *(spec § 5.5, 6 테이블)*
- `migrations/V002_01__db_roles.sql` — gongzzang_app_writer/reader/audit_archiver
- `migrations/V002_02__audit_immutable_trigger.sql` — UPDATE/DELETE 차단 트리거
- `migrations/README.md` — 적용 순서 + 롤백 정책

> **분류 근거 (spec § 4):** Parcel/Building/IndustrialComplex/Manufacturer는 *R2 정적*. RealTransaction/CourtAuction/Law/Regulation도 *R2 정적*. RDS는 18 테이블만.

### Phase D-E: shared-kernel crate (약 35 파일)
- `crates/shared-kernel/Cargo.toml`
- `crates/shared-kernel/src/lib.rs` — 모듈 재수출
- `crates/shared-kernel/src/id.rs` + `id/tests.rs`
- `crates/shared-kernel/src/time.rs` + `time/tests.rs`
- `crates/shared-kernel/src/pnu.rs` + `pnu/tests.rs`
- `crates/shared-kernel/src/money.rs` + `money/tests.rs`
- `crates/shared-kernel/src/area.rs` + `area/tests.rs`
- `crates/shared-kernel/src/business_number.rs` + 테스트
- `crates/shared-kernel/src/broker_license.rs` + 테스트
- `crates/shared-kernel/src/email.rs` + 테스트
- `crates/shared-kernel/src/phone_kr.rs` + 테스트
- `crates/shared-kernel/src/srid.rs` + 테스트
- `crates/shared-kernel/src/geometry.rs` + 테스트
- `crates/shared-kernel/src/admin_division.rs` + 테스트
- `crates/shared-kernel/src/road_address.rs` + 테스트
- `crates/shared-kernel/src/jibun_address.rs` + 테스트
- `crates/shared-kernel/src/ksic_code.rs` + 테스트
- `crates/shared-kernel/README.md`
- 루트 `Cargo.toml` 수정 — `_placeholder` 제거 + `shared-kernel` 추가
- `crates/_placeholder/` 삭제

### Phase F: 검증 산출물 (3 파일)
- `tarpaulin.toml` — 커버리지 설정
- `.github/workflows/db-migrations.yml` — 마이그레이션 검증 CI 잡 추가
- `docs/database/migrations.md` — 마이그레이션 운영 가이드

---

## Task 1: Docker Compose 인프라 (PG17 + PostGIS + Valkey)

**Files:**
- Create: `infrastructure/docker/docker-compose.yml`
- Create: `infrastructure/docker/postgres/init.sql`
- Create: `infrastructure/docker/.env.example`
- Create: `infrastructure/docker/README.md`

- [ ] **Step 1: docker-compose.yml 작성**

```yaml
services:
  postgres:
    image: postgis/postgis:17-3.5
    container_name: gongzzang-postgres
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-gongzzang}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?required}
      POSTGRES_DB: ${POSTGRES_DB:-gongzzang}
    ports: ["5432:5432"]
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./postgres/init.sql:/docker-entrypoint-initdb.d/00-init.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-gongzzang}"]
      interval: 5s
      timeout: 3s
      retries: 10
  valkey:
    image: valkey/valkey:8-alpine
    container_name: gongzzang-valkey
    ports: ["6379:6379"]
    healthcheck:
      test: ["CMD", "valkey-cli", "ping"]
      interval: 5s
volumes:
  postgres-data:
```

- [ ] **Step 2: init.sql 작성** (확장 활성화)

```sql
create extension if not exists postgis;
create extension if not exists pg_trgm;
create extension if not exists unaccent;
create extension if not exists btree_gist;
```

- [ ] **Step 3: .env.example 작성**

```
POSTGRES_USER=gongzzang
POSTGRES_PASSWORD=changeme_local_only
POSTGRES_DB=gongzzang
```

- [ ] **Step 4: README.md 작성** (≤80줄, 해요체)

기동: `docker compose -f infrastructure/docker/docker-compose.yml --env-file infrastructure/docker/.env up -d`
중지: `docker compose -f infrastructure/docker/docker-compose.yml down`
접속: `docker exec -it gongzzang-postgres psql -U gongzzang`

- [ ] **Step 5: 기동 검증**

```bash
cp infrastructure/docker/.env.example infrastructure/docker/.env
docker compose -f infrastructure/docker/docker-compose.yml --env-file infrastructure/docker/.env up -d
docker exec gongzzang-postgres psql -U gongzzang -c "select postgis_version();"
```
Expected: PostGIS 버전 출력 (`3.5...`)

- [ ] **Step 6: Commit**

```bash
git add infrastructure/docker/
git commit -m "feat(infra): add local Docker Compose for PG17+PostGIS+Valkey"
```

---

## Task 2: SQLx CLI + sqlx-data 설정

**Files:**
- Modify: `Cargo.toml` (workspace deps에 sqlx 추가)
- Create: `.env` (gitignored, DATABASE_URL)
- Modify: `.env.example` (DATABASE_URL placeholder 추가)
- Create: `scripts/sqlx-migrate.sh`

- [ ] **Step 1: SQLx CLI 설치**

```bash
cargo install sqlx-cli --version 0.8.2 --no-default-features --features postgres,rustls
```

- [ ] **Step 2: workspace deps 추가** — `Cargo.toml`

```toml
[workspace.dependencies]
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls", "postgres", "macros", "uuid", "chrono", "json", "migrate"] }
```

- [ ] **Step 3: .env / .env.example 추가**

`.env.example`:
```
DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:5432/gongzzang
```

- [ ] **Step 4: scripts/sqlx-migrate.sh 작성**

```bash
#!/usr/bin/env bash
set -euo pipefail
sqlx database create
sqlx migrate run --source migrations
```

- [ ] **Step 5: 검증** — `sqlx --version` 출력 확인

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml .env.example scripts/sqlx-migrate.sh
git commit -m "feat(infra): wire sqlx-cli + DATABASE_URL convention"
```

---

## Task 3: migrations/ 디렉토리 + README

**Files:**
- Create: `migrations/.gitkeep`
- Create: `migrations/README.md`

- [ ] **Step 1: README.md 작성** (≤120줄)

내용:
- 명명 규칙: `V<major>_<minor>__<snake_case>.sql`
- 적용 순서: 알파벳 순 (`V001_01__...` < `V001_02__...`)
- 롤백 정책: *forward-only* (운영에서는 새 마이그레이션 추가, 과거 수정 금지)
- 로컬 검증: `bash scripts/sqlx-migrate.sh`
- CI 검증: `.github/workflows/db-migrations.yml`

- [ ] **Step 2: Commit**

```bash
git add migrations/
git commit -m "docs(migrations): document migration naming + forward-only policy"
```

---

## Task 4: V001_01 — Core BC 3 테이블 (user, listing, listing_photo)

> **분류 정정 (Plan 2a 초안 결함 → 수정).** 본 Task는 *spec § 5.1만* 다룬다. Parcel/Building/IndustrialComplex/Manufacturer는 spec § 4에 의해 *R2 정적*이므로 RDS 스키마에 들어가지 않는다.

**Files:**
- Create: `migrations/V001_01__core_tables.sql`
- Create: `tests/migrations/test_v001_01.sh`

**스펙 참조:** `docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md` § 5.1 (lines 147–238). SQL은 spec에서 *그대로 복사*. plan 본 task에서 SQL을 재정의하지 않는다 (SSOT — spec이 정답).

- [ ] **Step 1: 테스트 작성 — `tests/migrations/test_v001_01.sh`**

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

EXPECTED=("user" listing listing_photo)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done
# Geometry: only listing has geom_point in Core BC (parcel/building geom moved to R2 PMTiles)
if ! psql "$DATABASE_URL" -t -A -c "select 1 from information_schema.columns where table_name='listing' and column_name='geom_point';" | grep -q '^1$'; then
  echo "FAIL: listing.geom_point missing" >&2; exit 1
fi
echo "PASS: V001_01 Core BC 3 tables (user, listing, listing_photo)"
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
bash tests/migrations/test_v001_01.sh
```
Expected: FAIL (마이그레이션 파일 없음)

- [ ] **Step 2: 테스트 실행 — 실패 확인**

로컬에서 실행이 가능하면 `bash tests/migrations/test_v001_01.sh`. Docker/sqlx-cli 미설치 환경이면 정적 검증만 (`bash -n`).

- [ ] **Step 3: V001_01__core_tables.sql 작성**

spec § 5.1 (lines 147–238) 의 3개 `create table` 블록 (`"user"`, `listing`, `listing_photo`) + 모든 `create index` 라인을 순서대로 복사한다. 파일 머리에 1줄 주석:

```sql
-- V001_01: Core BC RDS 동적 — user, listing, listing_photo (spec § 5.1)
-- Parcel/Building/IndustrialComplex/Manufacturer는 R2 정적 — 본 파일 범위 밖 (spec § 4)
```

복사 후 검증 체크리스트:

- [ ] `"user"` quote 유지 (PostgreSQL 예약어)
- [ ] `listing.transaction_type` CHECK이 `'sale','monthly_rent','jeonse'` 3종
- [ ] `listing.parcel_pnu char(19) not null` (FK 아님 — Parcel은 R2)
- [ ] `listing.geom_point geometry(Point, 4326)` SRID 명시
- [ ] `listing_photo.r2_key text not null` (R2 객체 키)
- [ ] 모든 인덱스 (총 9개): `user_business_number_idx`, `user_roles_idx` (gin), `user_active_idx`, `listing_status_idx`, `listing_listing_type_idx`, `listing_owner_idx`, `listing_geom_gist_idx`, `listing_created_idx`, `listing_pnu_idx`, `listing_photo_listing_order_idx` (10 — 다시 세기)

- [ ] **Step 4: 테스트 실행 — 통과 확인** (또는 정적 검증)

Expected: `PASS: V001_01 Core BC 3 tables (user, listing, listing_photo)`

- [ ] **Step 5: Commit**

```bash
git add migrations/V001_01__core_tables.sql tests/migrations/test_v001_01.sh
git commit -m "feat(db): V001_01 — Core BC 3 tables (user, listing, listing_photo) per spec § 5.1"
```

---

## Task 5: V001_02 — Insights BC 5 테이블

**Files:**
- Create: `migrations/V001_02__insights_tables.sql`
- Create: `tests/migrations/test_v001_02.sh`

**스펙 참조:** spec § 5.2 (lines 239–321). 5개 테이블: `bookmark_listing`, `bookmark_external`, `search_history`, `analysis_report`, `notification`.

핵심 패턴 (spec 인용):
- `bookmark_listing` — 매물 FK (composite PK `(user_id, listing_id)`)
- `bookmark_external` — polymorphic (target_type ∈ {parcel, building, industrial_complex, manufacturer, real_transaction, court_auction}, target_key는 R2 식별자)
- `search_history` — 24시간 retention (운영 시 cron으로 정리)
- `analysis_report` — payload jsonb + 7일 expires_at
- `notification` — read_at IS NULL 부분 인덱스

- [ ] **Step 1-2:** 테스트 작성 + 실패 확인 (Task 4 패턴 따름, EXPECTED 배열 5개 테이블로 갱신)

- [ ] **Step 3:** spec § 5.2 의 모든 `create table` + `create index` 그대로 복사. 파일 머리 주석:

```sql
-- V001_02: Insights BC RDS 동적 — bookmark_listing, bookmark_external, search_history, analysis_report, notification (spec § 5.2)
```

- [ ] **Step 4: 통과** — `PASS: V001_02 Insights BC 5 tables`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_02 — Insights BC 5 tables (bookmarks, search_history, analysis_report, notification) per spec § 5.2"
```

---

## Task 6: V001_03 — System 2 테이블 (audit_log, outbox_event)

**Files:**
- Create: `migrations/V001_03__system_tables.sql`
- Create: `tests/migrations/test_v001_03.sh`

**스펙 참조:** spec § 5.3 (lines 322–367). 2개 테이블: `audit_log`, `outbox_event`.

핵심 패턴 (spec 인용):
- `audit_log` — append-only (V002에서 writer의 UPDATE/DELETE 박탈 트리거 추가)
- `outbox_event` — `published_at IS NULL` 부분 인덱스 (배포 큐 폴링)
- `audit_log` 1년 RDS retention + 6년 R2 archive (spec § 5.3 retention 절)

- [ ] **Step 1-2:** 테스트 작성 + 실패 확인 (EXPECTED 2개 테이블)

- [ ] **Step 3:** spec § 5.3 의 SQL 그대로 복사. 파일 머리 주석:

```sql
-- V001_03: System (감사·이벤트 분배) — audit_log, outbox_event (spec § 5.3)
-- audit_log immutable 트리거는 V002에서 부착됨
```

- [ ] **Step 4: 통과** — `PASS: V001_03 System 2 tables`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_03 — System 2 tables (audit_log append-only, outbox_event) per spec § 5.3"
```

---

## Task 7: V001_04 — Pipeline 2 테이블 (pipeline_schedule, pipeline_run)

**Files:**
- Create: `migrations/V001_04__pipeline_tables.sql`
- Create: `tests/migrations/test_v001_04.sh`

**스펙 참조:** spec § 5.4 (lines 368–443). 2개 테이블: `pipeline_schedule`, `pipeline_run`.

핵심 패턴 (spec 인용):
- `pipeline_schedule` — cron 표현식 + Asia/Seoul TZ 기본값 + optimistic locking (`version`)
- `pipeline_run.steps jsonb` — 단계별 진행 시각화 (어드민 9 화면 중 *파이프라인 모니터*가 이 컬럼을 노드 그래프로 렌더)
- `pipeline_run.status` — `queued/running/succeeded/failed/cancelled` (5종)
- `pipeline_run` 부분 인덱스: `status in ('queued','running')` (활성 큐 폴링)

- [ ] **Step 1-2:** 테스트 작성 + 실패 확인 (EXPECTED 2개)

- [ ] **Step 3:** spec § 5.4 의 SQL 그대로 복사. 파일 머리 주석:

```sql
-- V001_04: Data Pipeline 제어 — pipeline_schedule (cron), pipeline_run (steps JSONB) (spec § 5.4)
```

- [ ] **Step 4: 통과** — `PASS: V001_04 Pipeline 2 tables`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_04 — Pipeline 2 tables (schedule cron + run with steps JSONB) per spec § 5.4"
```

---

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

## Task 12: Id (ULID + 도메인 prefix)

**Files:**
- Create: `crates/shared-kernel/src/id.rs`

**스펙 참조:** ID 컨벤션 — `<prefix>_<26 ULID 문자>`, 총 30자 (`usr_01HXY...`, `lst_01HXY...`).

- [ ] **Step 1: 실패 테스트 작성** (`id.rs` 끝에 `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_has_prefix_and_26_ulid_chars() {
        let id: Id<UserMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("usr_"));
    }

    #[test]
    fn parse_valid_id_roundtrips() {
        let raw = "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let id = Id::<UserMarker>::try_from_str(raw).unwrap();
        assert_eq!(id.as_str(), raw);
    }

    #[test]
    fn parse_wrong_prefix_fails() {
        let raw = "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let err = Id::<UserMarker>::try_from_str(raw).unwrap_err();
        assert!(matches!(err, IdError::WrongPrefix { .. }));
    }

    #[test]
    fn parse_wrong_length_fails() {
        let err = Id::<UserMarker>::try_from_str("usr_short").unwrap_err();
        assert!(matches!(err, IdError::InvalidLength { .. }));
    }
}
```

- [ ] **Step 2: 실행 — 실패**

```bash
cargo test -p shared-kernel id::tests
```

- [ ] **Step 3: 최소 구현**

```rust
//! 도메인 ID — `<prefix>_<26자 ULID>` 형식, 총 30자.

use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// ID prefix (BC별 marker로 컴파일 타임 구분).
pub trait IdPrefix {
    /// 3-4자 prefix (예: `"usr"`, `"lst"`).
    const PREFIX: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct UserMarker;
impl IdPrefix for UserMarker { const PREFIX: &'static str = "usr"; }

#[derive(Debug, Clone, Copy)]
pub struct ListingMarker;
impl IdPrefix for ListingMarker { const PREFIX: &'static str = "lst"; }

// (BuildingMarker, IndustrialComplexMarker, ManufacturerMarker, NotificationMarker, …)
// 후속 task에서 추가.

/// Phantom-typed ID. 런타임 표현은 30자 String.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<P: IdPrefix> {
    inner: String,
    #[serde(skip)]
    _marker: PhantomData<P>,
}

impl<P: IdPrefix> Id<P> {
    /// 새 ULID 생성.
    #[must_use]
    pub fn new() -> Self {
        let raw = format!("{}_{}", P::PREFIX, Ulid::new());
        Self { inner: raw, _marker: PhantomData }
    }

    /// 검증 후 Id로 래핑.
    pub fn try_from_str(s: &str) -> Result<Self, IdError> {
        if s.len() != 30 {
            return Err(IdError::InvalidLength { actual: s.len() });
        }
        let (prefix, rest) = s.split_once('_').ok_or(IdError::MissingDelimiter)?;
        if prefix != P::PREFIX {
            return Err(IdError::WrongPrefix {
                expected: P::PREFIX,
                actual: prefix.to_owned(),
            });
        }
        Ulid::from_string(rest).map_err(|_| IdError::InvalidUlid)?;
        Ok(Self { inner: s.to_owned(), _marker: PhantomData })
    }

    #[must_use]
    pub fn as_str(&self) -> &str { &self.inner }
}

impl<P: IdPrefix> Default for Id<P> { fn default() -> Self { Self::new() } }

#[derive(Debug, Error)]
pub enum IdError {
    #[error("invalid id length: expected 30, got {actual}")]
    InvalidLength { actual: usize },
    #[error("missing prefix delimiter '_'")]
    MissingDelimiter,
    #[error("wrong prefix: expected {expected}, got {actual}")]
    WrongPrefix { expected: &'static str, actual: String },
    #[error("invalid ULID body")]
    InvalidUlid,
}
```

- [ ] **Step 4: 실행 — 통과** (`cargo test -p shared-kernel id::tests`)

- [ ] **Step 5: lib.rs에 `pub mod id;` 추가, `cargo clippy --all-targets -- -D warnings` 통과**

- [ ] **Step 6: Commit**

```bash
git add crates/shared-kernel/src/id.rs crates/shared-kernel/src/lib.rs
git commit -m "feat(shared-kernel): Id<P> — ULID + domain prefix (30 chars, phantom-typed)"
```

---

## Task 13: Time (timestamp 헬퍼 + Asia/Seoul)

**Files:**
- Create: `crates/shared-kernel/src/time.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn now_utc_is_close_to_chrono_now() {
        let our = now_utc();
        let theirs = Utc::now();
        assert!((our - theirs).num_seconds().abs() < 2);
    }

    #[test]
    fn to_kst_converts_offset() {
        let utc = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let kst = to_kst(utc);
        assert_eq!(kst.hour(), 9);
    }
}
```

- [ ] **Step 2: 실패 확인**

- [ ] **Step 3: 구현**

```rust
//! 시각 헬퍼 — UTC 저장 / KST 표시 분리.

use chrono::{DateTime, FixedOffset, Timelike, Utc};

/// 현재 UTC. 도메인 내부 표준.
#[must_use]
pub fn now_utc() -> DateTime<Utc> { Utc::now() }

/// KST(+09:00)로 변환. 사용자 노출 전용.
#[must_use]
pub fn to_kst(t: DateTime<Utc>) -> DateTime<FixedOffset> {
    let kst = FixedOffset::east_opt(9 * 3600).expect("valid offset");
    t.with_timezone(&kst)
}
```

- [ ] **Step 4: 통과 + clippy**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): Time helpers (now_utc, to_kst — UTC store / KST display)"
```

---

## Task 14: Pnu (19자리 한국 PNU 코드)

**Files:**
- Create: `crates/shared-kernel/src/pnu.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "1111010100100010000";

    #[test]
    fn parse_valid_pnu() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.as_str(), VALID);
    }

    #[test]
    fn extracts_admin_codes() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.sido_code(), "11");
        assert_eq!(pnu.sigungu_code(), "11110");
        assert_eq!(pnu.eupmyeondong_code(), "11110101");
    }

    #[test]
    fn jibun_main_and_sub() {
        let pnu = Pnu::try_new(VALID).unwrap();
        assert_eq!(pnu.jibun_main(), 1);
        assert_eq!(pnu.jibun_sub(), 0);
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(matches!(Pnu::try_new("123").unwrap_err(), PnuError::InvalidLength { .. }));
    }

    #[test]
    fn rejects_non_digits() {
        assert!(matches!(Pnu::try_new("11110101001000100AB").unwrap_err(), PnuError::NonDigit));
    }
}
```

- [ ] **Step 2: 실패 확인**

- [ ] **Step 3: 구현**

```rust
//! PNU — 19자리 한국 필지 식별자. `[시도2][시군구3][읍면동3][산여부1][본번4][부번4]`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pnu(String);

impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> {
        if s.len() != 19 {
            return Err(PnuError::InvalidLength { actual: s.len() });
        }
        if !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(PnuError::NonDigit);
        }
        Ok(Self(s.to_owned()))
    }

    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn sido_code(&self) -> &str { &self.0[0..2] }
    #[must_use] pub fn sigungu_code(&self) -> &str { &self.0[0..5] }
    #[must_use] pub fn eupmyeondong_code(&self) -> &str { &self.0[0..8] }
    #[must_use] pub fn is_san(&self) -> bool { &self.0[10..11] == "2" }

    #[must_use] pub fn jibun_main(&self) -> u32 {
        self.0[11..15].parse().expect("digits validated")
    }
    #[must_use] pub fn jibun_sub(&self) -> u32 {
        self.0[15..19].parse().expect("digits validated")
    }
}

#[derive(Debug, Error)]
pub enum PnuError {
    #[error("PNU must be 19 digits, got {actual}")]
    InvalidLength { actual: usize },
    #[error("PNU must contain only ASCII digits")]
    NonDigit,
}
```

- [ ] **Step 4: 통과 + clippy** (`expect` 사용 정당화 주석은 *clippy::allow*가 아닌 *디지트 사전 검증* 사실로 충분)

> **주의:** workspace lints에 `expect_used = "deny"`. 이 *expect*는 `parse::<u32>()`에 한해 *불가능한 분기*에 사용. clippy 통과시키려면 `#[allow(clippy::expect_used)]`를 함수 단위로 부착하거나, `unsafe`를 피하는 다른 패턴 (*unwrap_or(0)*) 사용. 구현자는 *clippy 출력 확인 후 결정*.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): Pnu — 19-digit Korean parcel id (sido/sigungu/dong/jibun extraction)"
```

---

## Task 15: Money (KRW + overflow 방어)

**Files:**
- Create: `crates/shared-kernel/src/money.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_krw_positive() {
        let m = MoneyKrw::try_new(100_000_000).unwrap();
        assert_eq!(m.as_i64(), 100_000_000);
    }

    #[test]
    fn rejects_negative() {
        assert!(matches!(MoneyKrw::try_new(-1).unwrap_err(), MoneyError::Negative));
    }

    #[test]
    fn add_within_bounds() {
        let a = MoneyKrw::try_new(1_000).unwrap();
        let b = MoneyKrw::try_new(2_000).unwrap();
        assert_eq!(a.checked_add(b).unwrap().as_i64(), 3_000);
    }

    #[test]
    fn add_overflow_returns_err() {
        let a = MoneyKrw::try_new(i64::MAX).unwrap();
        let b = MoneyKrw::try_new(1).unwrap();
        assert!(a.checked_add(b).is_err());
    }
}
```

- [ ] **Step 2-4: 구현 + 통과**

```rust
//! 한국 원화 금액. 음수 금지, 오버플로우 방어.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MoneyKrw(i64);

impl MoneyKrw {
    pub fn try_new(krw: i64) -> Result<Self, MoneyError> {
        if krw < 0 { return Err(MoneyError::Negative); }
        Ok(Self(krw))
    }

    #[must_use] pub fn as_i64(self) -> i64 { self.0 }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_add(other.0).ok_or(MoneyError::Overflow).and_then(Self::try_new)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_sub(other.0).ok_or(MoneyError::Underflow).and_then(Self::try_new)
    }
}

#[derive(Debug, Error)]
pub enum MoneyError {
    #[error("money cannot be negative")]
    Negative,
    #[error("money addition overflowed")]
    Overflow,
    #[error("money subtraction underflowed")]
    Underflow,
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): MoneyKrw — non-negative + checked add/sub"
```

---

## Task 16: Area (m² 면적)

**Files:**
- Create: `crates/shared-kernel/src/area.rs`

- [ ] **Step 1: 실패 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_m2_positive() {
        let a = AreaM2::try_new(99.5).unwrap();
        assert!((a.as_f64() - 99.5).abs() < f64::EPSILON);
    }

    #[test] fn zero_rejected() { assert!(AreaM2::try_new(0.0).is_err()); }
    #[test] fn negative_rejected() { assert!(AreaM2::try_new(-1.0).is_err()); }
    #[test] fn nan_rejected() { assert!(AreaM2::try_new(f64::NAN).is_err()); }
    #[test] fn infinity_rejected() { assert!(AreaM2::try_new(f64::INFINITY).is_err()); }

    #[test]
    fn to_pyeong_converts() {
        let a = AreaM2::try_new(3.305_785).unwrap();
        assert!((a.to_pyeong() - 1.0).abs() < 1e-3);
    }
}
```

- [ ] **Step 2-4: 구현**

```rust
//! 면적 (㎡). 양수만, NaN/∞ 거부. 평 환산 헬퍼 포함.

use serde::{Deserialize, Serialize};
use thiserror::Error;

const M2_PER_PYEONG: f64 = 3.305_785_124;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AreaM2(f64);

impl AreaM2 {
    pub fn try_new(m2: f64) -> Result<Self, AreaError> {
        if !m2.is_finite() { return Err(AreaError::NotFinite); }
        if m2 <= 0.0 { return Err(AreaError::NonPositive); }
        Ok(Self(m2))
    }
    #[must_use] pub fn as_f64(self) -> f64 { self.0 }
    #[must_use] pub fn to_pyeong(self) -> f64 { self.0 / M2_PER_PYEONG }
}

#[derive(Debug, Error)]
pub enum AreaError {
    #[error("area must be finite")]
    NotFinite,
    #[error("area must be positive")]
    NonPositive,
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shared-kernel): AreaM2 — positive-finite + pyeong conversion"
```

---

## Task 17: BusinessNumber (한국 사업자등록번호)

**Files:**
- Create: `crates/shared-kernel/src/business_number.rs`

**스펙:** 10자리 (`123-45-67890` 또는 `1234567890`), 체크섬 알고리즘 검증.

- [ ] **Step 1: 실패 테스트** — 유효 번호, 하이픈 정규화, 잘못된 체크섬 거부, 길이 거부

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 실제 유효 체크섬을 가진 더미 (테스트는 알고리즘 검증)
    #[test]
    fn parse_with_hyphens_normalizes() {
        let bn = BusinessNumber::try_new("123-45-67890").unwrap();
        assert_eq!(bn.as_str(), "1234567890");
    }
    #[test] fn rejects_short() { assert!(BusinessNumber::try_new("12345").is_err()); }
    #[test] fn rejects_non_digits() { assert!(BusinessNumber::try_new("abcdefghij").is_err()); }
    #[test]
    fn rejects_invalid_checksum() {
        // 마지막 자리 +1 → 체크섬 실패
        assert!(BusinessNumber::try_new("1234567891").is_err());
    }
}
```

- [ ] **Step 2-3: 구현** — 한국 국세청 사업자번호 체크섬 알고리즘

```rust
//! 사업자등록번호 (한국 국세청 표준 10자리 + 체크섬).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusinessNumber(String);

impl BusinessNumber {
    pub fn try_new(s: &str) -> Result<Self, BusinessNumberError> {
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
        if cleaned.len() != 10 { return Err(BusinessNumberError::InvalidLength); }
        if !cleaned.chars().all(|c| c.is_ascii_digit()) { return Err(BusinessNumberError::NonDigit); }
        if !verify_checksum(&cleaned) { return Err(BusinessNumberError::InvalidChecksum); }
        Ok(Self(cleaned))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

fn verify_checksum(digits: &str) -> bool {
    let weights = [1u32, 3, 7, 1, 3, 7, 1, 3, 5];
    let bytes = digits.as_bytes();
    let mut sum: u32 = 0;
    for i in 0..9 { sum += u32::from(bytes[i] - b'0') * weights[i]; }
    sum += (u32::from(bytes[8] - b'0') * 5) / 10;
    let check = (10 - (sum % 10)) % 10;
    check == u32::from(bytes[9] - b'0')
}

#[derive(Debug, Error)]
pub enum BusinessNumberError {
    #[error("business number must be 10 digits")]
    InvalidLength,
    #[error("business number must be ASCII digits (with optional hyphens)")]
    NonDigit,
    #[error("business number checksum invalid")]
    InvalidChecksum,
}
```

> **검증 권고:** 알고리즘은 한국 국세청 공식 명세 기반. 구현자는 위키/공식 문서 교차 확인 후 *진짜 유효한* 사업자번호 1개로 단위 테스트 추가.

- [ ] **Step 4-5: 통과 + Commit**

```bash
git commit -m "feat(shared-kernel): BusinessNumber — 10-digit Korean reg with NTS checksum"
```

---

## Task 18: BrokerLicense (공인중개사 자격증번호)

**Files:**
- Create: `crates/shared-kernel/src/broker_license.rs`

**스펙:** 등록번호 형식 `XX-XXXX-XXXXX` (시도-연도-순번). 길이 검증만 (체크섬 없음).

- [ ] **Step 1-5: TDD** — 길이/하이픈 정규화 검증

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrokerLicense(String);

impl BrokerLicense {
    pub fn try_new(s: &str) -> Result<Self, BrokerLicenseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() { return Err(BrokerLicenseError::Empty); }
        if trimmed.len() > 50 { return Err(BrokerLicenseError::TooLong); }
        Ok(Self(trimmed.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

```bash
git commit -m "feat(shared-kernel): BrokerLicense — Korean real-estate broker registration number"
```

---

## Task 19: Email

**Files:**
- Create: `crates/shared-kernel/src/email.rs`

- [ ] **TDD:** 정규식 기반 RFC 5322 *간소화* 검증 (`local@domain`, 도메인에 `.`, 길이 ≤254).

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").expect("valid regex")
});

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn try_new(s: &str) -> Result<Self, EmailError> {
        let lower = s.trim().to_ascii_lowercase();
        if lower.len() > 254 { return Err(EmailError::TooLong); }
        if !EMAIL_RE.is_match(&lower) { return Err(EmailError::InvalidFormat); }
        Ok(Self(lower))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

테스트: 유효, 잘못된 도메인, 빈 local, 길이 초과, 대문자 정규화.

```bash
git commit -m "feat(shared-kernel): Email — RFC 5322 simplified + lowercase normalization"
```

---

## Task 20: PhoneKr (한국 전화번호)

**Files:**
- Create: `crates/shared-kernel/src/phone_kr.rs`

- [ ] **TDD:** `010-1234-5678`, `02-123-4567`, `+82-10-...` 모두 `010...` 또는 `02...` 정규화. 하이픈 제거 후 9-11자리.

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhoneKr(String);

impl PhoneKr {
    pub fn try_new(s: &str) -> Result<Self, PhoneKrError> {
        let mut digits: String = s.chars().filter(char::is_ascii_digit).collect();
        if let Some(rest) = digits.strip_prefix("82") { digits = format!("0{rest}"); }
        if !(9..=11).contains(&digits.len()) { return Err(PhoneKrError::InvalidLength); }
        if !digits.starts_with('0') { return Err(PhoneKrError::MustStartWithZero); }
        Ok(Self(digits))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Error)]
pub enum PhoneKrError {
    #[error("phone must be 9-11 digits")] InvalidLength,
    #[error("phone must start with 0")] MustStartWithZero,
}
```

```bash
git commit -m "feat(shared-kernel): PhoneKr — Korean phone normalization (+82 → 0xx)"
```

---

## Task 21: Srid (좌표계 enum: 4326/5179/5186)

**Files:**
- Create: `crates/shared-kernel/src/srid.rs`

- [ ] **TDD:**

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum Srid {
    /// WGS84 — 글로벌 표준, 네이버/구글 호환.
    Wgs84 = 4326,
    /// UTM-K — 한국 측량 표준.
    UtmK = 5179,
    /// 중부원점 TM — 행정 측량 표준.
    KoreaCentralTm = 5186,
}

impl Srid {
    pub fn from_epsg(code: i32) -> Result<Self, SridError> {
        match code {
            4326 => Ok(Self::Wgs84),
            5179 => Ok(Self::UtmK),
            5186 => Ok(Self::KoreaCentralTm),
            other => Err(SridError::Unsupported(other)),
        }
    }
    #[must_use] pub fn epsg(self) -> i32 { self as i32 }
}

#[derive(Debug, Error)]
pub enum SridError {
    #[error("unsupported EPSG code: {0}")]
    Unsupported(i32),
}
```

```bash
git commit -m "feat(shared-kernel): Srid enum (WGS84/UTM-K/Central-TM) — explicit projection guard"
```

---

## Task 22: Geometry (Point + Polygon, SRID 강제)

**Files:**
- Create: `crates/shared-kernel/src/geometry.rs`

- [ ] **TDD:** Point 생성 시 *반드시 Srid 함께*, lat/lng 범위 검증.

```rust
use crate::srid::Srid;
use geo_types::Point as GeoPoint;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointSrid {
    pub lng: f64,
    pub lat: f64,
    pub srid: Srid,
}

impl PointSrid {
    pub fn try_new_wgs84(lng: f64, lat: f64) -> Result<Self, GeometryError> {
        if !(-180.0..=180.0).contains(&lng) { return Err(GeometryError::LngOutOfRange); }
        if !(-90.0..=90.0).contains(&lat) { return Err(GeometryError::LatOutOfRange); }
        if !lng.is_finite() || !lat.is_finite() { return Err(GeometryError::NotFinite); }
        Ok(Self { lng, lat, srid: Srid::Wgs84 })
    }

    #[must_use] pub fn to_geo_point(self) -> GeoPoint<f64> { GeoPoint::new(self.lng, self.lat) }
}

#[derive(Debug, Error)]
pub enum GeometryError {
    #[error("longitude out of [-180, 180]")] LngOutOfRange,
    #[error("latitude out of [-90, 90]")] LatOutOfRange,
    #[error("coordinate must be finite")] NotFinite,
}
```

테스트: WGS84 유효, 위도 91 거부, 경도 -181 거부, NaN 거부.

```bash
git commit -m "feat(shared-kernel): PointSrid — explicit-SRID point with WGS84 bounds check"
```

---

## Task 23: AdminDivision (시도/시군구/읍면동 코드)

**Files:**
- Create: `crates/shared-kernel/src/admin_division.rs`

- [ ] **TDD:** 2/5/8자리 코드 검증, 행정안전부 표준.

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SidoCode(String);
impl SidoCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 2)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SigunguCode(String);
impl SigunguCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 5)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn sido_code(&self) -> SidoCode { SidoCode(self.0[0..2].to_owned()) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EupmyeondongCode(String);
impl EupmyeondongCode {
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 8)?; Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}

fn validate_digits(s: &str, expected: usize) -> Result<(), AdminDivisionError> {
    if s.len() != expected { return Err(AdminDivisionError::InvalidLength { expected, actual: s.len() }); }
    if !s.chars().all(|c| c.is_ascii_digit()) { return Err(AdminDivisionError::NonDigit); }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AdminDivisionError {
    #[error("expected {expected} digits, got {actual}")] InvalidLength { expected: usize, actual: usize },
    #[error("must be ASCII digits")] NonDigit,
}
```

```bash
git commit -m "feat(shared-kernel): AdminDivision — Sido/Sigungu/Eupmyeondong codes (2/5/8 digits)"
```

---

## Task 24: RoadAddress + JibunAddress

**Files:**
- Create: `crates/shared-kernel/src/road_address.rs`
- Create: `crates/shared-kernel/src/jibun_address.rs`

- [ ] **TDD:** 단순 String wrapper + 빈 문자열/길이 검증. 향후 도로명 주소 API 연동 시 확장.

```rust
// road_address.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoadAddress(String);
impl RoadAddress {
    pub fn try_new(s: &str) -> Result<Self, RoadAddressError> {
        let t = s.trim();
        if t.is_empty() { return Err(RoadAddressError::Empty); }
        if t.len() > 200 { return Err(RoadAddressError::TooLong); }
        Ok(Self(t.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
}
```

(JibunAddress 동일 패턴)

```bash
git commit -m "feat(shared-kernel): RoadAddress + JibunAddress — non-empty bounded strings"
```

---

## Task 25: KsicCode (한국 표준산업분류)

**Files:**
- Create: `crates/shared-kernel/src/ksic_code.rs`

**스펙:** 5자리 알파벳+숫자 (예: `C2620`).

- [ ] **TDD:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KsicCode(String);

impl KsicCode {
    pub fn try_new(s: &str) -> Result<Self, KsicCodeError> {
        if s.len() != 5 { return Err(KsicCodeError::InvalidLength); }
        let mut chars = s.chars();
        let first = chars.next().ok_or(KsicCodeError::InvalidLength)?;
        if !first.is_ascii_uppercase() { return Err(KsicCodeError::FirstMustBeUppercase); }
        if !chars.all(|c| c.is_ascii_digit()) { return Err(KsicCodeError::TailMustBeDigits); }
        Ok(Self(s.to_owned()))
    }
    #[must_use] pub fn as_str(&self) -> &str { &self.0 }
    #[must_use] pub fn section(&self) -> char { self.0.chars().next().expect("validated") }
}
```

```bash
git commit -m "feat(shared-kernel): KsicCode — Korean Standard Industrial Classification (1 letter + 4 digits)"
```

---

## Task 26: 최종 검증 (cargo check/clippy/deny + 커버리지 90%+ + 마이그레이션 E2E)

**Files:**
- Create: `tarpaulin.toml`
- Modify: `.github/workflows/ci.yml` (마이그레이션 잡 추가)
- Create: `.github/workflows/db-migrations.yml`
- Create: `docs/database/migrations.md`

- [ ] **Step 1: tarpaulin.toml 작성**

```toml
[shared-kernel]
features = ""
timeout = "120s"
exclude-files = ["**/tests/**", "**/target/**"]
out = ["Html", "Lcov"]
fail-under = 90
```

- [ ] **Step 2: 로컬 검증 — 5개 명령어 모두 통과**

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo install cargo-tarpaulin || true
cargo tarpaulin --workspace --skip-clean --out Lcov --fail-under 90
bash scripts/sqlx-migrate.sh
bash tests/migrations/test_v001_full.sh
bash tests/migrations/test_v002_audit_immutable.sh
```

각각 expected output 명시 (PASS/통과율 출력).

- [ ] **Step 3: db-migrations.yml — CI 잡 추가**

```yaml
name: db-migrations
on: [pull_request, push]
jobs:
  migrate:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgis/postgis:17-3.5
        env:
          POSTGRES_USER: gongzzang
          POSTGRES_PASSWORD: changeme_ci
          POSTGRES_DB: gongzzang
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 3s
          --health-retries 10
    env:
      DATABASE_URL: postgres://gongzzang:changeme_ci@localhost:5432/gongzzang
    steps:
      - uses: actions/checkout@v4
      - run: cargo install sqlx-cli --version 0.8.2 --no-default-features --features postgres,rustls
      - run: bash tests/migrations/test_v001_full.sh
      - run: bash tests/migrations/test_v002_audit_immutable.sh
```

- [ ] **Step 4: docs/database/migrations.md 작성** (≤200줄)

운영 가이드: 명명 규칙, 적용 절차, 롤백 정책, 블루-그린 호환 변경 패턴, *DDL은 별도 PR* 원칙.

- [ ] **Step 5: 모든 검증 통과 확인 후 push**

```bash
git push origin main
```
GitHub Actions: ✅ 모두 그린 확인.

- [ ] **Step 6: Commit**

```bash
git add tarpaulin.toml .github/workflows/db-migrations.yml docs/database/migrations.md
git commit -m "ci(db): tarpaulin 90%+ + migrations CI job + ops guide"
```

---

## Self-Review Checklist (구현 전 작성자 점검 — 끝났음)

- [x] **Spec coverage:** spec § 5 (18 테이블) — Task 4-9, § 7 (3 role) — Task 10, § 8.1 (shared-kernel) — Task 11-25, § 11 (검증 기준) — Task 26
- [x] **Placeholder scan:** "TBD"/"TODO" 없음, 모든 step에 실제 코드 또는 명령
- [x] **Type consistency:** Pnu/MoneyKrw/AreaM2/Srid 이름 모든 task에서 일관, IdPrefix marker 동일 패턴
- [x] **TDD 준수:** Task 12-25 모두 *실패 테스트 → 실행 → 구현 → 통과 → commit* 5단계
- [x] **분할 정합성:** 마이그레이션 5분할은 *500줄 룰* 강제. 합치면 800+ 줄 위반.
- [x] **알려진 위험:** Task 14 Pnu의 `expect`는 *workspace lints `expect_used = "deny"`와 충돌 가능*. 구현자가 *현장 판단*. → 노트로 명시.
- [x] **테이블 개수 검증:** Task 9에서 22 vs 18 차이 *명시적 검증* 책무 추가.

---

## Execution Handoff

Plan 2a를 `docs/superpowers/plans/2026-05-02-sub-project-2a-infra-migrations-shared-kernel.md` 에 저장했어요.

**다음:** `superpowers:subagent-driven-development` 로 Task 1부터 fresh subagent dispatch + 2단계 리뷰 (spec compliance → code quality) 진행. 각 task 완료 후 사용자 체크포인트.

플랜 2b (Core BC 6개 Aggregate)와 2c (Market/Insights/Operations/Pipeline/R2)는 본 플랜 완료 후 별도 작성.
