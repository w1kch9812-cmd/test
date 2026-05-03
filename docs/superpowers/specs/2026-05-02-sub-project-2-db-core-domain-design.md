# Sub-project 2 — DB + Core 도메인 설계

| | |
|---|---|
| **작성일** | 2026-05-02 |
| **상태** | Draft (사용자 검토 대기) |
| **타입** | Foundation Layer 2 (sub-project 1 헌법 위에 첫 코드 + 데이터 모델) |
| **소요 추정** | 2-3주 |
| **결과물** | RDS 마이그레이션 V001 + Rust 도메인 모델 + R2 정적 데이터 어댑터 + 첫 단위 테스트 |
| **선행** | sub-project 1 (charter + monorepo) ✅ 완료 |

---

## 1. 목적 (Why)

Sub-project 1이 *집의 설계도면*이었다면, sub-project 2는 *방의 골조 + 벽 + 배관*입니다.

본 sub-project가 끝나면 다음이 가능:
- 한국 산업용 부동산 도메인의 *모든 데이터 종류*가 코드로 표현됨
- 첫 PostgreSQL 마이그레이션이 가능한 18개 테이블 정의
- R2의 정적 공공 데이터 (필지·건축물·산업단지·실거래·경매·법령)를 읽는 어댑터
- 모든 값 객체 (Pnu, Money, Area, BusinessNumber 등)가 Rust로 구현
- 어드민 운영을 위한 *데이터 모델 + 운영 흐름* 디자인

이 골조 위에 sub-project 3(인증), 4(외부 API 통합), 5(API endpoint), 6(프론트엔드)이 쌓인다.

---

## 2. 범위 (Scope)

### 2.1 포함 (In Scope)

- **Rust 도메인 모델** — 6 Aggregate (User, Listing, Parcel, Building, IndustrialComplex, Manufacturer) + 4 Aggregate (RealTransaction, CourtAuction, Bookmark, Notification 등)
- **값 객체** — Pnu, Money, Area, BusinessNumber, BrokerLicense, Geometry, AdminDivision 등
- **Repository trait** (Port) 모든 Aggregate별
- **18개 RDS 테이블** + 첫 마이그레이션 V001__init.sql
- **DB role 분리** (writer / reader / audit_archiver) — V002__db_roles.sql
- **R2 정적 데이터 어댑터** (R2 reader trait + 구현체)
- **R2 디렉토리 구조** 정의 + presigned URL 생성 헬퍼
- **단위 테스트** 모든 값 객체 + 도메인 로직 (커버리지 90%+)
- **Pipeline schedule + run + steps JSONB** 데이터 모델
- **어드민 운영 데이터 모델** (verification queue, review queue, report, featured, alert, admin_action)
- **공유 위젯 데이터 계약** (AuditLogWidget, MetricsWidget 등이 사용할 쿼리 패턴)

### 2.2 제외 (Out of Scope)

- **API endpoint** 구현 — sub-project 5
- **외부 API 호출** (V-World, data.go.kr 등) 실제 통합 — sub-project 4
- **인증 미들웨어** (Zitadel JWT 검증) — sub-project 3
- **Repository 구현체** (SQLx) — *trait만* 정의, 구현은 sub-project 5
- **PMTiles 생성 워커** — sub-project 9 (data-pipeline)
- **어드민 UI 화면** — sub-project 6
- **관측성 (OTel/Tempo) 통합** — sub-project 7
- **인프라 프로비저닝** (Pulumi RDS/R2 셋업) — sub-project 8

### 2.3 결정 보류

- Outbox publisher 서비스 구현 — sub-project 4
- pgvector 임베딩 컬럼 — sub-project 11 (검색)
- 멀티 테넌시 (B2B 임직원 관리) — Phase 4+
- Read replica — Phase 4+

---

## 3. 핵심 의사결정 (이미 합의됨)

| # | 결정 | 출처 |
|---|------|------|
| 1 | 백엔드 = Rust + Axum + SQLx | ADR-0001 |
| 2 | DB = PostgreSQL 17 + PostGIS | ADR-0004 |
| 3 | 인증 = Zitadel | ADR-0005 |
| 4 | 캐시 = moka L1 + Valkey L2 | ADR-0007 |
| 5 | 옵션 A 데이터 플랫폼 (AI 생성 X) | ADR-0010 |
| 6 | 임베딩 = Gemini + pgvector (Phase 3+) | ADR-0011 |
| 7 | **경매 포함** (CourtAuction Aggregate, Market BC) | sub-project 2 brainstorming |
| 8 | **Index/Detail 분리 안 함** — 단일 테이블 + 두 endpoint | sub-project 2 brainstorming |
| 9 | **Listing 거래유형** = 매매 / 월세 / 전세 (산업용 + 한국 시장) | sub-project 2 brainstorming |
| 10 | **데이터 저장** = 정적(R2) + 동적(RDS) 분리 | sub-project 2 brainstorming |
| 11 | **객체 스토리지** = Cloudflare R2 (S3 호환, egress 무료) | sub-project 2 brainstorming |
| 12 | **단일 schema** (12 schema 분리는 YAGNI) — `public` schema 사용 | sub-project 2 brainstorming |
| 13 | **갱신 주기 어드민 동적 관리** — pipeline_schedule 테이블 | sub-project 2 brainstorming |
| 14 | **변경 감지 + 멱등성** — 해시 비교 + advisory lock + 시도별 shard 단위 | sub-project 2 brainstorming |
| 15 | **사진** = listing_photo 별도 테이블 (R2 key 저장, presigned URL 동적 생성) | sub-project 2 brainstorming |
| 16 | **Bookmark** = 하이브리드 (bookmark_listing FK + bookmark_external polymorphic) | sub-project 2 brainstorming |
| 17 | **Audit retention** = 1년 RDS + 6년 R2 IA archive (총 7년 PIPA + ISMS-P) | sub-project 2 brainstorming |
| 18 | **Audit immutable** = 별도 DB role + R2 Object Lock | sub-project 2 brainstorming |
| 19 | **시각화** = Grafana embed (서비스 맵, 트레이스, 메트릭) + 자체 UI (파이프라인 단계 진행) | sub-project 2 brainstorming |
| 20 | **어드민 UI** = 컨텍스트 중심 9 화면 + 공유 위젯 (기능별 분리 X) | sub-project 2 brainstorming |

---

## 4. 도메인 분류 — R2 정적 vs RDS 동적

### 원칙
- **외부 공공 데이터** = R2 정적 (V-World/data.go.kr/법제처 응답 → PMTiles/JSON)
- **사용자 등록 콘텐츠** = RDS 동적 (매물, 사용자 활동)
- **사용자 활동** = RDS 동적 (북마크, 검색, 분석 리포트)
- **바이너리 파일** = R2 (사진, presigned URL 업로드)

### 도메인 BC 매핑

```
Core BC:
  RDS 동적 ──── User
              Listing
  R2 정적 ───── Parcel
              Building
              IndustrialComplex
              Manufacturer
  공유 ───── shared-kernel (Pnu, Money, Area, BusinessNumber 등)

Market BC:
  R2 정적 ───── RealTransaction (이력)
              CourtAuction (활성+이력)
  RDS 동적 ──── Subscription (Phase 2+)
              Inquiry (Phase 2+)

Regulation BC:
  R2 정적 ───── Law
              Regulation

Insights BC:
  RDS 동적 ──── Bookmark (listing은 FK, 나머지는 polymorphic)
              SearchHistory
              AnalysisReport
              Notification

Admin/Operations:
  RDS 동적 ──── AdminAction
              BusinessVerificationQueue
              ListingReviewQueue
              ListingReport
              FeaturedContent
              SystemAlert

System:
  RDS 동적 ──── AuditLog (1년 + R2 archive)
              OutboxEvent
              PipelineSchedule
              PipelineRun (steps JSONB)
```

---

## 5. RDS 테이블 18개 (V001__init.sql)

### 5.1 사용자 + 매물 (Core BC 동적)

#### `user`

```sql
create table "user" (
    id char(30) primary key,                            -- usr_01HXY...
    zitadel_sub varchar(255) not null unique,           -- Zitadel JWT sub claim
    email varchar(320) not null unique,
    phone_kr_hash varchar(64),                          -- SHA-256 해시 (PIPA)
    display_name varchar(100) not null,
    user_kind varchar(20) not null check (user_kind in ('individual', 'corporation')),
    business_number varchar(12),                        -- XXX-XX-XXXXX (검증된 사업자만)
    business_verified_at timestamptz,
    broker_license_number varchar(50),                  -- 공인중개사 자격번호
    broker_verified_at timestamptz,
    roles text[] not null default '{}',                 -- ['Buyer','Seller','Broker','Developer','Enterprise','Operator','Admin']
    nice_verified_at timestamptz,                       -- NICE 본인인증
    marketing_consent_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    last_login_at timestamptz,
    deleted_at timestamptz,                             -- Soft delete (PIPA RTBF)
    version bigint not null default 1
);

create index user_business_number_idx on "user"(business_number) where business_number is not null;
create index user_roles_idx on "user" using gin(roles);
create index user_active_idx on "user"(created_at desc) where deleted_at is null;
```

#### `listing`

```sql
create table listing (
    id char(30) primary key,                            -- lst_...
    owner_id char(30) not null references "user"(id),
    parcel_pnu char(19) not null,                       -- R2의 Parcel과 매핑 (FK 아님 — R2이라)
    listing_type varchar(30) not null check (listing_type in
        ('factory', 'warehouse', 'office', 'knowledge_industry_center', 'industrial_land', 'logistics_center')),
    transaction_type varchar(20) not null check (transaction_type in
        ('sale', 'monthly_rent', 'jeonse')),
    price_krw bigint not null check (price_krw > 0),
    deposit_krw bigint check (deposit_krw is null or deposit_krw >= 0),
    monthly_rent_krw bigint check (monthly_rent_krw is null or monthly_rent_krw >= 0),
    area_m2 numeric(12, 2) not null check (area_m2 > 0),
    title varchar(200) not null,
    description text not null default '',
    status varchar(20) not null default 'draft' check (status in
        ('draft', 'pending_review', 'active', 'sold', 'expired', 'rejected')),
    contact_visibility varchar(20) not null default 'login_required' check
        (contact_visibility in ('public', 'login_required', 'verified_only')),
    view_count bigint not null default 0,
    bookmark_count bigint not null default 0,
    geom_point geometry(Point, 4326),                   -- 매물 위치 (지도 마커)
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    expires_at timestamptz,
    version bigint not null default 1,
    constraint listing_transaction_fields_chk check (
        (transaction_type = 'sale' and deposit_krw is null and monthly_rent_krw is null)
        or (transaction_type = 'monthly_rent' and deposit_krw is not null and monthly_rent_krw is not null)
        or (transaction_type = 'jeonse' and deposit_krw is not null and monthly_rent_krw is null)
    )
);

create index listing_status_idx on listing(status);
create index listing_listing_type_idx on listing(listing_type);
create index listing_owner_idx on listing(owner_id);
create index listing_geom_gist_idx on listing using gist(geom_point);
create index listing_created_idx on listing(created_at desc) where status = 'active';
create index listing_pnu_idx on listing(parcel_pnu);
```

#### `listing_photo`

```sql
create table listing_photo (
    id char(30) primary key,                            -- lph_... (3-char prefix per shared-kernel Id<P>)
    listing_id char(30) not null references listing(id) on delete cascade,
    r2_key text not null,                               -- 'listings/lst_01HXY/photos/p1.jpg'
    thumbnail_r2_key text,
    caption varchar(200),
    display_order int not null default 0,
    width_px int,
    height_px int,
    file_size_bytes bigint,
    content_type varchar(50) not null check (content_type in
        ('image/jpeg', 'image/png', 'image/webp')),
    uploaded_at timestamptz not null default now(),
    deleted_at timestamptz
);

create index listing_photo_listing_order_idx on listing_photo(listing_id, display_order)
    where deleted_at is null;
```

### 5.2 사용자 활동 (Insights BC)

#### `bookmark_listing`

```sql
create table bookmark_listing (
    user_id char(30) not null references "user"(id) on delete cascade,
    listing_id char(30) not null references listing(id) on delete cascade,
    note text,
    created_at timestamptz not null default now(),
    primary key (user_id, listing_id)
);

create index bookmark_listing_user_idx on bookmark_listing(user_id, created_at desc);
```

#### `bookmark_external`

```sql
create table bookmark_external (
    id char(30) primary key,                            -- bme_...
    user_id char(30) not null references "user"(id) on delete cascade,
    target_kind varchar(30) not null check (target_kind in
        ('parcel', 'court_auction', 'manufacturer', 'industrial_complex')),
    target_id varchar(50) not null,                     -- PNU 19자리 또는 다른 식별자
    note text,
    created_at timestamptz not null default now(),
    unique(user_id, target_kind, target_id)
);

create index bookmark_external_user_idx on bookmark_external(user_id, created_at desc);
```

#### `search_history`

```sql
create table search_history (
    id char(30) primary key,                            -- srh_... (3-char prefix per shared-kernel Id<P>)
    user_id char(30) references "user"(id),             -- nullable (비로그인)
    query text not null,
    filters jsonb not null default '{}',
    result_count int not null,
    correlation_id varchar(30) not null,
    created_at timestamptz not null default now()
);

create index search_history_user_time_brin_idx on search_history using brin(created_at);
-- retention: 90일 후 user_id 가명화, 1년 후 삭제 (PIPA)
```

#### `analysis_report`

```sql
create table analysis_report (
    id char(30) primary key,                            -- rpt_...
    user_id char(30) not null references "user"(id) on delete cascade,
    title varchar(200) not null,
    target_pnus char(19)[] not null,                    -- 분석 대상 필지들
    snapshot jsonb not null,                            -- 시점 고정 분석 결과 (R2 데이터 캐시)
    created_at timestamptz not null default now(),
    version bigint not null default 1
);

create index analysis_report_user_idx on analysis_report(user_id, created_at desc);
```

#### `notification`

```sql
create table notification (
    id char(30) primary key,                            -- ntf_...
    user_id char(30) not null references "user"(id) on delete cascade,
    kind varchar(50) not null,                          -- 'bookmark_listing_changed', 'auction_deadline_approaching'
    payload jsonb not null,
    read_at timestamptz,
    created_at timestamptz not null default now()
);

create index notification_user_unread_idx on notification(user_id, created_at desc)
    where read_at is null;
-- retention: 365일 후 자동 삭제
```

### 5.3 시스템 (횡단 관심사)

#### `audit_log`

```sql
create table audit_log (
    id char(30) primary key,                            -- aud_...
    actor_id char(30),                                  -- 행위자 (system은 null)
    action varchar(50) not null,                        -- 'listing.created', 'user.business_verified'
    resource_kind varchar(30) not null,
    resource_id varchar(50) not null,
    before_state jsonb,
    after_state jsonb,
    correlation_id varchar(30) not null,
    ip_address inet,
    user_agent text,
    created_at timestamptz not null default now()
);

create index audit_log_created_brin_idx on audit_log using brin(created_at);
create index audit_log_resource_idx on audit_log(resource_kind, resource_id, created_at desc);
create index audit_log_actor_idx on audit_log(actor_id, created_at desc) where actor_id is not null;

-- 1년 RDS + 6년 R2 IA archive (V003__retention_jobs.sql에서 archiver 등록)
-- DB role: gongzzang_app_writer는 INSERT만 (UPDATE/DELETE 권한 없음)
```

#### `outbox_event`

```sql
create table outbox_event (
    id char(30) primary key,                            -- evt_...
    aggregate_kind varchar(30) not null,
    aggregate_id varchar(50) not null,
    event_type varchar(50) not null,                    -- 'listing.published', 'user.business_verified'
    payload jsonb not null,
    correlation_id varchar(30) not null,
    created_at timestamptz not null default now(),
    published_at timestamptz                            -- publisher 발송 완료 시각
);

create index outbox_unpublished_idx on outbox_event(created_at)
    where published_at is null;
-- retention: published 후 30일 후 삭제
```

### 5.4 데이터 파이프라인 (어드민 관리)

#### `pipeline_schedule`

```sql
create table pipeline_schedule (
    id char(30) primary key,                            -- pls_...
    pipeline_kind varchar(50) not null unique,          -- 'parcel_sync', 'building_sync', 'realtransaction_daily'
    cron_expression varchar(100) not null,              -- '0 3 1 */3 *'
    enabled boolean not null default true,
    timezone varchar(50) not null default 'Asia/Seoul',
    last_run_at timestamptz,
    next_run_at timestamptz,
    config jsonb not null default '{}',                 -- 파이프라인별 설정 (시도 화이트리스트 등)
    running_lock_acquired_at timestamptz,               -- 동시 실행 방지 (Postgres advisory lock 보조)
    running_worker_id varchar(50),
    updated_at timestamptz not null default now(),
    updated_by char(30) references "user"(id),
    version bigint not null default 1
);

create index pipeline_schedule_next_idx on pipeline_schedule(next_run_at)
    where enabled = true;
```

#### `pipeline_run`

```sql
create table pipeline_run (
    id char(30) primary key,                            -- plr_...
    schedule_id char(30) not null references pipeline_schedule(id),
    started_at timestamptz not null default now(),
    finished_at timestamptz,
    status varchar(20) not null default 'running' check (status in
        ('running', 'success', 'failed', 'skipped_unchanged', 'aborted')),
    items_processed bigint not null default 0,
    items_changed bigint not null default 0,
    output_hashes jsonb not null default '{}',          -- {"sido_11": "abc...", "sido_26": "def..."} (시도별 hash)
    error_message text,
    triggered_by varchar(20) not null check (triggered_by in ('schedule', 'manual', 'event')),
    triggered_by_user char(30) references "user"(id),
    correlation_id varchar(30) not null,
    log_url text,                                        -- Loki 또는 CloudWatch 링크
    steps jsonb not null default '[]'                   -- 단계별 진행 (UI 시각화용)
);

create index pipeline_run_schedule_time_idx on pipeline_run(schedule_id, started_at desc);
create index pipeline_run_running_idx on pipeline_run(started_at) where status = 'running';
```

`steps` JSONB 형식:
```jsonc
[
  {
    "order": 1,
    "name": "fetch_vworld",
    "label": "V-World API fetch",
    "status": "success",  // pending/running/success/failed/skipped
    "started_at": "2026-05-02T03:00:00Z",
    "finished_at": "2026-05-02T03:03:12Z",
    "progress_pct": 100,
    "progress_message": null,
    "metrics": {"items": 4218341}
  },
  {
    "order": 3,
    "name": "shard_by_sido",
    "label": "시도별 분할",
    "status": "running",
    "started_at": "2026-05-02T03:04:18Z",
    "progress_pct": 70,
    "progress_message": "처리 중: 26 부산광역시 (12/17)"
  }
]
```

### 5.5 어드민 운영 (Admin/Operations)

#### `admin_action`

```sql
create table admin_action (
    id char(30) primary key,                            -- ada_...
    admin_id char(30) not null references "user"(id),
    action_kind varchar(50) not null,                   -- 'verify_business', 'approve_listing', 'force_pipeline_run'
    target_kind varchar(30),
    target_id varchar(50),
    payload jsonb not null default '{}',
    correlation_id varchar(30) not null,
    created_at timestamptz not null default now()
);

create index admin_action_admin_idx on admin_action(admin_id, created_at desc);
create index admin_action_target_idx on admin_action(target_kind, target_id);
```

#### `business_verification_queue`

```sql
create table business_verification_queue (
    id char(30) primary key,                            -- bvq_...
    user_id char(30) not null references "user"(id),
    business_number varchar(12) not null,
    submitted_documents jsonb not null,                 -- 사업자등록증 등 R2 key
    status varchar(20) not null default 'pending' check (status in
        ('pending', 'approved', 'rejected', 'needs_more_info')),
    reviewer_id char(30) references "user"(id),
    reviewer_note text,
    submitted_at timestamptz not null default now(),
    reviewed_at timestamptz,
    sla_due_at timestamptz,                             -- SLA (24h)
    version bigint not null default 1                   -- OCC (concurrent admin edit 방어)
);

create index bvq_pending_idx on business_verification_queue(submitted_at)
    where status = 'pending';
create index bvq_user_idx on business_verification_queue(user_id);
```

#### `listing_review_queue`

```sql
create table listing_review_queue (
    id char(30) primary key,                            -- lrq_...
    listing_id char(30) not null references listing(id) on delete cascade,
    submitted_at timestamptz not null default now(),
    auto_check_score int,                                -- 0-100 (룰 기반)
    auto_check_flags jsonb,                              -- ['suspected_duplicate', 'price_anomaly']
    reviewer_id char(30) references "user"(id),
    reviewer_note text,
    decision varchar(20) check (decision in ('approve', 'reject', 'request_changes')),
    decided_at timestamptz,
    sla_due_at timestamptz,                             -- SLA (12h)
    version bigint not null default 1                   -- OCC (concurrent admin edit 방어)
);

create index lrq_pending_idx on listing_review_queue(submitted_at)
    where decision is null;
```

#### `listing_report`

```sql
create table listing_report (
    id char(30) primary key,                            -- lrp_...
    listing_id char(30) not null references listing(id),
    reporter_id char(30) references "user"(id),         -- nullable (익명)
    reason varchar(50) not null check (reason in
        ('fake_listing', 'wrong_price', 'wrong_location', 'inappropriate_content', 'spam', 'other')),
    detail text,
    status varchar(20) not null default 'open' check (status in
        ('open', 'investigating', 'confirmed', 'dismissed')),
    handler_id char(30) references "user"(id),
    handler_note text,
    created_at timestamptz not null default now(),
    resolved_at timestamptz
);

create index listing_report_open_idx on listing_report(created_at desc) where status = 'open';
create index listing_report_listing_idx on listing_report(listing_id);
```

#### `featured_content`

```sql
create table featured_content (
    id char(30) primary key,                            -- fea_... (3-char prefix invariant; was `fc_` in earlier drafts)
    target_kind varchar(30) not null check (target_kind in
        ('listing', 'industrial_complex', 'manufacturer')),
    target_id varchar(50) not null,
    feature_kind varchar(30) not null check (feature_kind in
        ('homepage_featured', 'search_top', 'sponsored_marker', 'newsletter')),
    weight int not null default 1,
    starts_at timestamptz not null,
    ends_at timestamptz not null,
    purchased_by char(30) references "user"(id),        -- Phase 2+ 결제
    impression_count bigint not null default 0,
    click_count bigint not null default 0,
    created_at timestamptz not null default now(),
    constraint featured_content_time_bound_chk check (ends_at > starts_at)
);

-- Note: partial index predicate cannot use now() (PG requires IMMUTABLE).
-- Range scan on (starts_at, ends_at) suffices; queries filter `ends_at > now()` at runtime.
create index featured_active_idx on featured_content(feature_kind, starts_at, ends_at);
```

#### `system_alert`

```sql
create table system_alert (
    id char(30) primary key,                            -- sal_...
    severity varchar(10) not null check (severity in ('info', 'warning', 'error', 'critical')),
    source varchar(50) not null,                        -- 'pipeline.parcel_sync', 'circuit_breaker.vworld'
    title varchar(200) not null,
    detail text,
    metadata jsonb not null default '{}',
    acknowledged_at timestamptz,
    acknowledged_by char(30) references "user"(id),
    resolved_at timestamptz,
    created_at timestamptz not null default now()
);

create index system_alert_unack_idx on system_alert(severity, created_at desc)
    where acknowledged_at is null;
```

### 5.6 합계

총 **18 테이블**:

| BC | 테이블 |
|----|--------|
| Core (RDS 동적) | `user`, `listing`, `listing_photo` |
| Insights | `bookmark_listing`, `bookmark_external`, `search_history`, `analysis_report`, `notification` |
| System | `audit_log`, `outbox_event` |
| Pipeline | `pipeline_schedule`, `pipeline_run` |
| Admin | `admin_action`, `business_verification_queue`, `listing_review_queue`, `listing_report`, `featured_content`, `system_alert` |

---

## 6. DB Role 분리 (V002__db_roles.sql)

```sql
-- 권한 분리 (audit immutable + 최소 권한)

-- 1. 일반 앱 writer (INSERT/UPDATE/DELETE 대부분 테이블)
create role gongzzang_app_writer;
grant connect on database gongzzang to gongzzang_app_writer;
grant usage on schema public to gongzzang_app_writer;
grant select, insert, update, delete on all tables in schema public to gongzzang_app_writer;

-- audit_log는 INSERT만, UPDATE/DELETE 박탈 (immutable)
revoke update, delete on audit_log from gongzzang_app_writer;

-- 2. 읽기 전용 (분석·리포트)
create role gongzzang_app_reader;
grant connect on database gongzzang to gongzzang_app_reader;
grant usage on schema public to gongzzang_app_reader;
grant select on all tables in schema public to gongzzang_app_reader;

-- 3. audit archiver (audit_log SELECT + DELETE만, archive worker 전용)
create role gongzzang_audit_archiver;
grant connect on database gongzzang to gongzzang_audit_archiver;
grant usage on schema public to gongzzang_audit_archiver;
grant select, delete on audit_log to gongzzang_audit_archiver;

-- 실제 사용자 (Pulumi에서 생성)
-- gongzzang_api_user (gongzzang_app_writer 역할 부여) — services/api 가 사용
-- gongzzang_analytics_user (gongzzang_app_reader) — 분석 도구
-- gongzzang_archiver_user (gongzzang_audit_archiver) — services/worker archive job
```

---

## 7. R2 정적 데이터 구조

### 7.1 두 버킷

```
gongzzang-public-data/      공개 정적 데이터 (CDN 배포, 사용자 다운로드)
gongzzang-raw-archive/      외부 API raw 응답 (감사용, 비공개)
```

### 7.2 `gongzzang-public-data` 구조

```
parcels/
  {sido_code}/
    parcels.pmtiles             시도별 PMTiles (서울 ~2GB)
    last_sync.json              마지막 sync 메타 (timestamp, hash, source)

buildings/
  {sido_code}/
    buildings.pmtiles
    last_sync.json

industrial-complexes/
  index.geojson                 전국 1천 개 (작음, 한 파일)
  details/
    {complex_id}.json

real-transactions/
  by-month/
    {year}-{month}.json         예: 2026-05.json (전국 신규)
  by-region/
    {sido_code}/
      {sigungu_code}/
        {year}.json             분석용

court-auctions/
  active.json                   진행 중 경매 (매일 갱신)
  history/
    {year}/{month}.json         종료 경매 (월별 archive)

manufacturers/
  by-industry/
    {ksic_code}.json
  by-region/
    {sido_code}.json

laws/
  index.json                    법령 목록
  texts/
    {law_id}.json               법령 본문 (조항별)
  embeddings/                   (Phase 3+)
    {law_id}.bin

masters/
  administrative-divisions.json
  road-addresses.json
  ksic-codes.json
  zoning-codes.json
  land-use-types.json

listings/
  {listing_id}/
    photos/
      p1.jpg, p1_thumb.jpg, p2.jpg, ...   매물 사진 (presigned URL 업로드)
```

### 7.3 `gongzzang-raw-archive` 구조

```
vworld/
  {date}/                                          예: 2026-05-02/
    {request_id}.json.gz                            V-World 응답 raw
data-go-kr/
  {date}/
    {request_id}.json.gz
korean-law/
  {date}/
    {request_id}.json.gz
nice-identity/                                     (Phase 3+)
  {date}/
    {request_id}.json.gz                            인증 요청 raw (PII 마스킹 후)
```

retention: 7년 (PIPA + ISMS-P + 분쟁 시 증빙). R2 Object Lock immutable.

### 7.4 갱신 전략

| 데이터 | 갱신 주기 | 변경 감지 | shard 단위 |
|--------|---------|----------|---------|
| 필지 PMTiles | 분기 (어드민 조정 가능) | V-World 응답 해시 vs R2 hash | 시도 17개 |
| 건축물 PMTiles | 분기 | data.go.kr `lastUpdtDt` + hash | 시도 17개 |
| 산업단지 | 연 + 이벤트 | 정부 공시 (수동 트리거) | 단일 |
| 실거래 | 일 | API에 `dealYmd` 필터, 신규분 append | 월별 분할 |
| 경매 active | 일 | 사건번호별 갱신일 | 단일 |
| 경매 history | 월 (1일 03:00) | active → history 이전 | 월별 |
| 법령 | 변경 이벤트 | 법제처 webhook 또는 polling | 법령별 |
| 마스터 (행정구역 등) | 분기 | 정부 표준 코드 변경 | 단일 |

### 7.5 Shard 단위 hash 비교

워커 흐름 (시도 17개 예):
```
1. V-World 호출 → 전국 필지 응답
2. 시도별로 분할 → 17 그룹
3. 각 그룹 PMTiles 생성 → 17 hash 계산
4. R2의 sido_11/last_sync.json hash와 비교
   - 같음 → 그 시도 PMTiles 업로드 skip
   - 다름 → 업로드 + last_sync.json 갱신 + Cloudflare CDN purge
5. pipeline_run.output_hashes에 17 hash 모두 기록
```

→ 4천만 필지 중 *서울만 변경*이면 *서울 PMTiles만* 재업로드.

### 7.6 멱등성 (Postgres advisory lock)

```sql
-- 워커 시작 시
select pg_try_advisory_lock(hashtext('pipeline:parcel_sync'));
-- 1 = lock 획득 → pipeline_run INSERT (status='running')
-- 0 = 다른 워커 실행 중 → skip + log
```

`pipeline_schedule.running_lock_acquired_at` + `running_worker_id`로 모니터링 (어드민 UI에서 stuck 워커 감지 + 강제 해제).

---

## 8. Rust 도메인 코드 구조

### 8.1 워크스페이스

```
crates/
├── domain/
│   ├── core/
│   │   ├── user/                    RDS 동적
│   │   ├── listing/                 RDS 동적
│   │   ├── parcel/                  R2 정적 (Reader trait)
│   │   ├── building/                R2 정적
│   │   ├── industrial-complex/      R2 정적
│   │   ├── manufacturer/            R2 정적
│   │   └── shared-kernel/           Pnu, Money, Area, Geometry, AdminDivision 등
│   │
│   ├── market/
│   │   ├── real-transaction/        R2 정적 (read-only)
│   │   ├── court-auction/           R2 정적
│   │   ├── inquiry/                 RDS 동적 (Phase 2+ 자리)
│   │   └── subscription/            RDS 동적 (Phase 2+ 자리)
│   │
│   ├── regulation/
│   │   ├── law/                     R2 정적
│   │   └── regulation/              R2 정적 (자리)
│   │
│   ├── insights/
│   │   ├── bookmark/                RDS 동적 (Listing FK + External polymorphic)
│   │   ├── search-history/          RDS 동적
│   │   ├── analysis-report/         RDS 동적
│   │   └── notification/            RDS 동적
│   │
│   └── audit/
│       └── audit-log/               RDS 동적 (immutable)
│
├── operations/                      신규 — 어드민 운영 도메인
│   ├── admin-action/
│   ├── business-verification/
│   ├── listing-review/
│   ├── listing-report/
│   ├── featured-content/
│   └── system-alert/
│
├── data-pipeline-control/           신규 — 파이프라인 schedule + run
│   ├── schedule/
│   ├── run/
│   └── steps/
│
├── db/                              SQLx + PostGIS Repository (sub-project 5에서 본격 구현)
├── data-clients/                    R2 reader + 외부 API HTTP (sub-project 4에서 본격)
├── geo/, auth/, cache/, observability/, circuit-breaker/, api-types/, embedding/
```

### 8.2 값 객체 (shared-kernel)

```rust
// crates/domain/core/shared-kernel/src/pnu.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Pnu(String);

impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> {
        if s.len() != 19 || !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(PnuError::InvalidFormat);
        }
        // 추가: 시도/시군구 코드 검증
        Ok(Self(s.to_owned()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn sido_code(&self) -> &str { &self.0[0..2] }
    pub fn sigungu_code(&self) -> &str { &self.0[0..5] }
}

// crates/domain/core/shared-kernel/src/money.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct Money(i64); // KRW 단위, 음수 불가능

impl Money {
    pub fn try_new(krw: i64) -> Result<Self, MoneyError> {
        if krw < 0 { return Err(MoneyError::Negative); }
        Ok(Self(krw))
    }
    pub fn krw(&self) -> i64 { self.0 }
}

// 다른 값 객체:
// Area (㎡), BusinessNumber (10자리), BrokerLicense, RoadAddress, JibunAddress,
// Email, PhoneKr, AdminDivision, ListingTitle, Description, ULID 헬퍼 등
```

### 8.3 Aggregate 예시 (Listing)

```rust
// crates/domain/core/listing/src/entity.rs
pub struct Listing {
    pub id: ListingId,
    pub owner_id: UserId,
    pub parcel_pnu: Pnu,
    pub listing_type: ListingType,
    pub transaction_type: TransactionType,
    pub price: Money,
    pub deposit: Option<Money>,
    pub monthly_rent: Option<Money>,
    pub area: Area,
    pub title: ListingTitle,
    pub description: Description,
    pub status: ListingStatus,
    pub contact_visibility: ContactVisibility,
    pub view_count: u64,
    pub bookmark_count: u64,
    pub geom_point: Option<Point>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub version: i64,
}

// 상태 머신
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingStatus {
    Draft,
    PendingReview,
    Active,
    Sold,
    Expired,
    Rejected,
}

impl Listing {
    pub fn submit_for_review(&mut self) -> Result<(), ListingError> {
        match self.status {
            ListingStatus::Draft => {
                self.status = ListingStatus::PendingReview;
                self.version += 1;
                Ok(())
            }
            _ => Err(ListingError::InvalidTransition {
                from: self.status, to: ListingStatus::PendingReview
            })
        }
    }
    // approve, reject, mark_sold, expire 등
}

// crates/domain/core/listing/src/repository.rs
#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn find(&self, id: &ListingId) -> Result<Option<Listing>, RepoError>;
    async fn find_markers_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<ListingMarker>, RepoError>;
    async fn save(&self, listing: &Listing) -> Result<(), RepoError>;
}
```

### 8.4 R2 Reader (Parcel)

```rust
// crates/domain/core/parcel/src/entity.rs
pub struct Parcel {
    pub pnu: Pnu,
    pub admin: AdminDivision,
    pub road_address: Option<RoadAddress>,
    pub jibun_address: JibunAddress,
    pub land_use_type: LandUseType,
    pub area: Area,
    pub official_land_price_per_m2: Option<Money>,
    pub zoning: Zoning,
    pub geom: Polygon,
    pub fetched_at: DateTime<Utc>,
}

// Reader trait (Repository와 다름 — read-only)
#[async_trait::async_trait]
pub trait ParcelReader: Send + Sync {
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError>;
    async fn fetch_markers_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<ParcelMarker>, ReaderError>;
}

// 구현체 (sub-project 4에서):
// crates/data-clients/r2-public-data/src/parcel_reader.rs
// - PMTiles에서 spatial query
// - 시도 코드 추출 → 해당 시도 PMTiles만 fetch
// - moka L1 + Valkey L2 캐시
```

---

## 9. 어드민 UI 통합 원칙 (sub-project 6 미리보기)

### 9.1 9 화면 구조

```
admin-web/
├── /dashboard               전체 헬스 + 알림 + 비용 위젯 + 큐 사이즈
├── /users                   목록·검증 큐 (탭) → /users/{id} (컨텍스트)
├── /listings                목록·검수·신고 (탭) → /listings/{id} (컨텍스트) + /listings/review-queue
├── /pipelines               목록·진행 → /pipelines/{id} (단계별 진행 시각화)
├── /content                 광고/추천 (Phase 2+)
├── /observability           Grafana embed (서비스 맵 / 트레이스 / 메트릭 / 로그 / 에러 — 탭)
├── /audit                   전역 audit 검색 (대상별 audit는 컨텍스트 페이지에)
├── /costs                   비용 (Phase 3+)
└── /settings                Feature flag, 권한, 시스템 설정
```

### 9.2 공유 위젯 (어디든 embed)

| 위젯 | 데이터 소스 | 용도 |
|------|---------|------|
| `AuditLogWidget` | RDS `audit_log` 필터 | 이 대상의 변경 이력 |
| `MetricsWidget` | Grafana API | 이 대상의 메트릭 |
| `AlertsWidget` | RDS `system_alert` | 이 대상 관련 알림 |
| `RelatedActionsWidget` | RDS query + admin_action insert | 운영 액션 (검증/검수/신고처리) |
| `TraceLinkWidget` | Grafana Tempo URL | 이 대상 관련 트레이스 |

### 9.3 컨텍스트 중심 예시 — `/listings/{id}`

한 화면에:
- 매물 본체 (RDS `listing`)
- 매물 사진 (R2 + presigned URL via `listing_photo`)
- 등록자 (UserCard 위젯)
- 위치 정보 (R2 `parcel/...` reader)
- 신고 (`listing_report` 필터)
- audit log (위젯)
- 메트릭 (위젯)
- 운영 액션 (승인/거부/일시정지/추천)

→ 운영자가 *3초 내 판단 + 처리*. 별도 페이지 왕복 X.

---

## 10. 파이프라인 진행 시각화

### 10.1 두 종류 시각화

| 시각화 | 어디 | 데이터 |
|------|------|------|
| **서비스 맵** (이미지의 Maple 식) | Grafana Tempo embed in `/observability` | OTel 트레이스 자동 |
| **파이프라인 진행** (단계별 카드) | 자체 admin-web UI in `/pipelines/{id}` | RDS `pipeline_run.steps` JSONB |
| **분산 추적** (한 요청 호출 체인) | Grafana Tempo embed | OTel 트레이스 |
| **메트릭·로그·에러** | Grafana embed | Prometheus/Loki/Sentry |

### 10.2 파이프라인 단계 시각화 데이터 흐름

```
Worker 실행 시:
1. pipeline_run INSERT (status='running', steps=[])
2. 각 단계 시작 시: steps[i] = {status: 'running', started_at: now, progress_pct: 0}
3. 단계 진행 시: steps[i].progress_pct = N, progress_message = "..."
4. 단계 완료 시: steps[i] = {status: 'success', finished_at: now, progress_pct: 100, metrics: {...}}
5. 다음 단계로
6. 모든 단계 완료 시: pipeline_run.status='success', finished_at=now
   (실패 시: status='failed', error_message=...)

동시에 OTel:
- 각 단계는 tracing::span (Tempo로 전송)
- 어드민 UI는 자체 진행(JSONB) + Grafana 트레이스 둘 다 표시
```

---

## 11. 검증 기준 (Sub-project 2 완료 판정)

### 11.1 결과물

- [ ] **18 RDS 테이블** + 인덱스 + 제약 모두 정의 (V001__init.sql)
- [ ] **DB role 3개** (writer/reader/audit_archiver) 정의 (V002__db_roles.sql)
- [ ] **Rust 값 객체 15개+** 모두 단위 테스트 (Pnu/Money/Area/BusinessNumber/...)
- [ ] **6 Aggregate Entity** 모든 필드 + 상태 머신 + 도메인 메서드
- [ ] **Repository trait** Aggregate별 (구현체는 sub-project 5)
- [ ] **R2 Reader trait** + R2 디렉토리 구조 정의
- [ ] **Operations 도메인 6개** (admin/verification/review/report/featured/alert)
- [ ] **Pipeline control 도메인** (schedule + run + step JSONB schema)
- [ ] **공유 위젯 데이터 계약** 명시 (Rust types + OpenAPI spec preview)
- [ ] **모든 파일 ≤500줄**

### 11.2 자동 검증

- [ ] `cargo check --workspace` 통과
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] `cargo test --workspace` 통과 (단위 테스트 90%+ 도메인 커버리지)
- [ ] `cargo deny check` 통과 (라이선스 + 보안)
- [ ] Biome + markdownlint 통과
- [ ] CI 그린 (모든 job)

### 11.3 SSS 15 검증 추가 통과

- [x] (Q4) 의존성 방향 빌드 실패 — `[lints] workspace = true` + dependency-cruiser
- [x] (Q9) 임의 사용자 활동 재구성 가능 — audit_log 기록
- [x] (Q15) 외부 API raw 1년 후 재현 — `gongzzang-raw-archive` 정의

(Q1, Q7, Q10 등은 후속 sub-project 의존)

### 11.4 사용자 검증

- [ ] 사용자가 spec 검토 후 승인
- [ ] 사용자가 결과물 검토 후 승인 (마이그레이션 + 도메인 코드)

---

## 12. 의존성 + 전제

### 12.1 환경

- Rust 1.83 + Cargo workspace (sub-project 1 완료)
- pnpm + Biome (sub-project 1 완료)
- PostgreSQL 17 + PostGIS 3.5 (Docker Compose 로컬, sub-project 8 인프라 전)
- sqlx CLI (개발자 설치)

### 12.2 외부 결정 보류 (이 sub-project에서 안 정함)

- Pulumi RDS 인스턴스 사양 — sub-project 8
- Cloudflare R2 버킷 실제 생성 — sub-project 8
- 데이터 시드 (개발용) — sub-project 5+
- API endpoint URL 패턴 — sub-project 5

---

## 13. 후속 Sub-projects (의존)

```
SP2 (DB + Core 도메인)  ← 현재
 ↓
 ├─▶ SP3 (인증) — User Aggregate + Zitadel JWT 검증
 ├─▶ SP4 (V-World 통합) — Repository 구현 + R2 Reader 구현
 ├─▶ SP5 (첫 API endpoint) — Axum + utoipa
 ├─▶ SP6 (첫 프론트엔드) — admin-web 9 화면 + 공유 위젯
 ├─▶ SP7 (관측성) — OTel + Grafana embed
 ├─▶ SP8 (인프라) — Pulumi RDS + R2 + Role
 └─▶ SP9 (ETL) — 워커가 pipeline_schedule 따름
```

---

## 14. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| Aggregate 경계 모호 (Bookmark가 polymorphic) | 무결성 깨짐 | Listing은 FK, R2 데이터는 polymorphic — 절충 명시 |
| audit_log 폭증 | RDS 디스크 ↑ | 1년 RDS + 6년 R2 IA archive (월 1회 archiver) |
| `pipeline_run.steps` JSONB 크기 폭증 | RDS 디스크 + 쿼리 느림 | step별 metrics는 *짧게*, 큰 데이터는 Loki 링크 |
| R2 sync 동시 실행 (race condition) | 이상 데이터 | Postgres advisory lock + `running_lock_acquired_at` |
| 외부 API raw 보존 7년 비용 | R2 IA 비용 | 압축 (gzip) + 월별 묶음 |
| sub-project 4 (V-World 통합) 시점에 R2 reader trait 변경 | 재작업 | trait를 *최소*로 — 첫 메서드 2-3개만, 확장은 그때 |
| 어드민 운영 데이터 모델이 sub-project 6 UI와 mismatch | 재마이그레이션 | UI 디자인 (sub-project 6 brainstorming) 시점에 V003 마이그레이션 |

---

## 15. 자체 검토 (이 spec)

### Placeholder 스캔
- [ ] 모든 섹션 채워짐 (TBD/TODO 없음)
- 결정 보류는 § 2.3에 명시적

### 내부 일관성
- [ ] § 5 RDS 테이블 18개 = § 4 도메인 분류와 일치
- [ ] ID prefix 모두 glossary와 일치 (usr_, lst_, ph_, ...)
- [ ] DB role (§ 6) = audit_log 정책 (§ 5.3) 일치

### Scope 검증
- [ ] *데이터 모델 + 도메인 코드*에 한정 (UI/API/외부 통합 제외 명시)
- [ ] 후속 sub-project별 책임 명확

### 모호성
- [ ] R2 정적 vs RDS 동적 분류 명확 (§ 4)
- [ ] Aggregate vs 어드민 운영 모델 분리 (§ 5)

---

## 16. 다음 단계

이 spec이 사용자 승인되면:

1. **writing-plans 스킬 호출** — 18 테이블 + 도메인 코드를 Task별 분해 → implementation plan
2. **subagent-driven-development** — Task별 fresh subagent 실행
3. **검증** — § 11 기준 통과 확인

---

## 17. 참조

- ADR: 0001-0011 (sub-project 1)
- 헌법: → @docs/sss-charter.md
- 글로서리: → @docs/glossary.md
- 컨벤션: → @docs/conventions/
- 데이터 소스: → @docs/data-sources/
