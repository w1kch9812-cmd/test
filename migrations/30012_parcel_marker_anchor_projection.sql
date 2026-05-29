-- Gongzzang-local projection of platform-core parcel marker anchors.
--
-- This table is not a product-owned coordinate source. It is a read model copied from
-- platform-core so Gongzzang can generate listing marker MVT/PBF tiles without calling
-- platform-core on every tile request.

create table parcel_marker_anchor (
    pnu char(19) primary key,
    anchor_point geometry(Point, 4326) not null,
    algorithm varchar(64) not null,
    algorithm_version varchar(32) not null,
    anchor_snapshot_id varchar(128) not null,
    source_geometry_version varchar(128) not null,
    source_geometry_checksum_sha256 char(64) not null,
    platform_core_updated_at timestamptz not null,
    synced_at timestamptz not null default now(),
    constraint parcel_marker_anchor_pnu_format_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint parcel_marker_anchor_srid_chk
        check (ST_SRID(anchor_point) = 4326),
    constraint parcel_marker_anchor_algorithm_chk
        check (algorithm ~ '^[a-z][a-z0-9_]*$'),
    constraint parcel_marker_anchor_checksum_chk
        check (source_geometry_checksum_sha256 ~ '^[0-9a-f]{64}$')
);

create index parcel_marker_anchor_point_gist_idx
    on parcel_marker_anchor using gist(anchor_point);

create index parcel_marker_anchor_snapshot_idx
    on parcel_marker_anchor(anchor_snapshot_id, platform_core_updated_at desc);
