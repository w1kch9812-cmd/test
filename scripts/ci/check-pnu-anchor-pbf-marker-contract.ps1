[CmdletBinding()]
param(
    [string] $Root = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $scriptRoot = $PSScriptRoot
    if ([string]::IsNullOrWhiteSpace($scriptRoot)) {
        $scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
    }
    $Root = Join-Path $scriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot)) {
    throw "Root does not exist: $resolvedRoot"
}

$contracts = @(
    [pscustomobject]@{
        RelativePath = "AGENTS.md"
        Tokens = @(
            "ADR 0037",
            "Listing PBF design spec",
            "owns parcel geometry",
            "owns listing semantics and Gongzzang-owned listing PBF marker tiles",
            "listing rows must not own canonical marker coordinates",
            "marker request shapes",
            "verification-first",
            "tests, migration smoke, and"
        )
        Forbidden = @(
            "platform-core owns Gongzzang listing price",
            "platform-core owns Gongzzang listing status",
            "platform-core owns Gongzzang listing exposure"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/adr/0037-pnu-anchor-pbf-marker-tiles.md"
        Tokens = @(
            "marker_tile_response_format = MVT_PBF",
            "marker_position_source = PNU_ANCHOR",
            "bbox_marker_runtime_forbidden = true",
            "dropped_marker_success_forbidden = true",
            "Gongzzang remains the SSOT for listing semantics",
            "dynamic PBF generated from listing rows joined to platform-core anchors by PNU",
            "Product-specific listing marker PBF tiles are a Gongzzang market-domain runtime surface",
            "find_listing_marker_tile",
            "parcel_marker_anchor",
            "Active listing saves are rejected",
            "GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
            "approved by the user on",
            "No Gongzzang launch map/listing path may depend on viewport bounds as its public request shape"
        )
        Forbidden = @(
            "platform-core owns Gongzzang listing price",
            "platform-core owns Gongzzang listing status",
            "platform-core owns Gongzzang listing exposure"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/adr/0038-listing-marker-serving-index-filter-mask.md"
        Tokens = @(
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "PNU anchor",
            "marker-counts/listing",
            "marker-masks/listing",
            "browser instant filtering"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md"
        Tokens = @(
            "Gongzzang-owned listing PBF marker tiles",
            "platform-core owns PNU anchors",
            "Gongzzang owns listing semantics",
            "No listing-owned canonical coordinate",
            "No viewport-bounds public marker API",
            "No silent marker drop"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/specs/2026-05-26-listing-marker-serving-index-filter-mask-design.md"
        Tokens = @(
            "listing_marker_projection",
            "filter_hash",
            "base marker tile",
            "browser instant filter",
            "server marker/filter index",
            "optional filter mask"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/plans/2026-05-26-listing-marker-serving-index-filter-mask.md"
        Tokens = @(
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "buildListingMarkerLayerFilter",
            "marker-counts/listing",
            "marker-masks/listing"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/plans/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles.md"
        Tokens = @(
            "Serve Gongzzang-owned active listing marker tiles as MVT/PBF",
            "Successful tiles represent every eligible listing",
            "migrations/30012_parcel_marker_anchor_projection.sql",
            "services/api/src/routes/listing_marker_tiles.rs",
            "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/handoff/2026-05-22-listing-pbf-review-gate.md"
        Tokens = @(
            "Implementation slice verified locally",
            "full project completion not claimed",
            "former `"do not implement yet`" gate is closed",
            "Still Do Not Do",
            "Do not call platform-core databases directly from Gongzzang",
            "If this slice is touched again, re-run the implementation verification checklist"
        )
        Forbidden = @(
            "Runtime listing PBF implementation is still pending",
            "Do not implement the Gongzzang listing PBF endpoint",
            "Do not create the Gongzzang anchor read model migration",
            "Do not switch the frontend to the Gongzzang listing PBF layer",
            "Spec and DB migration approved",
            "implementation verification in progress"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/next-actions.md"
        Tokens = @(
            "local-verification-backed",
            "not a whole-product launch completion claim",
            "handoff/audit verification",
            "platform-core owns PNU anchors; Gongzzang owns listing semantics"
        )
        Forbidden = @(
            "Do not implement the listing PBF endpoint",
            "implementation-approved",
            "Verify the listing PBF endpoint",
            "guardrails before any completion claim"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/roadmap.md"
        Tokens = @(
            "Current supersession",
            "ADR 0037",
            "Gongzzang-owned listing PBF design spec",
            "verification evidence",
            "not a whole-product launch completion claim",
            "handoff/audit verification"
        )
        Forbidden = @(
            "waiting for user review",
            "Do not implement the listing PBF endpoint",
            "implementation-approved",
            "implementation verification in progress"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/superpowers/handoff/2026-05-22-active-goal-completion-audit.md"
        Tokens = @(
            "Active Goal Completion Audit",
            "Completion claim allowed | false",
            "Prompt-To-Artifact Checklist",
            "completion_claim_allowed=false",
            "Do not call update_goal"
        )
    },
    [pscustomobject]@{
        RelativePath = "migrations/30012_parcel_marker_anchor_projection.sql"
        Tokens = @(
            "create table parcel_marker_anchor",
            "anchor_point geometry(Point, 4326) not null",
            "anchor_snapshot_id",
            "source_geometry_checksum_sha256",
            "platform_core_updated_at",
            "parcel_marker_anchor_srid_chk",
            "parcel_marker_anchor_point_gist_idx"
        )
        Forbidden = @(
            "anchor_lng",
            "anchor_lat"
        )
    },
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
    },
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
    },
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
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/map-zoom-policy.ts"
        Tokens = @(
            "GONGZZANG_MAP_ZOOM_POLICY",
            "exactParcelAnchorMinZoom: 12",
            "parcel",
            "minZoom: 14",
            "maxZoom: 22",
            "LISTING_MARKER_RENDER_MIN_ZOOM",
            "LISTING_MARKER_RENDER_MAX_ZOOM"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/marker-tile-style.ts"
        Tokens = @(
            "buildParcelAnchorMarkerLayerRegistration",
            "buildListingMarkerLayerRegistration",
            "buildListingMarkerDeltaLayerRegistration",
            "PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
            "LISTING_MARKER_DELTA_TILE_SOURCE_ID",
            '"source-layer": LISTING_MARKER_TILE_LAYER'
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/components/listings/listing-map.tsx"
        Tokens = @(
            "setupMapboxRuntime",
            "buildListingMarkerLayerFilter",
            "buildListingMarkerServerKey",
            "loadListingMarkerServerState",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            'pushPanel({ kind: "listing", id: listingId, view: "summary" })',
            'pushPanel({ kind: "parcel", id: pnu, view: "summary" })'
        )
        Forbidden = @(
            "new naver.maps.Marker",
            "listing.lat",
            "listing.lng",
            "pinIconHtml",
            "markersRef",
            "bounds_changed",
            "boundsTimerRef",
            "setBounds",
            "getBounds()"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-map-runtime.ts"
        Tokens = @(
            "setupListingMarkerTileLayers",
            "buildListingMarkerLayerRegistration",
            "buildListingMarkerDeltaLayerRegistration",
            "LISTING_MARKER_RENDER_MIN_ZOOM",
            "LISTING_MARKER_RENDER_MAX_ZOOM",
            "buildParcelAnchorMarkerLayerRegistrations",
            "fetchVectorTileManifest",
            "setupMarkerTileLayers"
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/routes.ts"
        Tokens = @(
            "listingMarkerCounts",
            "listingMarkerFilters",
            "listingMarkerDeltasPrefix",
            "listingMarkerDeltaTemplate",
            "listingMarkerMaskTemplate",
            "listingMarkerTombstonesPrefix",
            "listingMarkerTombstoneTemplate",
            "marker-counts/listing",
            "marker-filters/listing",
            "marker-masks/listing",
            "marker-deltas/listing",
            "marker-tombstones/listing"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-marker-filter.ts"
        Tokens = @(
            "buildListingMarkerLayerFilter",
            "listing_type",
            "transaction_type",
            "price_krw",
            "area_m2"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-marker-server-state.ts"
        Tokens = @(
            "buildListingMarkerFilterRequest",
            "buildListingMarkerServerKey",
            "min_area_m2",
            "max_price_krw"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/app/api/proxy/[...path]/route.ts"
        Tokens = @(
            "isBinaryProxyResponse",
            "application/vnd.mapbox-vector-tile",
            "arrayBuffer()",
            "text()"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/api-proxy-route.test.ts"
        Tokens = @(
            "preserves Mapbox vector tile responses as binary",
            "application/vnd.mapbox-vector-tile",
            "arrayBuffer()",
            "map/v1/marker-tiles/listing/14/8780/6345.pbf"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/proxy.ts"
        Tokens = @(
            "API.proxy.listingMarkerTilesPrefix",
            "isLocalHostname",
            "allowLocalHttpMapRuntime",
            "PUBLIC_PATHS",
            "isPublic"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/platform-core-proxy.test.ts"
        Tokens = @(
            "allows Gongzzang listing PBF marker tile proxy without sid",
            "/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1",
            "allows Naver HTTP resources only for local production preview CSP"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/panel/codec.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "PNU_PATTERN",
            "IdPatternViolation"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/components/panels/listing/register.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "idPattern: LISTING_ID_PATTERN"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/listings/schema.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "id: z.string().regex(LISTING_ID_PATTERN)"
        )
        Forbidden = @(
            "geom_point",
            "lat: z.number",
            "lng: z.number"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/map/marker-tile-contract.test.ts"
        Tokens = @(
            "builds the Gongzzang-owned listing marker vector source through same-origin proxy",
            "LISTING_MARKER_TILE_LAYER",
            "LISTING_MARKER_DELTA_TILE_LAYER",
            "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
            "http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41",
            "http://localhost:3900/api/proxy/map/v1/marker-tombstones/listing/14/13970/6344?base_version=41",
            "not.toContain(`"bbox=`")",
            "not.toContain(`"bounds=`")"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/map/marker-tile-style.test.ts"
        Tokens = @(
            "registers Gongzzang listing marker source and circle layer without coordinate inputs",
            "registers Gongzzang listing marker delta source with the listing delta layer",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
            "LISTING_MARKER_DELTA_TILE_SOURCE_ID",
            "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/panel/codec.test.ts"
        Tokens = @(
            "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
            "rejects UUID listing ids because Listing ids are lst-prefixed ULIDs"
        )
    },
    [pscustomobject]@{
        RelativePath = "tests/migrations/test_v001_full.sh"
        Tokens = @(
            "parcel_marker_anchor",
            "parcel_marker_anchor_srid_chk",
            "parcel_marker_anchor_point_gist_idx",
            "must not duplicate anchor_lng/anchor_lat columns",
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "listing_marker_projection_anchor_srid_chk",
            "listing_marker_filter_registry_spec_shape_chk",
            "platform_core_event_inbox",
            "platform_core_event_inbox_anchor_payload_chk",
            "platform_core_event_inbox_pending_idx"
        )
        Forbidden = @(
            "listing.geom_point SRID expected 4326",
            "f_geometry_column='geom_point'"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/frontend/listings-search.md"
        Tokens = @(
            "Listing Marker Serving",
            "listing_marker_projection",
            "browser instant filter",
            "server marker indexes"
        )
    }
)

$checkedFiles = 0
$violations = @()
foreach ($contract in $contracts) {
    $relativePaths = @()
    if ($null -ne $contract.PSObject.Properties["RelativePaths"]) {
        $relativePaths = @($contract.RelativePaths)
    }
    else {
        $relativePaths = @([string] $contract.RelativePath)
    }
    $relativePathLabel = $relativePaths -join ", "
    $contentParts = @()
    foreach ($relativePath in $relativePaths) {
        $path = Join-Path $resolvedRoot ($relativePath -replace "/", "\")
        if (!(Test-Path -LiteralPath $path)) {
            [Console]::Error.WriteLine("missing PNU anchor PBF marker contract file: {0}", $relativePath)
            exit 1
        }

        $checkedFiles += 1
        if (Test-Path -LiteralPath $path -PathType Container) {
            $contentParts += (Get-ChildItem -LiteralPath $path -Recurse -File -Filter "*.rs" |
                    Sort-Object -Property FullName |
                    ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw } |
                    Out-String)
        }
        else {
            $contentParts += Get-Content -LiteralPath $path -Raw
        }
    }
    $content = $contentParts -join "`n"

    foreach ($token in @($contract.Tokens)) {
        if ($content.Contains($token)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "missing token"
            Value = $token
        }
    }

    $forbiddenTokens = @()
    if ($null -ne $contract.PSObject.Properties["Forbidden"]) {
        $forbiddenTokens = @($contract.Forbidden)
    }
    foreach ($token in $forbiddenTokens) {
        if ([string]::IsNullOrEmpty($token) -or !$content.Contains($token)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "forbidden token"
            Value = $token
        }
    }

    $forbiddenPatterns = @()
    if ($null -ne $contract.PSObject.Properties["ForbiddenRegex"]) {
        $forbiddenPatterns = @($contract.ForbiddenRegex)
    }
    foreach ($pattern in $forbiddenPatterns) {
        if ([string]::IsNullOrEmpty($pattern) -or ![regex]::IsMatch($content, $pattern)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "forbidden pattern"
            Value = $pattern
        }
    }
}

if (@($violations).Count -gt 0) {
    foreach ($violation in $violations) {
        [Console]::Error.WriteLine(
            "PNU anchor PBF marker contract {0}: {1}: {2}",
            $violation.Kind,
            $violation.Path,
            $violation.Value
        )
    }
    exit 1
}

Write-Host "pnu-anchor-pbf-marker-contract-ok files=$checkedFiles"
