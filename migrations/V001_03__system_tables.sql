-- V001_03: System (감사·이벤트 분배) — audit_log, outbox_event (spec § 5.3)
-- audit_log immutable 트리거는 V002에서 부착됨 (writer의 UPDATE/DELETE 박탈)

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
