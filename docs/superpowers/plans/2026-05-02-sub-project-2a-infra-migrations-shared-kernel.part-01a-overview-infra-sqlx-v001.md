# Sub-project 2a - Part 01A: Overview, Infra, SQLx, And V001 Early Migrations

Parent index: [Sub-project 2a Part 01](./2026-05-02-sub-project-2a-infra-migrations-shared-kernel.part-01.md).
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

**Tech Stack:** PostgreSQL 17, PostGIS 3.5, Valkey 8 (Redis 호환), SQLx 0.8 (offline mode), Rust 1.85, Cargo workspace, ULID, geo-types 0.7, cargo-tarpaulin (커버리지).

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

핵심 패턴 (spec 발췌 — 정확한 컬럼/제약은 spec § 5.2 직접 참조):
- `bookmark_listing` — 매물 FK + composite PK `(user_id, listing_id)` + on delete cascade
- `bookmark_external` — polymorphic. `target_kind` ∈ {`parcel`, `court_auction`, `manufacturer`, `industrial_complex`} (4종). `target_id`는 PNU 또는 R2 식별자. UNIQUE `(user_id, target_kind, target_id)`
- `search_history` — `query text` + `filters jsonb` + `correlation_id` + BRIN index on `created_at`. retention: 90일 후 user_id 가명화, 1년 후 삭제 (PIPA)
- `analysis_report` — `title`, `target_pnus char(19)[]`, `snapshot jsonb` (R2 데이터 시점 고정 캐시), `version` (optimistic locking). expires_at 없음
- `notification` — `kind` + `payload jsonb` + `where read_at is null` 부분 인덱스. retention: 365일

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
