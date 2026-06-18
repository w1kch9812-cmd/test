    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_common.rs" -Content @'
resolve_listing_marker_filter
ALL_ACTIVE_LISTING_MARKER_FILTER_HASH
listing marker filter was not found
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_counts.rs" -Content @'
get_listing_marker_count
ListingMarkerCountsState
marker-counts/listing
count_listing_markers
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_filters.rs" -Content @'
post_listing_marker_filter
ListingMarkerFiltersState
register_listing_marker_filter
filter_hash
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_masks.rs" -Content @'
get_listing_marker_mask
ListingMarkerMasksState
find_listing_marker_mask
listing marker base tile version is stale
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_tombstones.rs" -Content @'
get_listing_marker_tombstones
ListingMarkerTombstonesState
find_listing_marker_tombstones
encoding: "hide"
marker_ids
base_version
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_deltas.rs" -Content @'
get_listing_marker_deltas
ListingMarkerDeltasState
find_listing_marker_deltas
LISTING_MARKER_TILE_CONTENT_TYPE
public, max-age=5
base_version
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_tiles.rs" -Content @'
get_listing_marker_tile
ListingMarkerTilesState
filter_hash is required
listing marker tile cannot be represented truthfully
LISTING_MARKER_TILE_CONTENT_TYPE
public, max-age=30
'@
    Write-File -Root $Root -RelativePath "services\api\src\platform_core_anchor_import.rs" -Content @'
platform-core.parcel_marker_anchor_artifact_manifest.v1
platform-core.parcel_marker_anchor_artifact_entry.v1
parse_anchor_manifest
parse_anchor_rows
parse_anchor_entry
source_srid
anchor_srid
EPSG:4326
algorithm_version
source_geometry_checksum_sha256
artifact_row_count
object row_count
'@
    Write-File -Root $Root -RelativePath "services\api\src\bin\platform_core_anchor_import.rs" -Content @'
PlatformCoreAnchorImport
parse_anchor_manifest
parse_anchor_rows
verify_size_bytes
object.size_bytes
verify_sha256
object.checksum_sha256
PLATFORM_CORE_EVENT_ID
mark_inbox_event_processing
mark_inbox_event_processed
mark_inbox_event_failed
truncate_failure_reason
InboxEventAlreadyLocked
ImportSource::EventPayload
ImportSource::PendingInboxBatch
PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT
run_pending_inbox_batch
find_pending_anchor_import_event_ids
BatchImportFailed
'@
    Write-File -Root $Root -RelativePath "services\api\src\bin\platform_core_anchor_import\source.rs" -Content @'
event_artifact_config_from_payload
find_inbox_event_payload
artifact_manifest_url
artifact_checksum_sha256
fetch_artifact_bytes
resolve_artifact_object_url
'@
    Write-File -Root $Root -RelativePath "services\api\src\bin\platform_core_anchor_import\error.rs" -Content @'
ChecksumMismatch
SizeMismatch
'@
    Write-File -Root $Root -RelativePath "services\api\src\bin\platform_core_anchor_import\lock.rs" -Content @'
pg_try_advisory_lock
pg_advisory_unlock
event_import_lock_key
'@
    Write-File -Root $Root -RelativePath "services\api\src\main.rs" -Content @'
pub mod listing_marker_tiles
pub mod listing_marker_counts
pub mod listing_marker_filters
pub mod listing_marker_masks
pub mod listing_marker_tombstones
pub mod listing_marker_deltas
'@
    Write-File -Root $Root -RelativePath "services\api\src\app.rs" -Content @'
/map/v1/marker-tiles/listing/:z/:x/:y_pbf
/map/v1/marker-counts/listing
/map/v1/marker-filters/listing
/map/v1/marker-masks/listing/:z/:x/:y
/map/v1/marker-tombstones/listing/:z/:x/:y
/map/v1/marker-deltas/listing/:z/:x/:y_pbf
get(routes::listing_marker_tiles::get_listing_marker_tile)
ListingMarkerTilesState
ListingMarkerMasksState
ListingMarkerTombstonesState
ListingMarkerDeltasState
'@
