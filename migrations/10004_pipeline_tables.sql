-- V001_04: Data Pipeline 제어 — pipeline_schedule (cron), pipeline_run (steps JSONB) (spec § 5.4)

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
