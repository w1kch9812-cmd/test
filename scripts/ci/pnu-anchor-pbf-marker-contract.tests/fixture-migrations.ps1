    Write-File -Root $Root -RelativePath "migrations\30012_parcel_marker_anchor_projection.sql" -Content @'
create table parcel_marker_anchor
anchor_point geometry(Point, 4326) not null
anchor_snapshot_id
source_geometry_checksum_sha256
platform_core_updated_at
parcel_marker_anchor_srid_chk
parcel_marker_anchor_point_gist_idx
'@
    Write-File -Root $Root -RelativePath "migrations\30013_listing_marker_projection.sql" -Content @'
create table listing_marker_projection
anchor_point geometry(Point, 4326) not null
listing_marker_projection_anchor_srid_chk
listing_marker_projection_z14_tile_idx
source_geometry_checksum_sha256
'@
    Write-File -Root $Root -RelativePath "migrations\30014_listing_marker_filter_registry.sql" -Content @'
create table listing_marker_filter_registry
listing_marker_filter_registry_hash_chk
listing_marker_filter_registry_spec_shape_chk
all-active-v1
'@
    Write-File -Root $Root -RelativePath "migrations\30016_platform_core_event_inbox_anchor_import.sql" -Content @'
alter table parcel_marker_anchor
    alter column algorithm_version type varchar(128);

create table platform_core_event_inbox (
    event_id uuid primary key,
    event_type varchar(128) not null,
    scope varchar(32) not null,
    effect varchar(64) not null,
    status varchar(32) not null,
    payload jsonb not null,
    anchor_snapshot_id varchar(128),
    source_geometry_version varchar(128),
    received_at timestamptz not null default now(),
    processed_at timestamptz,
    failed_at timestamptz,
    failure_reason text,
    constraint platform_core_event_inbox_scope_chk
        check (scope = 'catalog'),
    constraint platform_core_event_inbox_status_chk
        check (status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')),
    constraint platform_core_event_inbox_effect_chk
        check (effect in ('invalidate_catalog_cache', 'enqueue_anchor_projection_import')),
    constraint platform_core_event_inbox_anchor_payload_chk
        check (
            event_type <> 'catalog.parcel_marker_anchor.snapshot.published.v1'
            or (
                anchor_snapshot_id is not null
                and source_geometry_version is not null
                and effect = 'enqueue_anchor_projection_import'
            )
        )
);

create index platform_core_event_inbox_pending_idx
    on platform_core_event_inbox(event_type, received_at)
    where status = 'pending_import';

create index platform_core_event_inbox_anchor_snapshot_idx
    on platform_core_event_inbox(anchor_snapshot_id)
    where anchor_snapshot_id is not null;
'@
    Write-File -Root $Root -RelativePath "migrations\30017_listing_marker_overlay_and_dirty_queue.sql" -Content @'
create table listing_marker_tombstone_log
create table listing_marker_delta_log
create table listing_marker_dirty_tile_queue
expires_at
listing_marker_dirty_tile_pending_once_idx
status in ('pending', 'processing', 'done', 'failed')
'@
