Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-pnu-anchor-pbf-marker-contract.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-pnu-anchor-pbf-marker-contract-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    if (!$Text.Contains($Expected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function Write-ContractFiles {
    param([string] $Root)

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
    Write-File -Root $Root -RelativePath "apps\web\lib\identity\patterns.ts" -Content @'
PNU_PATTERN
LISTING_ID_PATTERN
lst_[0-9A-HJKMNP-TV-Z]{26}
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-contract.ts" -Content @'
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_DELTA_TILE_LAYER
LISTING_MARKER_TILE_ENDPOINT_TEMPLATE
buildListingMarkerDeltaTileSource
buildListingMarkerTombstoneUrl
createListingMarkerOverlayState
ALL_ACTIVE_MARKER_FILTER_HASH
buildListingMarkerTileSource
assertSupportedListingFilterHash
resolveSameOrigin
browser origin is required for listing marker tile URLs
lst_filter_v1_[0-9a-f]{64}
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\vector-tile-manifest.ts" -Content @'
PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER
PARCEL_ANCHOR_VECTOR_TILE_LAYER
render_min_zoom
render_max_zoom
tiles_url_template
fetchVectorTileManifest
buildVectorTileSource
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\map-zoom-policy.ts" -Content @'
GONGZZANG_MAP_ZOOM_POLICY
exactParcelAnchorMinZoom: 12
parcel
minZoom: 14
maxZoom: 22
LISTING_MARKER_RENDER_MIN_ZOOM
LISTING_MARKER_RENDER_MAX_ZOOM
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-style.ts" -Content @'
buildParcelAnchorMarkerLayerRegistration
buildListingMarkerLayerRegistration
buildListingMarkerDeltaLayerRegistration
PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
LISTING_MARKER_DELTA_TILE_SOURCE_ID
"source-layer": LISTING_MARKER_TILE_LAYER
'@
    Write-File -Root $Root -RelativePath "apps\web\components\listings\listing-map.tsx" -Content @'
setupMapboxRuntime
buildListingMarkerLayerFilter
buildListingMarkerServerKey
loadListingMarkerServerState
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
pushPanel({ kind: "listing", id: listingId, view: "summary" })
pushPanel({ kind: "parcel", id: pnu, view: "summary" })
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-map-runtime.ts" -Content @'
setupListingMarkerTileLayers
buildListingMarkerLayerRegistration
buildListingMarkerDeltaLayerRegistration
LISTING_MARKER_RENDER_MIN_ZOOM
LISTING_MARKER_RENDER_MAX_ZOOM
buildParcelAnchorMarkerLayerRegistrations
fetchVectorTileManifest
setupMarkerTileLayers
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\routes.ts" -Content @'
listingMarkerCounts
listingMarkerFilters
listingMarkerDeltasPrefix
listingMarkerDeltaTemplate
listingMarkerMaskTemplate
listingMarkerTombstonesPrefix
listingMarkerTombstoneTemplate
marker-counts/listing
marker-filters/listing
marker-masks/listing
marker-deltas/listing
marker-tombstones/listing
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-marker-filter.ts" -Content @'
buildListingMarkerLayerFilter
listing_type
transaction_type
price_krw
area_m2
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-marker-server-state.ts" -Content @'
buildListingMarkerFilterRequest
buildListingMarkerServerKey
min_area_m2
max_price_krw
'@
    Write-File -Root $Root -RelativePath "apps\web\app\api\proxy\[...path]\route.ts" -Content @'
isBinaryProxyResponse
application/vnd.mapbox-vector-tile
arrayBuffer()
text()
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\api-proxy-route.test.ts" -Content @'
preserves Mapbox vector tile responses as binary
application/vnd.mapbox-vector-tile
arrayBuffer()
map/v1/marker-tiles/listing/14/8780/6345.pbf
'@
    Write-File -Root $Root -RelativePath "apps\web\proxy.ts" -Content @'
API.proxy.listingMarkerTilesPrefix
isLocalHostname
allowLocalHttpMapRuntime
PUBLIC_PATHS
isPublic
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-proxy.test.ts" -Content @'
allows Gongzzang listing PBF marker tile proxy without sid
/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1
allows Naver HTTP resources only for local production preview CSP
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\panel\codec.ts" -Content @'
LISTING_ID_PATTERN
PNU_PATTERN
IdPatternViolation
'@
    Write-File -Root $Root -RelativePath "apps\web\components\panels\listing\register.ts" -Content @'
LISTING_ID_PATTERN
idPattern: LISTING_ID_PATTERN
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\listings\schema.ts" -Content @'
LISTING_ID_PATTERN
id: z.string().regex(LISTING_ID_PATTERN)
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\map\marker-tile-contract.test.ts" -Content @'
builds the Gongzzang-owned listing marker vector source through same-origin proxy
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_DELTA_TILE_LAYER
http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41
http://localhost:3900/api/proxy/map/v1/marker-tombstones/listing/14/13970/6344?base_version=41
not.toContain("bbox=")
not.toContain("bounds=")
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\map\marker-tile-style.test.ts" -Content @'
registers Gongzzang listing marker source and circle layer without coordinate inputs
registers Gongzzang listing marker delta source with the listing delta layer
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
LISTING_MARKER_DELTA_TILE_SOURCE_ID
http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\panel\codec.test.ts" -Content @'
lst_01HXY3NK0Z9F6S1B2C3D4E5F6G
rejects UUID listing ids because Listing ids are lst-prefixed ULIDs
'@
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
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-ContractFiles -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean PNU anchor PBF marker contract check exit code mismatch"
Assert-Contains $clean.Output "pnu-anchor-pbf-marker-contract-ok"

$missingAnchorInboxMigrationRoot = Join-Path $TempRoot "missing-anchor-inbox-migration"
Write-ContractFiles -Root $missingAnchorInboxMigrationRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorInboxMigrationRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql") -Force
$missingAnchorInboxMigration = Invoke-Checker -Root $missingAnchorInboxMigrationRoot
Assert-Equals $missingAnchorInboxMigration.ExitCode 1 "Missing anchor inbox migration contract check exit code mismatch"
Assert-Contains $missingAnchorInboxMigration.Output "missing PNU anchor PBF marker contract file: migrations/30016_platform_core_event_inbox_anchor_import.sql"

$missingAnchorImportRepositoryRoot = Join-Path $TempRoot "missing-anchor-import-repository"
Write-ContractFiles -Root $missingAnchorImportRepositoryRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImportRepositoryRoot "crates\db\src\platform_core_anchor.rs") -Force
$missingAnchorImportRepository = Invoke-Checker -Root $missingAnchorImportRepositoryRoot
Assert-Equals $missingAnchorImportRepository.ExitCode 1 "Missing anchor import repository contract check exit code mismatch"
Assert-Contains $missingAnchorImportRepository.Output "missing PNU anchor PBF marker contract file: crates/db/src/platform_core_anchor.rs"

$weakAnchorImportRepositoryRoot = Join-Path $TempRoot "weak-anchor-import-repository"
Write-ContractFiles -Root $weakAnchorImportRepositoryRoot
Write-File -Root $weakAnchorImportRepositoryRoot -RelativePath "crates\db\src\platform_core_anchor.rs" -Content @'
insert_inbox_event
import_anchor_rows
'@
$weakAnchorImportRepository = Invoke-Checker -Root $weakAnchorImportRepositoryRoot
Assert-Equals $weakAnchorImportRepository.ExitCode 1 "Weak anchor import repository contract check exit code mismatch"
Assert-Contains $weakAnchorImportRepository.Output "missing token"

$missingAnchorImporterRoot = Join-Path $TempRoot "missing-anchor-importer"
Write-ContractFiles -Root $missingAnchorImporterRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImporterRoot "services\api\src\platform_core_anchor_import.rs") -Force
$missingAnchorImporter = Invoke-Checker -Root $missingAnchorImporterRoot
Assert-Equals $missingAnchorImporter.ExitCode 1 "Missing anchor importer parser contract check exit code mismatch"
Assert-Contains $missingAnchorImporter.Output "missing PNU anchor PBF marker contract file: services/api/src/platform_core_anchor_import.rs"

$weakAnchorImporterRoot = Join-Path $TempRoot "weak-anchor-importer"
Write-ContractFiles -Root $weakAnchorImporterRoot
Write-File -Root $weakAnchorImporterRoot -RelativePath "services\api\src\platform_core_anchor_import.rs" -Content @'
parse_anchor_manifest
parse_anchor_rows
parse_anchor_entry
EPSG:4326
'@
$weakAnchorImporter = Invoke-Checker -Root $weakAnchorImporterRoot
Assert-Equals $weakAnchorImporter.ExitCode 1 "Weak anchor importer parser contract check exit code mismatch"
Assert-Contains $weakAnchorImporter.Output "missing token"

$missingAnchorImporterBinRoot = Join-Path $TempRoot "missing-anchor-importer-bin"
Write-ContractFiles -Root $missingAnchorImporterBinRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImporterBinRoot "services\api\src\bin\platform_core_anchor_import.rs") -Force
$missingAnchorImporterBin = Invoke-Checker -Root $missingAnchorImporterBinRoot
Assert-Equals $missingAnchorImporterBin.ExitCode 1 "Missing anchor importer binary contract check exit code mismatch"
Assert-Contains $missingAnchorImporterBin.Output "missing PNU anchor PBF marker contract file: services/api/src/bin/platform_core_anchor_import.rs"

$weakAnchorImporterBinRoot = Join-Path $TempRoot "weak-anchor-importer-bin"
Write-ContractFiles -Root $weakAnchorImporterBinRoot
Write-File -Root $weakAnchorImporterBinRoot -RelativePath "services\api\src\bin\platform_core_anchor_import.rs" -Content @'
PlatformCoreAnchorImport
parse_anchor_manifest
parse_anchor_rows
'@
$weakAnchorImporterBin = Invoke-Checker -Root $weakAnchorImporterBinRoot
Assert-Equals $weakAnchorImporterBin.ExitCode 1 "Weak anchor importer binary contract check exit code mismatch"
Assert-Contains $weakAnchorImporterBin.Output "missing token"

$missingRoot = Join-Path $TempRoot "missing"
New-Item -ItemType Directory -Force -Path $missingRoot | Out-Null
$missing = Invoke-Checker -Root $missingRoot
Assert-Equals $missing.ExitCode 1 "Missing contract file check exit code mismatch"
Assert-Contains $missing.Output "missing PNU anchor PBF marker contract file"

$weakRoot = Join-Path $TempRoot "weak-listing-source"
Write-ContractFiles -Root $weakRoot
Write-File -Root $weakRoot -RelativePath "apps\web\lib\map\marker-tile-contract.ts" -Content @'
response_format: z.literal("mvt_pbf")
position_source: z.literal("pnu_anchor")
bbox_marker_runtime_forbidden: z.literal(true)
dropped_marker_success_forbidden: z.literal(true)
PARCEL_ANCHOR_MARKER_TILE_LAYER
ALL_ACTIVE_MARKER_FILTER_HASH
buildMarkerTileSource
'@
$weak = Invoke-Checker -Root $weakRoot
Assert-Equals $weak.ExitCode 1 "Missing listing tile source token check exit code mismatch"
Assert-Contains $weak.Output "missing token"

$bboxRoot = Join-Path $TempRoot "bbox"
Write-ContractFiles -Root $bboxRoot
Add-Content -LiteralPath (Join-Path $bboxRoot "apps\web\lib\map\marker-tile-contract.ts") -Value "bbox="
$bbox = Invoke-Checker -Root $bboxRoot
Assert-Equals $bbox.ExitCode 1 "Forbidden bbox marker contract check exit code mismatch"
Assert-Contains $bbox.Output "forbidden token"

$legacyDbRoot = Join-Path $TempRoot "legacy-db"
Write-ContractFiles -Root $legacyDbRoot
Add-Content -LiteralPath (Join-Path $legacyDbRoot "crates\db\src\listing\marker_tile.rs") -Value "ST_MakeEnvelope"
$legacyDb = Invoke-Checker -Root $legacyDbRoot
Assert-Equals $legacyDb.ExitCode 1 "Legacy bbox DB query check exit code mismatch"
Assert-Contains $legacyDb.Output "forbidden token"

$unanchoredRoot = Join-Path $TempRoot "missing-unanchored-readiness"
Write-ContractFiles -Root $unanchoredRoot
Write-File -Root $unanchoredRoot -RelativePath "crates\db\src\listing\marker_tile.rs" -Content @'
find_listing_marker_tile
parcel_marker_anchor
ST_AsMVTGeom
ST_AsMVT
listing marker tile completeness violation
eligible_count
represented_count
'@
$unanchored = Invoke-Checker -Root $unanchoredRoot
Assert-Equals $unanchored.ExitCode 1 "Missing unanchored readiness check exit code mismatch"
Assert-Contains $unanchored.Output "missing token"

$missingAnchorInboxSmokeRoot = Join-Path $TempRoot "missing-anchor-inbox-migration-smoke"
Write-ContractFiles -Root $missingAnchorInboxSmokeRoot
Write-File -Root $missingAnchorInboxSmokeRoot -RelativePath "tests\migrations\test_v001_full.sh" -Content @'
parcel_marker_anchor
parcel_marker_anchor_srid_chk
parcel_marker_anchor_point_gist_idx
must not duplicate anchor_lng/anchor_lat columns
listing_marker_projection
listing_marker_filter_registry
listing_marker_projection_anchor_srid_chk
listing_marker_filter_registry_spec_shape_chk
'@
$missingAnchorInboxSmoke = Invoke-Checker -Root $missingAnchorInboxSmokeRoot
Assert-Equals $missingAnchorInboxSmoke.ExitCode 1 "Missing anchor inbox migration smoke check exit code mismatch"
Assert-Contains $missingAnchorInboxSmoke.Output "missing token"

$staleGateRoot = Join-Path $TempRoot "stale-review-gate"
Write-ContractFiles -Root $staleGateRoot
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\next-actions.md") -Value "implementation-approved"
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\roadmap.md") -Value "waiting for user review"
$staleGate = Invoke-Checker -Root $staleGateRoot
Assert-Equals $staleGate.ExitCode 1 "Stale review-gate wording check exit code mismatch"
Assert-Contains $staleGate.Output "forbidden token"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-pnu-anchor-pbf-marker-contract-tests-ok"
