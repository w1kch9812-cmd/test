    Write-File -Root $Root -RelativePath "crates\domain\core\listing\src\repository\mod.rs" -Content @'
find_listing_marker_tile
ALL_ACTIVE_LISTING_MARKER_FILTER_HASH
find_listing_marker_mask
find_listing_marker_tombstones
find_listing_marker_deltas
'@
    Write-File -Root $Root -RelativePath "crates\domain\core\listing\src\repository\marker_tile.rs" -Content @'
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_DELTA_TILE_LAYER
LISTING_MARKER_TILE_EXACT_MIN_ZOOM
LISTING_MARKER_TILE_CONTENT_TYPE
ListingMarkerFilter
ListingMarkerTileQuery
ListingMarkerTile
ListingMarkerMaskQuery
ListingMarkerMask
ListingMarkerTombstones
ListingMarkerDeltas
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_tile.rs" -Content @'
find_listing_marker_tile
parcel_marker_anchor
listing_marker_projection
ST_AsMVTGeom
ST_AsMVT
unanchored_active_count
unprojected_active_count
listing marker tile completeness violation
eligible_count
represented_count
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_delta.rs" -Content @'
find_listing_marker_deltas
listing_marker_delta_log
listing_marker_projection
LISTING_MARKER_DELTA_TILE_LAYER
ST_AsMVTGeom
ST_AsMVT
projection_version
anchor_snapshot_id
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_tombstone.rs" -Content @'
find_listing_marker_tombstones
listing_marker_tombstone_log
marker_ids
projection_version
anchor_snapshot_id
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_mask.rs" -Content @'
find_listing_marker_mask
listing_marker_projection
ListingMarkerMaskEncoding::Show
marker_id
projection_version
anchor_snapshot_id
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_filter_registry.rs" -Content @'
register_listing_marker_filter
resolve_listing_marker_filter
listing_marker_filter_registry
request_count
last_used_at
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing\marker_projection.rs" -Content @'
listing_marker_delta_log
listing_marker_tombstone_log
listing_marker_dirty_tile_queue
values (0), (6), (10), (11), (12), (13), (14)
old_public
new_public
'@
    Write-File -Root $Root -RelativePath "crates\db\src\platform_core_anchor.rs" -Content @'
insert_inbox_event
find_inbox_event_payload
find_pending_anchor_import_event_ids
mark_inbox_event_processing
mark_inbox_event_processed
mark_inbox_event_failed
status in ('pending_import', 'processing')
status = 'processing'
processed_at = now()
failed_at = now()
failure_reason
import_anchor_rows
listing_marker_projection
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\listing_marker_tile_integration\tiles.rs" -Content @'
listing_marker_tile_represents_every_active_listing_on_same_pnu
listing_marker_save_rejects_active_listing_without_anchor
ListingMarkerTileQuery
ListingMarkerFilter::AllActive
missing PNU anchor
feature_count
aggregate_count
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\listing_marker_tile_integration\projection.rs" -Content @'
listing_marker_projection_upsert_uses_platform_core_anchor_snapshot
listing_marker_tile_applies_normalized_filter_spec
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\listing_marker_tile_integration\filter_index.rs" -Content @'
listing_marker_filter_registry_round_trips_normalized_filter
listing_marker_mask_returns_show_ids_for_loaded_tile
count_listing_markers
find_listing_marker_mask
ListingMarkerMaskEncoding::Show
'@
