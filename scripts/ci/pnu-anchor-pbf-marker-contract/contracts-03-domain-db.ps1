$contracts += @(
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_tile.rs"
        Tokens = @(
            "find_listing_marker_tile",
            "parcel_marker_anchor",
            "listing_marker_projection",
            "ST_AsMVTGeom",
            "ST_AsMVT",
            "unanchored_active_count",
            "unprojected_active_count",
            "listing marker tile completeness violation",
            "eligible_count",
            "represented_count"
        )
        Forbidden = @(
            "find_markers_in_bbox",
            "find_card_summaries_in_bbox",
            "LISTING_MARKER_COLUMNS",
            "row_to_marker",
            "ST_MakeEnvelope",
            "geom_point",
            "geom_lng",
            "geom_lat",
            "has_geom",
            "ST_SetSRID",
            "ST_MakePoint"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_delta.rs"
        Tokens = @(
            "find_listing_marker_deltas",
            "listing_marker_delta_log",
            "listing_marker_projection",
            "LISTING_MARKER_DELTA_TILE_LAYER",
            "ST_AsMVTGeom",
            "ST_AsMVT",
            "projection_version",
            "anchor_snapshot_id"
        )
        Forbidden = @(
            "contact",
            "business_number",
            "business_verified"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_tombstone.rs"
        Tokens = @(
            "find_listing_marker_tombstones",
            "listing_marker_tombstone_log",
            "marker_ids",
            "projection_version",
            "anchor_snapshot_id"
        )
        Forbidden = @(
            "price_krw",
            "area_m2",
            "contact",
            "business_number"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_mask.rs"
        Tokens = @(
            "find_listing_marker_mask",
            "listing_marker_projection",
            "ListingMarkerMaskEncoding::Show",
            "marker_id",
            "projection_version",
            "anchor_snapshot_id"
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "listing_lng",
            "listing_lat",
            "geom_point"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_filter_registry.rs"
        Tokens = @(
            "register_listing_marker_filter",
            "resolve_listing_marker_filter",
            "listing_marker_filter_registry",
            "request_count",
            "last_used_at"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/listing/marker_projection.rs"
        Tokens = @(
            "listing_marker_delta_log",
            "listing_marker_tombstone_log",
            "listing_marker_dirty_tile_queue",
            "values (0), (6), (10), (11), (12), (13), (14)",
            "old_public",
            "new_public"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/src/platform_core_anchor.rs"
        Tokens = @(
            "insert_inbox_event",
            "find_inbox_event_payload",
            "find_pending_anchor_import_event_ids",
            "mark_inbox_event_processing",
            "mark_inbox_event_processed",
            "mark_inbox_event_failed",
            "status in ('pending_import', 'processing')",
            "status = 'processing'",
            "processed_at = now()",
            "failed_at = now()",
            "failure_reason",
            "import_anchor_rows",
            "listing_marker_projection"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/tests/listing_marker_tile_integration"
        Tokens = @(
            "listing_marker_tile_represents_every_active_listing_on_same_pnu",
            "listing_marker_save_rejects_active_listing_without_anchor",
            "ListingMarkerTileQuery",
            "ListingMarkerFilter::AllActive",
            "missing PNU anchor",
            "feature_count",
            "aggregate_count",
            "listing_marker_projection_upsert_uses_platform_core_anchor_snapshot",
            "listing_marker_tile_applies_normalized_filter_spec"
        )
    },
    [pscustomobject]@{
        RelativePath = "crates/db/tests/listing_marker_tile_integration/filter_index.rs"
        Tokens = @(
            "listing_marker_filter_registry_round_trips_normalized_filter",
            "listing_marker_mask_returns_show_ids_for_loaded_tile",
            "count_listing_markers",
            "find_listing_marker_mask",
            "ListingMarkerMaskEncoding::Show"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_common.rs"
        Tokens = @(
            "resolve_listing_marker_filter",
            "ALL_ACTIVE_LISTING_MARKER_FILTER_HASH",
            "listing marker filter was not found"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_counts.rs"
        Tokens = @(
            "get_listing_marker_count",
            "ListingMarkerCountsState",
            "marker-counts/listing",
            "count_listing_markers"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_filters.rs"
        Tokens = @(
            "post_listing_marker_filter",
            "ListingMarkerFiltersState",
            "register_listing_marker_filter",
            "filter_hash"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_masks.rs"
        Tokens = @(
            "get_listing_marker_mask",
            "ListingMarkerMasksState",
            "find_listing_marker_mask",
            "listing marker base tile version is stale"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_tombstones.rs"
        Tokens = @(
            "get_listing_marker_tombstones",
            "ListingMarkerTombstonesState",
            "find_listing_marker_tombstones",
            "encoding: `"hide`"",
            "marker_ids",
            "base_version"
        )
        Forbidden = @(
            "bbox",
            "bounds",
            "price_krw",
            "area_m2",
            "contact"
        )
    }
)
