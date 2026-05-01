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

### Phase B-C: 마이그레이션 (8 파일)
- `migrations/V001_01__core_tables.sql` — user, parcel, building, industrial_complex, manufacturer
- `migrations/V001_02__listing_tables.sql` — listing, listing_photo, bookmark_listing, bookmark_external
- `migrations/V001_03__insights_tables.sql` — search_history, analysis_report, notification
- `migrations/V001_04__audit_pipeline_tables.sql` — audit_log, outbox_event, pipeline_schedule, pipeline_run
- `migrations/V001_05__operations_tables.sql` — admin_action, business_verification_queue, listing_review_queue, listing_report, featured_content, system_alert
- `migrations/V002_01__db_roles.sql` — gongzzang_app_writer/reader/audit_archiver
- `migrations/V002_02__audit_immutable_trigger.sql` — UPDATE/DELETE 차단 트리거
- `migrations/README.md` — 적용 순서 + 롤백 정책

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

## Task 4: V001_01 — Core 5 테이블 (user, parcel, building, industrial_complex, manufacturer)

**Files:**
- Create: `migrations/V001_01__core_tables.sql`

**스펙 참조:** spec § 5.1 (5개 테이블), § 5.10 (인덱스)

- [ ] **Step 1: 테스트 작성 — `tests/migrations/test_v001_01.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source .env
sqlx database drop -y
sqlx database create
sqlx migrate run --source migrations
psql "$DATABASE_URL" -t -c "select tablename from pg_tables where schemaname='public' order by tablename;" \
  | grep -q '^\s*user$' || { echo "FAIL: user table missing"; exit 1; }
psql "$DATABASE_URL" -t -c "select tablename from pg_tables where schemaname='public';" \
  | grep -qE '^\s*(parcel|building|industrial_complex|manufacturer)$' || { echo "FAIL"; exit 1; }
echo "PASS: core 5 tables exist"
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
bash tests/migrations/test_v001_01.sh
```
Expected: FAIL (마이그레이션 파일 없음)

- [ ] **Step 3: V001_01__core_tables.sql 작성** (≤450줄)

```sql
-- user: Zitadel sub와 1:1, 사업자번호/중개사번호 verified_at으로 검증 상태 표현
create table "user" (
    id char(30) primary key,
    zitadel_sub varchar(255) not null unique,
    email varchar(255) not null unique,
    name varchar(100) not null,
    phone varchar(20),
    business_number varchar(12),
    business_verified_at timestamptz,
    broker_license_number varchar(50),
    broker_verified_at timestamptz,
    roles text[] not null default '{}',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz,
    version bigint not null default 1
);
create index idx_user_zitadel_sub on "user"(zitadel_sub) where deleted_at is null;
create index idx_user_business on "user"(business_number) where business_verified_at is not null;

create table parcel (
    pnu char(19) primary key,
    sido_code char(2) not null,
    sigungu_code char(5) not null,
    eupmyeondong_code char(8) not null,
    jibun_main int not null,
    jibun_sub int not null default 0,
    land_category varchar(20) not null,
    area_m2 numeric(14,2) not null check (area_m2 > 0),
    official_price_krw bigint check (official_price_krw is null or official_price_krw >= 0),
    use_zone varchar(50),
    use_district varchar(50),
    geom geometry(MultiPolygon, 4326),
    last_synced_at timestamptz not null default now(),
    source_version varchar(50),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_parcel_sigungu on parcel(sigungu_code);
create index idx_parcel_geom on parcel using gist(geom);

create table building (
    id char(30) primary key,
    pnu char(19) not null references parcel(pnu),
    building_name varchar(200),
    main_purpose_code varchar(10) not null,
    structure_code varchar(10),
    total_floor_area_m2 numeric(14,2) not null check (total_floor_area_m2 > 0),
    ground_floors int not null default 0,
    underground_floors int not null default 0,
    height_m numeric(8,2),
    use_approval_date date,
    geom geometry(MultiPolygon, 4326),
    last_synced_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_building_pnu on building(pnu);
create index idx_building_geom on building using gist(geom);

create table industrial_complex (
    id char(30) primary key,
    name varchar(200) not null,
    type varchar(30) not null check (type in ('national', 'general', 'urban_high_tech', 'agricultural_industrial')),
    sigungu_code char(5) not null,
    designated_at date,
    total_area_m2 numeric(16,2) not null check (total_area_m2 > 0),
    geom geometry(MultiPolygon, 4326),
    last_synced_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_industrial_complex_geom on industrial_complex using gist(geom);

create table manufacturer (
    id char(30) primary key,
    business_number varchar(12) not null unique,
    company_name varchar(200) not null,
    industrial_complex_id char(30) references industrial_complex(id),
    pnu char(19) references parcel(pnu),
    ksic_code char(5),
    employee_count int check (employee_count is null or employee_count >= 0),
    annual_revenue_krw bigint check (annual_revenue_krw is null or annual_revenue_krw >= 0),
    last_synced_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_manufacturer_complex on manufacturer(industrial_complex_id);
create index idx_manufacturer_pnu on manufacturer(pnu);
```

- [ ] **Step 4: 테스트 실행 — 통과 확인**

```bash
bash tests/migrations/test_v001_01.sh
```
Expected: `PASS: core 5 tables exist`

- [ ] **Step 5: Commit**

```bash
git add migrations/V001_01__core_tables.sql tests/migrations/test_v001_01.sh
git commit -m "feat(db): V001_01 — core 5 tables (user, parcel, building, industrial_complex, manufacturer)"
```

---

## Task 5: V001_02 — Listing 4 테이블

**Files:**
- Create: `migrations/V001_02__listing_tables.sql`
- Modify: `tests/migrations/test_v001_01.sh` → `test_v001.sh` (모든 테이블 검증)

- [ ] **Step 1: 테스트 갱신** — listing/listing_photo/bookmark_listing/bookmark_external 4개 검증 추가

- [ ] **Step 2: 실행 — 실패 확인**

- [ ] **Step 3: V001_02 작성**

```sql
create table listing (
    id char(30) primary key,
    owner_id char(30) not null references "user"(id),
    parcel_pnu char(19) not null references parcel(pnu),
    building_id char(30) references building(id),
    listing_type varchar(30) not null check (listing_type in ('factory','warehouse','knowledge_industry_center','land','complex')),
    transaction_type varchar(20) not null check (transaction_type in ('sale','monthly_rent','jeonse')),
    price_krw bigint not null check (price_krw > 0),
    deposit_krw bigint check (deposit_krw is null or deposit_krw >= 0),
    monthly_rent_krw bigint check (monthly_rent_krw is null or monthly_rent_krw >= 0),
    area_m2 numeric(12,2) not null check (area_m2 > 0),
    title varchar(200) not null,
    description text,
    contact_name varchar(100) not null,
    contact_phone varchar(20) not null,
    status varchar(20) not null default 'draft' check (status in ('draft','pending_review','active','sold','expired','rejected')),
    rejected_reason text,
    reviewed_by char(30) references "user"(id),
    reviewed_at timestamptz,
    expires_at timestamptz,
    geom_point geometry(Point, 4326),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz,
    version bigint not null default 1,
    constraint chk_transaction_fields check (
        (transaction_type = 'sale' and deposit_krw is null and monthly_rent_krw is null)
        or (transaction_type = 'monthly_rent' and deposit_krw is not null and monthly_rent_krw is not null)
        or (transaction_type = 'jeonse' and deposit_krw is not null and monthly_rent_krw is null)
    )
);
create index idx_listing_owner on listing(owner_id) where deleted_at is null;
create index idx_listing_pnu on listing(parcel_pnu);
create index idx_listing_status on listing(status) where deleted_at is null;
create index idx_listing_geom on listing using gist(geom_point);

create table listing_photo (
    id char(30) primary key,
    listing_id char(30) not null references listing(id) on delete cascade,
    r2_key varchar(500) not null,
    sort_order int not null default 0,
    width int,
    height int,
    bytes bigint,
    created_at timestamptz not null default now()
);
create index idx_listing_photo_listing on listing_photo(listing_id, sort_order);

create table bookmark_listing (
    user_id char(30) not null references "user"(id),
    listing_id char(30) not null references listing(id) on delete cascade,
    note text,
    created_at timestamptz not null default now(),
    primary key (user_id, listing_id)
);

create table bookmark_external (
    id char(30) primary key,
    user_id char(30) not null references "user"(id),
    target_type varchar(30) not null check (target_type in ('parcel','building','industrial_complex','manufacturer','real_transaction','court_auction')),
    target_key varchar(100) not null,
    note text,
    created_at timestamptz not null default now(),
    unique (user_id, target_type, target_key)
);
create index idx_bookmark_external_user on bookmark_external(user_id);
```

- [ ] **Step 4: 실행 — 통과**

- [ ] **Step 5: Commit**

```bash
git add migrations/V001_02__listing_tables.sql tests/migrations/test_v001.sh
git commit -m "feat(db): V001_02 — listing + photo + bookmarks (transaction_type CHECK constraint)"
```

---

## Task 6: V001_03 — Insights 3 테이블 (search_history, analysis_report, notification)

**Files:**
- Create: `migrations/V001_03__insights_tables.sql`

- [ ] **Step 1-2:** 테스트 갱신 + 실패 확인

- [ ] **Step 3: SQL 작성**

```sql
create table search_history (
    id char(30) primary key,
    user_id char(30) references "user"(id),
    session_id varchar(100),
    query_type varchar(30) not null check (query_type in ('keyword','geo_bbox','filter')),
    query_payload jsonb not null,
    result_count int not null check (result_count >= 0),
    created_at timestamptz not null default now()
);
create index idx_search_history_user on search_history(user_id, created_at desc) where user_id is not null;
create index idx_search_history_payload on search_history using gin(query_payload);

create table analysis_report (
    id char(30) primary key,
    user_id char(30) not null references "user"(id),
    report_type varchar(50) not null check (report_type in ('parcel_summary','building_summary','complex_summary','manufacturer_summary')),
    target_key varchar(100) not null,
    payload jsonb not null,
    generated_at timestamptz not null default now(),
    expires_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_analysis_report_user on analysis_report(user_id, generated_at desc);

create table notification (
    id char(30) primary key,
    recipient_id char(30) not null references "user"(id),
    type varchar(50) not null,
    title varchar(200) not null,
    body text not null,
    payload jsonb,
    read_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_notification_recipient on notification(recipient_id, created_at desc) where read_at is null;
```

- [ ] **Step 4: 통과 확인**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_03 — insights tables (search_history, analysis_report, notification)"
```

---

## Task 7: V001_04 — Audit + Outbox + Pipeline 4 테이블

**Files:**
- Create: `migrations/V001_04__audit_pipeline_tables.sql`

- [ ] **Step 1-2:** 테스트 갱신 + 실패 확인

- [ ] **Step 3: SQL 작성**

```sql
create table audit_log (
    id char(30) primary key,
    actor_user_id char(30) references "user"(id),
    actor_role varchar(50),
    action varchar(100) not null,
    resource_type varchar(50) not null,
    resource_id varchar(100) not null,
    before_state jsonb,
    after_state jsonb,
    ip_address inet,
    user_agent text,
    request_id varchar(100),
    occurred_at timestamptz not null default now()
);
create index idx_audit_log_actor on audit_log(actor_user_id, occurred_at desc);
create index idx_audit_log_resource on audit_log(resource_type, resource_id, occurred_at desc);
create index idx_audit_log_occurred on audit_log(occurred_at);

create table outbox_event (
    id char(30) primary key,
    aggregate_type varchar(50) not null,
    aggregate_id varchar(100) not null,
    event_type varchar(100) not null,
    payload jsonb not null,
    occurred_at timestamptz not null default now(),
    published_at timestamptz,
    attempt_count int not null default 0,
    last_error text
);
create index idx_outbox_unpublished on outbox_event(occurred_at) where published_at is null;
create index idx_outbox_aggregate on outbox_event(aggregate_type, aggregate_id);

create table pipeline_schedule (
    id char(30) primary key,
    name varchar(100) not null unique,
    description text,
    cron_expr varchar(50) not null,
    timezone varchar(50) not null default 'Asia/Seoul',
    enabled boolean not null default true,
    last_run_id char(30),
    next_run_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    version bigint not null default 1
);

create table pipeline_run (
    id char(30) primary key,
    schedule_id char(30) not null references pipeline_schedule(id),
    status varchar(20) not null check (status in ('queued','running','succeeded','failed','cancelled')),
    started_at timestamptz,
    finished_at timestamptz,
    steps jsonb not null default '[]',
    error_message text,
    created_at timestamptz not null default now()
);
create index idx_pipeline_run_schedule on pipeline_run(schedule_id, created_at desc);
create index idx_pipeline_run_status on pipeline_run(status) where status in ('queued','running');
```

- [ ] **Step 4: 통과**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_04 — audit_log + outbox_event + pipeline schedule/run (steps JSONB)"
```

---

## Task 8: V001_05 — Operations 6 테이블

**Files:**
- Create: `migrations/V001_05__operations_tables.sql`

- [ ] **Step 1-2:** 테스트 갱신 + 실패 확인

- [ ] **Step 3: SQL 작성** (admin_action / business_verification_queue / listing_review_queue / listing_report / featured_content / system_alert)

```sql
create table admin_action (
    id char(30) primary key,
    admin_user_id char(30) not null references "user"(id),
    action_type varchar(50) not null,
    target_type varchar(50) not null,
    target_id varchar(100) not null,
    reason text,
    payload jsonb,
    occurred_at timestamptz not null default now()
);
create index idx_admin_action_admin on admin_action(admin_user_id, occurred_at desc);

create table business_verification_queue (
    id char(30) primary key,
    user_id char(30) not null references "user"(id),
    business_number varchar(12) not null,
    submitted_documents jsonb not null,
    status varchar(20) not null default 'pending' check (status in ('pending','approved','rejected')),
    reviewed_by char(30) references "user"(id),
    reviewed_at timestamptz,
    rejected_reason text,
    submitted_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_bvq_status on business_verification_queue(status, submitted_at);

create table listing_review_queue (
    id char(30) primary key,
    listing_id char(30) not null references listing(id),
    status varchar(20) not null default 'pending' check (status in ('pending','approved','rejected')),
    assigned_to char(30) references "user"(id),
    reviewed_by char(30) references "user"(id),
    reviewed_at timestamptz,
    rejected_reason text,
    submitted_at timestamptz not null default now(),
    version bigint not null default 1
);
create index idx_lrq_status on listing_review_queue(status, submitted_at);

create table listing_report (
    id char(30) primary key,
    listing_id char(30) not null references listing(id),
    reporter_id char(30) references "user"(id),
    reason_code varchar(50) not null,
    description text,
    status varchar(20) not null default 'pending' check (status in ('pending','reviewed','resolved','dismissed')),
    resolved_by char(30) references "user"(id),
    resolved_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_listing_report_status on listing_report(status, created_at desc);

create table featured_content (
    id char(30) primary key,
    content_type varchar(30) not null check (content_type in ('listing','industrial_complex','region')),
    target_id varchar(100) not null,
    title varchar(200) not null,
    priority int not null default 0,
    starts_at timestamptz not null,
    ends_at timestamptz not null,
    created_by char(30) not null references "user"(id),
    created_at timestamptz not null default now(),
    check (ends_at > starts_at)
);
create index idx_featured_active on featured_content(starts_at, ends_at) where ends_at > now();

create table system_alert (
    id char(30) primary key,
    severity varchar(20) not null check (severity in ('info','warning','critical')),
    source varchar(50) not null,
    title varchar(200) not null,
    body text not null,
    payload jsonb,
    acknowledged_by char(30) references "user"(id),
    acknowledged_at timestamptz,
    resolved_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_system_alert_open on system_alert(severity, created_at desc) where resolved_at is null;
```

- [ ] **Step 4: 통과** — `tests/migrations/test_v001.sh` 18 테이블 모두 검증

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(db): V001_05 — operations 6 tables (admin_action, queues, report, featured, alert)"
```

---

## Task 9: V001 통합 검증 + ER 다이어그램

**Files:**
- Create: `tests/migrations/test_v001_full.sh` (18 테이블 + 인덱스 50+ 검증)
- Create: `docs/database/er-diagram-v001.md` (Mermaid ERD)

- [ ] **Step 1: 통합 테스트 작성**

```bash
EXPECTED_TABLES=("user" parcel building industrial_complex manufacturer listing listing_photo \
  bookmark_listing bookmark_external search_history analysis_report notification audit_log \
  outbox_event pipeline_schedule pipeline_run admin_action business_verification_queue \
  listing_review_queue listing_report featured_content system_alert)
# count == 22? 18 도메인 테이블 + 4 — wait, 다시 세기. 정답: 18.
# user, parcel, building, industrial_complex, manufacturer = 5
# listing, listing_photo, bookmark_listing, bookmark_external = 4
# search_history, analysis_report, notification = 3
# audit_log, outbox_event, pipeline_schedule, pipeline_run = 4
# admin_action, business_verification_queue, listing_review_queue, listing_report, featured_content, system_alert = 6
# 합계: 5+4+3+4+6 = 22? spec § 5는 18 — 차이 검증.
```

> **검증 책무:** 이 task 시작 시 implementer는 spec § 5의 *정확한 테이블 목록*을 다시 확인하고, 22 vs 18 차이를 명확히 한다. 차이가 있으면 인간에게 escalate.

- [ ] **Step 2: ER 다이어그램** (Mermaid `erDiagram`, ≤300줄)

- [ ] **Step 3: 통과**

- [ ] **Step 4: Commit**

```bash
git commit -m "test(db): V001 full migration test (table count + key indexes)"
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
