-- Gongzzang listing marker map-serving projection.
--
-- This table is not the listing source of truth and does not make Gongzzang the owner of
-- canonical coordinates. It is a read model derived from `listing` semantics and the
-- platform-core-owned `parcel_marker_anchor` projection.

create table listing_marker_projection (
    marker_id varchar(64) primary key,
    listing_id char(30) not null unique references listing(id) on delete cascade,
    pnu char(19) not null,
    anchor_point geometry(Point, 4326) not null,
    anchor_snapshot_id varchar(128) not null,
    source_geometry_version varchar(128) not null,
    source_geometry_checksum_sha256 char(64) not null,
    source_listing_version bigint not null,
    projection_version bigint not null default 1,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    listing_status varchar(32) not null,
    visibility_scope varchar(32) not null default 'public',
    listing_type varchar(40) not null,
    transaction_type varchar(32) not null,
    price_krw bigint not null,
    area_m2 numeric(12, 2) not null,
    rank_score integer not null default 0,
    listing_updated_at timestamptz not null,
    updated_at timestamptz not null default now(),
    constraint listing_marker_projection_marker_id_chk
        check (marker_id ~ '^lm_lst_[0-9A-Z]{26}$'),
    constraint listing_marker_projection_pnu_format_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_projection_anchor_srid_chk
        check (ST_SRID(anchor_point) = 4326),
    constraint listing_marker_projection_checksum_chk
        check (source_geometry_checksum_sha256 ~ '^[0-9a-f]{64}$'),
    constraint listing_marker_projection_version_positive_chk
        check (source_listing_version >= 1 and projection_version >= 1),
    constraint listing_marker_projection_z14_x_chk
        check (z14_tile_x >= 0 and z14_tile_x < 16384),
    constraint listing_marker_projection_z14_y_chk
        check (z14_tile_y >= 0 and z14_tile_y < 16384),
    constraint listing_marker_projection_scope_chk
        check (visibility_scope in ('public', 'authenticated', 'owner_private')),
    constraint listing_marker_projection_status_chk
        check (listing_status in ('draft', 'pending_review', 'active', 'sold', 'expired', 'rejected')),
    constraint listing_marker_projection_type_chk
        check (listing_type in (
            'factory',
            'warehouse',
            'office',
            'knowledge_industry_center',
            'industrial_land',
            'logistics_center'
        )),
    constraint listing_marker_projection_transaction_chk
        check (transaction_type in ('sale', 'monthly_rent', 'jeonse')),
    constraint listing_marker_projection_price_positive_chk
        check (price_krw > 0),
    constraint listing_marker_projection_area_positive_chk
        check (area_m2 > 0)
);

create index listing_marker_projection_anchor_gist_idx
    on listing_marker_projection using gist(anchor_point);

create index listing_marker_projection_z14_tile_idx
    on listing_marker_projection(z14_tile_x, z14_tile_y, listing_status, visibility_scope);

create index listing_marker_projection_pnu_idx
    on listing_marker_projection(pnu);

create index listing_marker_projection_anchor_snapshot_idx
    on listing_marker_projection(anchor_snapshot_id, source_geometry_version);

create index listing_marker_projection_type_tx_idx
    on listing_marker_projection(listing_type, transaction_type)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_price_idx
    on listing_marker_projection(price_krw)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_area_idx
    on listing_marker_projection(area_m2)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_version_idx
    on listing_marker_projection(projection_version desc, updated_at desc);
