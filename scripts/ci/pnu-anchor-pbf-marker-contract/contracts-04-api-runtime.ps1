$contracts += @(
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_deltas.rs"
        Tokens = @(
            "get_listing_marker_deltas",
            "ListingMarkerDeltasState",
            "find_listing_marker_deltas",
            "LISTING_MARKER_TILE_CONTENT_TYPE",
            "public, max-age=5",
            "base_version"
        )
        Forbidden = @(
            "bbox",
            "bounds",
            "price_krw",
            "area_m2",
            "contact"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/routes/listing_marker_tiles.rs"
        Tokens = @(
            "get_listing_marker_tile",
            "ListingMarkerTilesState",
            "filter_hash is required",
            "listing marker tile cannot be represented truthfully",
            "LISTING_MARKER_TILE_CONTENT_TYPE",
            "public, max-age=30"
        )
        Forbidden = @(
            "bounds",
            "bbox",
            "latitude",
            "longitude"
        )
    },
    [pscustomobject]@{
        RelativePath = "services/api/src/platform_core_anchor_import.rs"
        Tokens = @(
            "platform-core.parcel_marker_anchor_artifact_manifest.v1",
            "platform-core.parcel_marker_anchor_artifact_entry.v1",
            "parse_anchor_manifest",
            "parse_anchor_rows",
            "parse_anchor_entry",
            "source_srid",
            "anchor_srid",
            "EPSG:4326",
            "algorithm_version",
            "source_geometry_checksum_sha256",
            "artifact_row_count",
            "object row_count"
        )
    },
    [pscustomobject]@{
        RelativePaths = @(
            "services/api/src/bin/platform_core_anchor_import.rs",
            "services/api/src/bin/platform_core_anchor_import"
        )
        Tokens = @(
            "PlatformCoreAnchorImport",
            "parse_anchor_manifest",
            "parse_anchor_rows",
            "verify_size_bytes",
            "object.size_bytes",
            "verify_sha256",
            "object.checksum_sha256",
            "ChecksumMismatch",
            "SizeMismatch",
            "PLATFORM_CORE_EVENT_ID",
            "mark_inbox_event_processing",
            "mark_inbox_event_processed",
            "mark_inbox_event_failed",
            "truncate_failure_reason",
            "pg_try_advisory_lock",
            "pg_advisory_unlock",
            "event_import_lock_key",
            "InboxEventAlreadyLocked",
            "ImportSource::EventPayload",
            "event_artifact_config_from_payload",
            "find_inbox_event_payload",
            "artifact_manifest_url",
            "artifact_checksum_sha256",
            "fetch_artifact_bytes",
            "resolve_artifact_object_url",
            "ImportSource::PendingInboxBatch",
            "PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT",
            "run_pending_inbox_batch",
            "find_pending_anchor_import_event_ids",
            "BatchImportFailed"
        )
    },
    [pscustomobject]@{
        RelativePaths = @(
            "services/api/src/main.rs",
            "services/api/src/app.rs"
        )
        Tokens = @(
            "pub mod listing_marker_tiles",
            "pub mod listing_marker_counts",
            "pub mod listing_marker_filters",
            "pub mod listing_marker_masks",
            "pub mod listing_marker_tombstones",
            "pub mod listing_marker_deltas",
            "/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
            "/map/v1/marker-counts/listing",
            "/map/v1/marker-filters/listing",
            "/map/v1/marker-masks/listing/:z/:x/:y",
            "/map/v1/marker-tombstones/listing/:z/:x/:y",
            "/map/v1/marker-deltas/listing/:z/:x/:y_pbf",
            "get(routes::listing_marker_tiles::get_listing_marker_tile)",
            "ListingMarkerTilesState",
            "ListingMarkerMasksState",
            "ListingMarkerTombstonesState",
            "ListingMarkerDeltasState"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/identity/patterns.ts"
        Tokens = @(
            "PNU_PATTERN",
            "LISTING_ID_PATTERN",
            "lst_[0-9A-HJKMNP-TV-Z]{26}"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/marker-tile-contract.ts"
        Tokens = @(
            "LISTING_MARKER_TILE_LAYER",
            "LISTING_MARKER_DELTA_TILE_LAYER",
            "LISTING_MARKER_TILE_ENDPOINT_TEMPLATE",
            "buildListingMarkerDeltaTileSource",
            "buildListingMarkerTombstoneUrl",
            "createListingMarkerOverlayState",
            "ALL_ACTIVE_MARKER_FILTER_HASH",
            "buildListingMarkerTileSource",
            "assertSupportedListingFilterHash",
            "resolveSameOrigin",
            "browser origin is required for listing marker tile URLs",
            "lst_filter_v1_[0-9a-f]{64}"
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/vector-tile-manifest.ts"
        Tokens = @(
            "PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER",
            "PARCEL_ANCHOR_VECTOR_TILE_LAYER",
            "render_min_zoom",
            "render_max_zoom",
            "tiles_url_template",
            "fetchVectorTileManifest",
            "buildVectorTileSource"
        )
        Forbidden = @(
            "bounds=",
            "bbox="
        )
    }
)
