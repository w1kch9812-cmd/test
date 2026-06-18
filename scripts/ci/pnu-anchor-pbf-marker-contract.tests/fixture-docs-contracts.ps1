    Write-File -Root $Root -RelativePath "AGENTS.md" -Content @'
ADR 0037
Listing PBF design spec
`platform-core` owns parcel geometry
`gongzzang` owns listing semantics and Gongzzang-owned listing PBF marker tiles
listing rows must not own canonical marker coordinates
launch marker requests must not use public `bbox`/`bounds` marker request shapes
implementation gate is now verification-first
tests, migration smoke, and guardrails before any completion claim
'@
    Write-File -Root $Root -RelativePath "docs\adr\0037-pnu-anchor-pbf-marker-tiles.md" -Content @'
marker_tile_response_format = MVT_PBF
marker_position_source = PNU_ANCHOR
bbox_marker_runtime_forbidden = true
dropped_marker_success_forbidden = true
Gongzzang remains the SSOT for listing semantics
dynamic PBF generated from listing rows joined to platform-core anchors by PNU
Product-specific listing marker PBF tiles are a Gongzzang market-domain runtime surface
find_listing_marker_tile
parcel_marker_anchor
Active listing saves are rejected
GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
approved by the user on 2026-05-22
No Gongzzang launch map/listing path may depend on viewport bounds as its public request shape
'@
    Write-File -Root $Root -RelativePath "docs\adr\0038-listing-marker-serving-index-filter-mask.md" -Content @'
listing_marker_projection
listing_marker_filter_registry
PNU anchor
marker-counts/listing
marker-masks/listing
browser instant filtering
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\specs\2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md" -Content @'
Gongzzang-owned listing PBF marker tiles
platform-core owns PNU anchors
Gongzzang owns listing semantics
No listing-owned canonical coordinate
No viewport-bounds public marker API
No silent marker drop
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\specs\2026-05-26-listing-marker-serving-index-filter-mask-design.md" -Content @'
listing_marker_projection
filter_hash
base marker tile
browser instant filter
server marker/filter index
optional filter mask
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\plans\2026-05-26-listing-marker-serving-index-filter-mask.md" -Content @'
listing_marker_projection
listing_marker_filter_registry
buildListingMarkerLayerFilter
marker-counts/listing
marker-masks/listing
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\plans\2026-05-22-gongzzang-owned-listing-pbf-marker-tiles.md" -Content @'
Serve Gongzzang-owned active listing marker tiles as MVT/PBF
Successful tiles represent every eligible listing
migrations/30012_parcel_marker_anchor_projection.sql
services/api/src/routes/listing_marker_tiles.rs
scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\handoff\2026-05-22-listing-pbf-review-gate.md" -Content @'
Implementation slice verified locally
full project completion not claimed
former "do not implement yet" gate is closed
Still Do Not Do
Do not call platform-core databases directly from Gongzzang
If this slice is touched again, re-run the implementation verification checklist
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\next-actions.md" -Content @'
local-verification-backed
not a whole-product launch completion claim
handoff/audit verification
platform-core owns PNU anchors; Gongzzang owns listing semantics
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\roadmap.md" -Content @'
Current supersession
ADR 0037
Gongzzang-owned listing PBF design spec
verification evidence
not a whole-product launch completion claim
handoff/audit verification
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\handoff\2026-05-22-active-goal-completion-audit.md" -Content @'
Active Goal Completion Audit
Completion claim allowed | false
Prompt-To-Artifact Checklist
completion_claim_allowed=false
Do not call update_goal
'@
