-- Registry for Gongzzang listing marker filter hashes.
--
-- A filter hash is not reversible. This registry stores the canonical normalized filter payload
-- so tile/count/mask routes can resolve a stable hash without guessing from request strings.

create table listing_marker_filter_registry (
    filter_hash varchar(96) primary key,
    spec jsonb not null,
    created_at timestamptz not null default now(),
    last_used_at timestamptz not null default now(),
    request_count bigint not null default 1,
    constraint listing_marker_filter_registry_hash_chk
        check (
            filter_hash = 'all-active-v1'
            or filter_hash ~ '^lst_filter_v1_[0-9a-f]{64}$'
        ),
    constraint listing_marker_filter_registry_request_count_chk
        check (request_count >= 1),
    constraint listing_marker_filter_registry_spec_shape_chk
        check (
            jsonb_typeof(spec) = 'object'
            and jsonb_typeof(spec -> 'types') = 'array'
            and jsonb_typeof(spec -> 'transactions') = 'array'
            and spec ? 'min_area_m2'
            and spec ? 'max_area_m2'
            and spec ? 'min_price_krw'
            and spec ? 'max_price_krw'
        )
);

create index listing_marker_filter_registry_last_used_idx
    on listing_marker_filter_registry(last_used_at desc);

insert into listing_marker_filter_registry (filter_hash, spec)
values (
    'all-active-v1',
    jsonb_build_object(
        'types', jsonb_build_array(),
        'transactions', jsonb_build_array(),
        'min_area_m2', null,
        'max_area_m2', null,
        'min_price_krw', null,
        'max_price_krw', null
    )
);
