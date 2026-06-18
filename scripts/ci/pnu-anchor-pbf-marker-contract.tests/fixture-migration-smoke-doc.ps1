    Write-File -Root $Root -RelativePath "tests\migrations\test_v001_full.sh" -Content @'
parcel_marker_anchor
parcel_marker_anchor_srid_chk
parcel_marker_anchor_point_gist_idx
must not duplicate anchor_lng/anchor_lat columns
listing_marker_projection
listing_marker_filter_registry
listing_marker_projection_anchor_srid_chk
listing_marker_filter_registry_spec_shape_chk
platform_core_event_inbox
platform_core_event_inbox_anchor_payload_chk
platform_core_event_inbox_pending_idx
'@
    Write-File -Root $Root -RelativePath "docs\frontend\listings-search.md" -Content @'
Listing Marker Serving
listing_marker_projection
browser instant filter
server marker indexes
'@
