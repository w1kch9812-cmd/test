-- V001_02: Insights BC RDS 동적 — bookmarks (listing FK + external polymorphic), search_history, analysis_report, notification (spec § 5.2)

create table bookmark_listing (
    user_id char(30) not null references "user"(id) on delete cascade,
    listing_id char(30) not null references listing(id) on delete cascade,
    note text,
    created_at timestamptz not null default now(),
    primary key (user_id, listing_id)
);

create index bookmark_listing_user_idx on bookmark_listing(user_id, created_at desc);

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

create table search_history (
    id char(30) primary key,                            -- sh_...
    user_id char(30) references "user"(id),             -- nullable (비로그인)
    query text not null,
    filters jsonb not null default '{}',
    result_count int not null,
    correlation_id varchar(30) not null,
    created_at timestamptz not null default now()
);

create index search_history_user_time_brin_idx on search_history using brin(created_at);
-- retention: 90일 후 user_id 가명화, 1년 후 삭제 (PIPA)

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
