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
            "fails the request when active listings are missing anchors",
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
        RelativePath = "crates/domain/core/listing/src/repository.rs"
        Tokens = @(
            "find_listing_marker_tile",
            "LISTING_MARKER_TILE_LAYER",
            "ALL_ACTIVE_LISTING_MARKER_FILTER_HASH",
            "LISTING_MARKER_TILE_CONTENT_TYPE",
            "ListingMarkerFilter",
            "ListingMarkerTileQuery",
            "ListingMarkerTile"
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
        RelativePath = "crates/db/src/listing.rs"
        Tokens = @(
            "find_listing_marker_tile",
            "parcel_marker_anchor",
            "ST_AsMVTGeom",
            "ST_AsMVT",
            "unanchored_active_count",
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
        RelativePath = "crates/db/tests/listing_marker_tile_integration.rs"
        Tokens = @(
            "listing_marker_tile_represents_every_active_listing_on_same_pnu",
            "listing_marker_tile_rejects_active_listing_without_anchor",
            "ListingMarkerTileQuery",
            "ListingMarkerFilter::AllActive",
            "unanchored_active_count=1",
            "feature_count",
            "aggregate_count"
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
        RelativePath = "services/api/src/main.rs"
        Tokens = @(
            "pub mod listing_marker_tiles",
            "/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
            "get(routes::listing_marker_tiles::get_listing_marker_tile)",
            "ListingMarkerTilesState"
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
            'response_format: z.literal("mvt_pbf")',
            'position_source: z.literal("pnu_anchor")',
            'bbox_marker_runtime_forbidden: z.literal(true)',
            'dropped_marker_success_forbidden: z.literal(true)',
            "PARCEL_ANCHOR_MARKER_TILE_LAYER",
            "LISTING_MARKER_TILE_LAYER",
            "LISTING_MARKER_TILE_ENDPOINT_TEMPLATE",
            "ALL_ACTIVE_MARKER_FILTER_HASH",
            "buildMarkerTileSource",
            "buildListingMarkerTileSource",
            "resolveSameOrigin",
            "browser origin is required for listing marker tile URLs"
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/marker-tile-style.ts"
        Tokens = @(
            "buildParcelAnchorMarkerLayerRegistration",
            "buildListingMarkerLayerRegistration",
            "PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
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
            "setupListingMarkerTileLayers",
            "buildListingMarkerLayerRegistration",
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
            "map/v1/marker-tiles/listing/0/0/0.pbf"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/proxy.ts"
        Tokens = @(
            "/api/proxy/map/v1/marker-tiles/listing",
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
            "/api/proxy/map/v1/marker-tiles/listing/0/0/0.pbf?filter_hash=all-active-v1",
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
            "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
            "not.toContain(`"bbox=`")",
            "not.toContain(`"bounds=`")"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/map/marker-tile-style.test.ts"
        Tokens = @(
            "registers Gongzzang listing marker source and circle layer without coordinate inputs",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
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
            "must not duplicate anchor_lng/anchor_lat columns"
        )
        Forbidden = @(
            "listing.geom_point SRID expected 4326",
            "f_geometry_column='geom_point'"
        )
    }
)

$checkedFiles = 0
$violations = @()
foreach ($contract in $contracts) {
    $relativePath = [string] $contract.RelativePath
    $path = Join-Path $resolvedRoot ($relativePath -replace "/", "\")
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        [Console]::Error.WriteLine("missing PNU anchor PBF marker contract file: {0}", $relativePath)
        exit 1
    }

    $checkedFiles += 1
    $content = Get-Content -LiteralPath $path -Raw

    foreach ($token in @($contract.Tokens)) {
        if ($content.Contains($token)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePath
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
            Path = $relativePath
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
            Path = $relativePath
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
