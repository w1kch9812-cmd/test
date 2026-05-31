-- Listing marker overlay and dirty-tile serving state.
--
-- These tables are Gongzzang-owned serving projections. They do not own canonical
-- marker coordinates; all positions are still derived from platform-core PNU anchors
-- through listing_marker_projection.

create table listing_marker_tombstone_log (
    tombstone_id bigserial primary key,
    marker_id varchar(64) not null,
    listing_id char(30) not null references listing(id) on delete cascade,
    pnu char(19) not null,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    projection_version bigint not null,
    anchor_snapshot_id varchar(128) not null,
    reason varchar(64) not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null default now() + interval '15 minutes',
    constraint listing_marker_tombstone_marker_id_chk
        check (marker_id ~ '^lm_lst_[0-9A-Z]{26}$'),
    constraint listing_marker_tombstone_pnu_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_tombstone_z14_x_chk
        check (z14_tile_x >= 0 and z14_tile_x < 16384),
    constraint listing_marker_tombstone_z14_y_chk
        check (z14_tile_y >= 0 and z14_tile_y < 16384),
    constraint listing_marker_tombstone_projection_version_chk
        check (projection_version >= 1),
    constraint listing_marker_tombstone_reason_chk
        check (reason in ('deleted', 'withdrawn', 'sold', 'expired', 'private', 'rejected'))
);

create unique index listing_marker_tombstone_once_idx
    on listing_marker_tombstone_log(marker_id, projection_version, reason);

create index listing_marker_tombstone_active_tile_idx
    on listing_marker_tombstone_log(z14_tile_x, z14_tile_y, expires_at);

create table listing_marker_delta_log (
    delta_id bigserial primary key,
    marker_id varchar(64) not null,
    listing_id char(30) not null references listing(id) on delete cascade,
    pnu char(19) not null,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    projection_version bigint not null,
    anchor_snapshot_id varchar(128) not null,
    change_kind varchar(64) not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null default now() + interval '5 minutes',
    constraint listing_marker_delta_marker_id_chk
        check (marker_id ~ '^lm_lst_[0-9A-Z]{26}$'),
    constraint listing_marker_delta_pnu_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_delta_z14_x_chk
        check (z14_tile_x >= 0 and z14_tile_x < 16384),
    constraint listing_marker_delta_z14_y_chk
        check (z14_tile_y >= 0 and z14_tile_y < 16384),
    constraint listing_marker_delta_projection_version_chk
        check (projection_version >= 1),
    constraint listing_marker_delta_change_kind_chk
        check (change_kind in ('created_public', 'updated_public', 'became_public'))
);

create unique index listing_marker_delta_once_idx
    on listing_marker_delta_log(marker_id, projection_version, change_kind);

create index listing_marker_delta_active_tile_idx
    on listing_marker_delta_log(z14_tile_x, z14_tile_y, expires_at);

create table listing_marker_dirty_tile_queue (
    dirty_tile_id bigserial primary key,
    layer varchar(64) not null default 'listing',
    tile_z integer not null,
    tile_x integer not null,
    tile_y integer not null,
    reason varchar(64) not null,
    status varchar(32) not null default 'pending',
    priority integer not null default 100,
    attempts integer not null default 0,
    first_seen_at timestamptz not null default now(),
    next_attempt_at timestamptz not null default now(),
    last_error text,
    constraint listing_marker_dirty_layer_chk
        check (layer = 'listing'),
    constraint listing_marker_dirty_tile_z_chk
        check (tile_z >= 0 and tile_z <= 22),
    constraint listing_marker_dirty_tile_x_chk
        check (tile_x >= 0),
    constraint listing_marker_dirty_tile_y_chk
        check (tile_y >= 0),
    constraint listing_marker_dirty_reason_chk
        check (reason in ('delta', 'tombstone', 'projection_update', 'anchor_snapshot')),
    constraint listing_marker_dirty_status_chk
        check (status in ('pending', 'processing', 'done', 'failed')),
    constraint listing_marker_dirty_attempts_chk
        check (attempts >= 0)
);

create unique index listing_marker_dirty_tile_pending_once_idx
    on listing_marker_dirty_tile_queue(layer, tile_z, tile_x, tile_y, reason)
    where status in ('pending', 'processing');

create index listing_marker_dirty_tile_due_idx
    on listing_marker_dirty_tile_queue(priority asc, next_attempt_at asc, first_seen_at asc)
    where status = 'pending';
