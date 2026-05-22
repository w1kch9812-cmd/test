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
fails the request when active listings are missing anchors
GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
approved by the user on 2026-05-22
No Gongzzang launch map/listing path may depend on viewport bounds as its public request shape
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\specs\2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md" -Content @'
Gongzzang-owned listing PBF marker tiles
platform-core owns PNU anchors
Gongzzang owns listing semantics
No listing-owned canonical coordinate
No viewport-bounds public marker API
No silent marker drop
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
    Write-File -Root $Root -RelativePath "crates\domain\core\listing\src\repository.rs" -Content @'
find_listing_marker_tile
LISTING_MARKER_TILE_LAYER
ALL_ACTIVE_LISTING_MARKER_FILTER_HASH
LISTING_MARKER_TILE_CONTENT_TYPE
ListingMarkerFilter
ListingMarkerTileQuery
ListingMarkerTile
'@
    Write-File -Root $Root -RelativePath "crates\db\src\listing.rs" -Content @'
find_listing_marker_tile
parcel_marker_anchor
ST_AsMVTGeom
ST_AsMVT
unanchored_active_count
listing marker tile completeness violation
eligible_count
represented_count
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\listing_marker_tile_integration.rs" -Content @'
listing_marker_tile_represents_every_active_listing_on_same_pnu
listing_marker_tile_rejects_active_listing_without_anchor
ListingMarkerTileQuery
ListingMarkerFilter::AllActive
unanchored_active_count=1
feature_count
aggregate_count
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_tiles.rs" -Content @'
get_listing_marker_tile
ListingMarkerTilesState
filter_hash is required
listing marker tile cannot be represented truthfully
LISTING_MARKER_TILE_CONTENT_TYPE
public, max-age=30
'@
    Write-File -Root $Root -RelativePath "services\api\src\main.rs" -Content @'
pub mod listing_marker_tiles
/map/v1/marker-tiles/listing/:z/:x/:y_pbf
get(routes::listing_marker_tiles::get_listing_marker_tile)
ListingMarkerTilesState
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\identity\patterns.ts" -Content @'
PNU_PATTERN
LISTING_ID_PATTERN
lst_[0-9A-HJKMNP-TV-Z]{26}
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-contract.ts" -Content @'
response_format: z.literal("mvt_pbf")
position_source: z.literal("pnu_anchor")
bbox_marker_runtime_forbidden: z.literal(true)
dropped_marker_success_forbidden: z.literal(true)
PARCEL_ANCHOR_MARKER_TILE_LAYER
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_TILE_ENDPOINT_TEMPLATE
ALL_ACTIVE_MARKER_FILTER_HASH
buildMarkerTileSource
buildListingMarkerTileSource
resolveSameOrigin
browser origin is required for listing marker tile URLs
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-style.ts" -Content @'
buildParcelAnchorMarkerLayerRegistration
buildListingMarkerLayerRegistration
PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
"source-layer": LISTING_MARKER_TILE_LAYER
'@
    Write-File -Root $Root -RelativePath "apps\web\components\listings\listing-map.tsx" -Content @'
setupListingMarkerTileLayers
buildListingMarkerLayerRegistration
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
pushPanel({ kind: "listing", id: listingId, view: "summary" })
pushPanel({ kind: "parcel", id: pnu, view: "summary" })
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
map/v1/marker-tiles/listing/0/0/0.pbf
'@
    Write-File -Root $Root -RelativePath "apps\web\proxy.ts" -Content @'
/api/proxy/map/v1/marker-tiles/listing
isLocalHostname
allowLocalHttpMapRuntime
PUBLIC_PATHS
isPublic
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-proxy.test.ts" -Content @'
allows Gongzzang listing PBF marker tile proxy without sid
/api/proxy/map/v1/marker-tiles/listing/0/0/0.pbf?filter_hash=all-active-v1
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
http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
not.toContain("bbox=")
not.toContain("bounds=")
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\map\marker-tile-style.test.ts" -Content @'
registers Gongzzang listing marker source and circle layer without coordinate inputs
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
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
'@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-ContractFiles -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean PNU anchor PBF marker contract check exit code mismatch"
Assert-Contains $clean.Output "pnu-anchor-pbf-marker-contract-ok"

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
Add-Content -LiteralPath (Join-Path $legacyDbRoot "crates\db\src\listing.rs") -Value "ST_MakeEnvelope"
$legacyDb = Invoke-Checker -Root $legacyDbRoot
Assert-Equals $legacyDb.ExitCode 1 "Legacy bbox DB query check exit code mismatch"
Assert-Contains $legacyDb.Output "forbidden token"

$unanchoredRoot = Join-Path $TempRoot "missing-unanchored-readiness"
Write-ContractFiles -Root $unanchoredRoot
Write-File -Root $unanchoredRoot -RelativePath "crates\db\src\listing.rs" -Content @'
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

$staleGateRoot = Join-Path $TempRoot "stale-review-gate"
Write-ContractFiles -Root $staleGateRoot
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\next-actions.md") -Value "implementation-approved"
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\roadmap.md") -Value "waiting for user review"
$staleGate = Invoke-Checker -Root $staleGateRoot
Assert-Equals $staleGate.ExitCode 1 "Stale review-gate wording check exit code mismatch"
Assert-Contains $staleGate.Output "forbidden token"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-pnu-anchor-pbf-marker-contract-tests-ok"
