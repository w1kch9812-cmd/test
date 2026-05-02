-- V001_05: Admin/Operations — admin_action, queues, reports, featured, alerts (spec § 5.5)

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
    sla_due_at timestamptz                              -- SLA (24h)
);

create index bvq_pending_idx on business_verification_queue(submitted_at)
    where status = 'pending';
create index bvq_user_idx on business_verification_queue(user_id);

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
    sla_due_at timestamptz                              -- SLA (12h)
);

create index lrq_pending_idx on listing_review_queue(submitted_at)
    where decision is null;

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

create table featured_content (
    id char(30) primary key,                            -- fc_...
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
    created_at timestamptz not null default now()
);

-- Note: partial index cannot reference now() (PG requires IMMUTABLE).
-- Range scan on (starts_at, ends_at) suffices; queries filter `ends_at > now()` at runtime.
create index featured_active_idx on featured_content(feature_kind, starts_at, ends_at);

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
