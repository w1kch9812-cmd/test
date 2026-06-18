$contracts += @(
    [pscustomobject]@{
        RelativePath = "migrations/30013_listing_marker_projection.sql"
        Tokens = @(
            "create table listing_marker_projection",
            "anchor_point geometry(Point, 4326) not null",
            "listing_marker_projection_anchor_srid_chk",
            "listing_marker_projection_z14_tile_idx",
            "source_geometry_checksum_sha256"
        )
        Forbidden = @(
            "listing_lng",
            "listing_lat",
            "geom_point"
        )
    },
    [pscustomobject]@{
        RelativePath = "migrations/30014_listing_marker_filter_registry.sql"
        Tokens = @(
            "create table listing_marker_filter_registry",
            "listing_marker_filter_registry_hash_chk",
            "listing_marker_filter_registry_spec_shape_chk",
            "all-active-v1"
        )
    },
    [pscustomobject]@{
        RelativePath = "migrations/30016_platform_core_event_inbox_anchor_import.sql"
        Tokens = @(
            "alter table parcel_marker_anchor",
            "alter column algorithm_version type varchar(128)",
            "create table platform_core_event_inbox",
            "event_id uuid primary key",
            "payload jsonb not null",
            "anchor_snapshot_id varchar(128)",
            "source_geometry_version varchar(128)",
            "status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')",
            "platform_core_event_inbox_anchor_payload_chk",
            "platform_core_event_inbox_pending_idx",
            "platform_core_event_inbox_anchor_snapshot_idx"
        )
    },
    [pscustomobject]@{
        RelativePath = "migrations/30017_listing_marker_overlay_and_dirty_queue.sql"
        Tokens = @(
            "create table listing_marker_tombstone_log",
            "create table listing_marker_delta_log",
            "create table listing_marker_dirty_tile_queue",
            "expires_at",
            "listing_marker_dirty_tile_pending_once_idx",
            "status in ('pending', 'processing', 'done', 'failed')"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/domain/core/listing/src/repository"
        Tokens = @(
            "find_listing_marker_tile",
            "LISTING_MARKER_TILE_LAYER",
            "LISTING_MARKER_DELTA_TILE_LAYER",
            "LISTING_MARKER_TILE_EXACT_MIN_ZOOM",
            "ALL_ACTIVE_LISTING_MARKER_FILTER_HASH",
            "LISTING_MARKER_TILE_CONTENT_TYPE",
            "ListingMarkerFilter",
            "ListingMarkerTileQuery",
            "ListingMarkerTile",
            "find_listing_marker_mask",
            "find_listing_marker_tombstones",
            "find_listing_marker_deltas",
            "ListingMarkerMaskQuery",
            "ListingMarkerMask",
            "ListingMarkerTombstones",
            "ListingMarkerDeltas"
        )
        Forbidden = @(
            "find_markers_in_bbox",
            "find_card_summaries_in_bbox",
            "BoundingBox",
            "Result<Vec<ListingMarker>",
            "pub geom: PointSrid"
        )
    }
)
