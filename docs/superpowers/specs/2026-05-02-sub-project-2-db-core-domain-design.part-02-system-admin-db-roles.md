# Sub-project 2 DB Core Domain Design - Part 02: System, Pipeline, Admin Tables, And DB Roles

Parent index: [Sub-project 2 DB Core Domain Design](./2026-05-02-sub-project-2-db-core-domain-design.md).

## Part Context

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
