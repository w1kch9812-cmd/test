Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-traffic-auth-policy-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-traffic-auth-policy-registry-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker {
    param(
        [string] $Root,
        [switch] $IncludeProductionEdge
    )
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    if ($IncludeProductionEdge) {
        $arguments += "-IncludeProductionEdge"
    }
    $output = & $PowerShellExe @arguments 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)
    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function New-DataExposurePolicyJson {
    param([string] $AllowedDataClass)
    return @"
"data_exposure_policy": {
  "exposure_class": "public_derived",
  "client_confidentiality_claim": "none",
  "raw_record_access": "forbidden",
  "bulk_export": "forbidden",
  "allowed_data_classes": ["$AllowedDataClass"],
  "forbidden_data_classes": [
    "raw_listing_detail",
    "private_listing",
    "business_verified_listing_detail",
    "contact_data",
    "raw_platform_core_catalog",
    "bulk_listing_export"
  ]
}
"@
}

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitDataExposurePolicy,
        [switch] $AllowRawListingDetail,
        [switch] $OmitGeneratedExposureMetadata,
        [switch] $OmitGeneratedAuthRatePolicies,
        [switch] $OmitGeneratedPageRoutePolicies,
        [switch] $OmitAuthRoutePolicies,
        [switch] $OmitPageRoutePolicies,
        [switch] $AllowAdminListingPageRole,
        [switch] $OmitApiProxyRoutePolicies,
        [switch] $OmitRouteRateProfiles,
        [switch] $OmitAuthenticatedApiProxyRateProfile,
        [switch] $OmitApiProxyExposureGate,
        [switch] $OmitPrivilegedRequiredRoles,
        [switch] $OmitBackendRoutePolicies,
        [switch] $OmitBackendRateProfile,
        [switch] $OmitBackendProtectedAuthLayer,
        [switch] $OmitGeneratedBackendRolePolicies,
        [switch] $OmitBackendAuthorizationLayer,
        [switch] $OmitGeneratedEdgePolicy,
        [switch] $OmitAwsWafEdgeManifest,
        [switch] $OmitPulumiWafConsumer,
        [switch] $OmitPulumiCliPackage,
        [switch] $OmitPulumiLocalPreviewStack,
        [switch] $PollutePulumiLocalPreviewStack,
        [switch] $OmitPulumiWafAssociation
    )

    $tileExposure = if ($OmitDataExposurePolicy) {
        ""
    } else {
        $allowed = if ($AllowRawListingDetail) { "raw_listing_detail" } else { "derived_marker_tile" }
        ",`n      $(New-DataExposurePolicyJson -AllowedDataClass $allowed)"
    }
    $countExposure = if ($OmitDataExposurePolicy) {
        ""
    } else {
        ",`n      $(New-DataExposurePolicyJson -AllowedDataClass "aggregate_count")"
    }
    $filterExposure = if ($OmitDataExposurePolicy) {
        ""
    } else {
        ",`n      $(New-DataExposurePolicyJson -AllowedDataClass "opaque_filter_hash")"
    }
    $maskExposure = if ($OmitDataExposurePolicy) {
        ""
    } else {
        ",`n      $(New-DataExposurePolicyJson -AllowedDataClass "marker_id_mask")"
    }
    $authRoutePolicies = if ($OmitAuthRoutePolicies) {
        ""
    } else {
        @'
  "auth_route_policies": [
    {
      "id": "gongzzang.auth.login",
      "path_source": "API.auth.login",
      "methods": ["POST"],
      "rate_policy": {
        "key_prefix": "auth:login",
        "key_strategy": "client_ip",
        "limit": 5,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    },
    {
      "id": "gongzzang.auth.callback",
      "path_source": "API.auth.callback",
      "methods": ["GET"],
      "rate_policy": {
        "key_prefix": "auth:callback",
        "key_strategy": "client_ip",
        "limit": 10,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    },
    {
      "id": "gongzzang.auth.refresh",
      "path_source": "API.auth.refresh",
      "methods": ["POST"],
      "rate_policy": {
        "key_prefix": "auth:refresh",
        "key_strategy": "session_or_anon",
        "limit": 30,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    }
  ],
'@
    }
    $pageRoutePolicies = if ($OmitPageRoutePolicies) {
        ""
    } else {
        $listingPageRoles = if ($AllowAdminListingPageRole) { '["Broker", "Admin"]' } else { '["Broker"]' }
        $pageRoutePoliciesTemplate = @'
  "page_route_policies": [
    {
      "id": "gongzzang.page.admin",
      "path_kind": "prefix",
      "path": "/admin",
      "required_roles": ["Admin", "Broker", "Operator"]
    },
    {
      "id": "gongzzang.page.listing_create",
      "path_kind": "exact",
      "path_source": "ROUTES.listings.new",
      "required_roles": PLACEHOLDER_LISTING_PAGE_ROLES
    },
    {
      "id": "gongzzang.page.listing_edit",
      "path_kind": "prefix_suffix",
      "prefix_source": "ROUTES.listings.index",
      "suffix": "/edit",
      "required_roles": PLACEHOLDER_LISTING_PAGE_ROLES
    }
  ],
'@
        $pageRoutePoliciesTemplate.Replace("PLACEHOLDER_LISTING_PAGE_ROLES", $listingPageRoles)
    }
    $privilegedRequiredRoles = if ($OmitPrivilegedRequiredRoles) { "" } else { ', "required_roles": ["Broker"]' }
    $backendAuthenticatedReadRateProfile = if ($OmitBackendRateProfile) { "" } else { ', "rate_profile": "api_proxy.authenticated_read"' }
    $backendPrivilegedWriteRateProfile = if ($OmitBackendRateProfile) { "" } else { ', "rate_profile": "api_proxy.privileged_write"' }
    $routeRateProfiles = if ($OmitRouteRateProfiles) {
        ""
    } else {
        @'
  "route_rate_profiles": [
    {
      "id": "api_proxy.authenticated_read",
      "key_prefix": "api-proxy:authenticated-read",
      "key_strategy": "session_sub",
      "limit": 240,
      "window_seconds": 60,
      "problem_type": "proxy/too-many-requests"
    },
    {
      "id": "api_proxy.authenticated_write",
      "key_prefix": "api-proxy:authenticated-write",
      "key_strategy": "session_sub",
      "limit": 120,
      "window_seconds": 60,
      "problem_type": "proxy/too-many-requests"
    },
    {
      "id": "api_proxy.privileged_write",
      "key_prefix": "api-proxy:privileged-write",
      "key_strategy": "session_sub",
      "limit": 60,
      "window_seconds": 60,
      "problem_type": "proxy/too-many-requests"
    }
  ],
'@
    }
    $authenticatedReadRateProfile = if ($OmitAuthenticatedApiProxyRateProfile) {
        ""
    } else {
        ', "rate_profile": "api_proxy.authenticated_read"'
    }
    $privilegedWriteRateProfile = ', "rate_profile": "api_proxy.privileged_write"'
    $apiProxyRoutePolicies = if ($OmitApiProxyRoutePolicies) {
        ""
    } else {
        @"
  "api_proxy_route_policies": [
    {
      "id": "gongzzang.api_proxy.public_marker_tiles",
      "target_path_kind": "template",
      "target_path": "map/v1/marker-tiles/listing/:z/:x/:y_pbf",
      "methods": ["GET"],
      "exposure_class": "public_derived"
    },
    {
      "id": "gongzzang.api_proxy.listing_photo_read_delete",
      "target_path_kind": "template",
      "target_path": "listings/:listing_id/photos/:photo_id",
      "methods": ["GET"],
      "exposure_class": "authenticated_user"$authenticatedReadRateProfile
    },
    {
      "id": "gongzzang.api_proxy.listings_collection_create",
      "target_path_kind": "exact",
      "target_path": "listings",
      "methods": ["POST"],
      "exposure_class": "privileged"$privilegedRequiredRoles$privilegedWriteRateProfile
    },
    {
      "id": "gongzzang.api_proxy.listing_detail_update",
      "target_path_kind": "template",
      "target_path": "listings/:id",
      "methods": ["PATCH"],
      "exposure_class": "privileged"$privilegedRequiredRoles$privilegedWriteRateProfile
    }
  ],
"@
    }
    $backendRoutePolicies = if ($OmitBackendRoutePolicies) {
        ""
    } else {
        @"
  "backend_route_policies": [
    {
      "id": "gongzzang.backend.public_marker_tiles",
      "path": "/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
      "methods": ["GET"],
      "router_group": "public_marker",
      "exposure_class": "public_derived",
      "auth_policy": "anonymous_public"
    },
    {
      "id": "gongzzang.backend.listing_create",
      "path": "/listings",
      "methods": ["POST"],
      "router_group": "protected",
      "exposure_class": "privileged",
      "auth_policy": "bearer_jwt",
      "required_roles": ["Broker"]$backendPrivilegedWriteRateProfile
    },
    {
      "id": "gongzzang.backend.listing_photo_read",
      "path": "/listings/:listing_id/photos/:photo_id",
      "methods": ["GET"],
      "router_group": "protected",
      "exposure_class": "authenticated_user",
      "auth_policy": "bearer_jwt"$backendAuthenticatedReadRateProfile
    },
    {
      "id": "gongzzang.backend.listing_detail_update",
      "path": "/listings/:id",
      "methods": ["PATCH"],
      "router_group": "protected",
      "exposure_class": "privileged",
      "auth_policy": "bearer_jwt",
      "required_roles": ["Broker"]$backendPrivilegedWriteRateProfile
    },
    {
      "id": "gongzzang.backend.auth_event",
      "path": "/internal/auth/event",
      "methods": ["POST"],
      "router_group": "internal",
      "exposure_class": "service_to_service",
      "auth_policy": "internal_shared_secret"
    }
  ],
"@
    }

    Write-File -Root $Root -RelativePath "docs\architecture\traffic-auth-policy-registry.v1.json" -Content @"
{
  "schema_version": "gongzzang.traffic_auth_policy_registry.v1",
  "repo_slug": "gongzzang",
  "exposure_classes": [
    {
      "class": "public_derived",
      "browser_visible": true,
      "direct_browser_access": "allowed",
      "confidentiality_guarantee": "none",
      "required_controls": [
        "data_minimization",
        "rate_limit",
        "response_budget_or_aggregate_only",
        "abuse_telemetry"
      ],
      "forbidden_data_classes": [
        "raw_listing_detail",
        "private_listing",
        "business_verified_listing_detail",
        "contact_data",
        "raw_platform_core_catalog",
        "bulk_listing_export"
      ]
    },
    {
      "class": "authenticated_user",
      "browser_visible": true,
      "direct_browser_access": "session_required",
      "confidentiality_guarantee": "authorization_enforced_per_request",
      "required_controls": ["session", "authorization", "audit_log", "rate_limit"],
      "forbidden_for_anonymous": true
    },
    {
      "class": "privileged",
      "browser_visible": true,
      "direct_browser_access": "session_and_role_required",
      "confidentiality_guarantee": "authorization_enforced_per_request",
      "required_controls": ["session", "role_or_entitlement", "audit_log", "abuse_detection"]
    },
    {
      "class": "service_to_service",
      "browser_visible": false,
      "direct_browser_access": "forbidden",
      "required_controls": ["service_identity", "allow_list", "audit_log"],
      "target_required_controls": ["mtls_or_short_lived_service_identity"]
    }
  ],
  "public_route_policies": [
    {
      "id": "gongzzang.public_map.listing_marker_tile",
      "exposure": "public_anonymous",
      "proxy_path_kind": "prefix",
      "proxy_path_source": "API.proxy.listingMarkerTilesPrefix",
      "proxy_path": "/api/proxy/map/v1/marker-tiles/listing",
      "backend_route": "/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf",
      "methods": ["GET"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-tile","limit":600,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "cache_policy": {"ttl_seconds":30},
      "single_flight_policy": {"lock_seconds":5,"wait_attempts":10,"wait_milliseconds":50},
      "response_budget": {"max_tile_bytes":262144,"max_features":10000}$tileExposure
    },
    {
      "id": "gongzzang.public_map.listing_marker_count",
      "exposure": "public_anonymous",
      "proxy_path_kind": "exact",
      "proxy_path_source": "API.proxy.listingMarkerCounts",
      "proxy_path": "/api/proxy/map/v1/marker-counts/listing",
      "backend_route": "/map/v1/marker-counts/listing",
      "methods": ["GET"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-count","limit":120,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "cache_policy": {"ttl_seconds":30},
      "single_flight_policy": {"lock_seconds":5,"wait_attempts":10,"wait_milliseconds":50}$countExposure
    },
    {
      "id": "gongzzang.public_map.listing_marker_filter",
      "exposure": "public_anonymous",
      "proxy_path_kind": "exact",
      "proxy_path_source": "API.proxy.listingMarkerFilters",
      "proxy_path": "/api/proxy/map/v1/marker-filters/listing",
      "backend_route": "/map/v1/marker-filters/listing",
      "methods": ["POST"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-filter","limit":60,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "filter_policy": {"normalized_hash_required":true,"idempotent_upsert_required":true}$filterExposure
    },
    {
      "id": "gongzzang.public_map.listing_marker_mask",
      "exposure": "public_anonymous",
      "proxy_path_kind": "prefix",
      "proxy_path_source": "LISTING_MARKER_MASK_PREFIX",
      "proxy_path": "/api/proxy/map/v1/marker-masks/listing",
      "backend_route": "/map/v1/marker-masks/listing/{z}/{x}/{y}",
      "methods": ["GET"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-mask","limit":120,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "cache_policy": {"ttl_seconds":30},
      "single_flight_policy": {"lock_seconds":5,"wait_attempts":10,"wait_milliseconds":50},
      "response_budget": {"max_mask_ids":20000}$maskExposure
    }
  ],
$authRoutePolicies
$pageRoutePolicies
$routeRateProfiles
$apiProxyRoutePolicies
$backendRoutePolicies
  "service_call_policies": [
    {
      "id": "gongzzang_to_platform_core_catalog",
      "current_auth_policy": {"env":"PLATFORM_CORE_SERVICE_TOKEN"},
      "target_auth_policy": {
        "method": "mtls_or_short_lived_service_identity",
        "service_identity": "gongzzang-api"
      }
    },
    {
      "id": "platform_core_to_gongzzang_events",
      "current_auth_policy": {"env":"PLATFORM_CORE_WEBHOOK_SECRET"},
      "target_auth_policy": {
        "method": "mtls_or_signed_event_envelope",
        "service_identity": "platform-core-outbox-publisher"
      }
    }
  ]
}
"@
    Write-File -Root $Root -RelativePath "apps\web\proxy.ts" -Content @'
GENERATED_PUBLIC_MAP_ROUTE_POLICIES
GENERATED_AUTH_RATE_ROUTE_POLICIES
GENERATED_PAGE_ROUTE_POLICIES
getAuthRateRoutePolicy
resolveAuthRateKey
getPageRoutePolicy
exposure: policy.exposure
API.proxy.listingMarkerTilesPrefix
API.proxy.listingMarkerCounts
API.proxy.listingMarkerFilters
LISTING_MARKER_MASK_PREFIX
'@
    $apiProxyExposureGate = if ($OmitApiProxyExposureGate) {
        ""
    } else {
        @'
enforceApiProxyExposure
sessionRequiredProblem
insufficientRoleProblem
policy.requiredRoles.includes
'@
    }
    Write-File -Root $Root -RelativePath "apps\web\app\api\proxy\[...path]\route.ts" -Content @"
GENERATED_API_PROXY_ROUTE_POLICIES
getApiProxyRoutePolicy
proxy/route-not-allowed
checkApiProxyRateLimit
resolveApiProxyRateKey
checkRate(
$apiProxyExposureGate
"@
    $generatedExposureMetadata = if ($OmitGeneratedExposureMetadata) {
        ""
    } else {
        @'
class: "public_derived"
rawRecordAccess: "forbidden"
bulkExport: "forbidden"
allowedDataClasses: ["derived_marker_tile"]
allowedDataClasses: ["aggregate_count"]
allowedDataClasses: ["opaque_filter_hash"]
allowedDataClasses: ["marker_id_mask"]
'@
    }
    $generatedAuthRatePolicies = if ($OmitGeneratedAuthRatePolicies) {
        ""
    } else {
        @'
GENERATED_AUTH_RATE_ROUTE_POLICIES
API.auth.login
API.auth.callback
API.auth.refresh
auth:login
auth:callback
auth:refresh
keyStrategy: "client_ip"
keyStrategy: "session_or_anon"
limit: 5
limit: 10
limit: 30
windowSec: 60
problemType: "auth/too-many-requests"
'@
    }
    $generatedPageRoutePolicies = if ($OmitGeneratedPageRoutePolicies) {
        ""
    } else {
        $generatedListingPageRoles = if ($AllowAdminListingPageRole) { 'requiredRoles: ["Broker", "Admin"]' } else { 'requiredRoles: ["Broker"]' }
        @'
GENERATED_PAGE_ROUTE_POLICIES
kind: "prefix"
kind: "exact"
kind: "prefix_suffix"
path: "/admin"
pathSource: "ROUTES.listings.new"
prefixSource: "ROUTES.listings.index"
suffix: "/edit"
requiredRoles: ["Admin", "Broker", "Operator"]
'@ + "`n$generatedListingPageRoles`n"
    }
    Write-File -Root $Root -RelativePath "apps\web\lib\policies\traffic-auth-policy.generated.ts" -Content @"
$generatedAuthRatePolicies
$generatedPageRoutePolicies
GENERATED_API_PROXY_ROUTE_POLICIES
keyPrefix: "api-proxy:authenticated-read"
keyPrefix: "api-proxy:authenticated-write"
keyPrefix: "api-proxy:privileged-write"
keyStrategy: "session_sub"
limit: 240
limit: 120
limit: 60
windowSec: 60
problemType: "proxy/too-many-requests"
map/v1/marker-tiles/listing/:z/:x/:y_pbf
listings/:listing_id/photos/:photo_id
listings
listings/:id
requiredRoles: []
requiredRoles: ["Broker"]
API.proxy.listingMarkerTilesPrefix
public-map:listing-marker-tile
limit: 600
windowSec: 60
API.proxy.listingMarkerCounts
public-map:listing-marker-count
limit: 120
windowSec: 60
API.proxy.listingMarkerFilters
public-map:listing-marker-filter
limit: 60
windowSec: 60
LISTING_MARKER_MASK_PREFIX
public-map:listing-marker-mask
limit: 120
windowSec: 60
$generatedExposureMetadata
"@
    Write-File -Root $Root -RelativePath "services\api\src\listing_marker_policy.rs" -Content @'
MAX_LISTING_MARKER_TILE_BYTES: usize = 262_144;
MAX_LISTING_MARKER_TILE_FEATURES: i64 = 10_000;
MAX_LISTING_MARKER_MASK_IDS: usize = 20_000;
LISTING_MARKER_CACHE_TTL_SECONDS: u64 = 30;
LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = 5;
LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = 10;
LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = 50;
'@
    $generatedBackendRolePolicies = if ($OmitGeneratedBackendRolePolicies) {
        ""
    } else {
        @'
BACKEND_ROLE_POLICIES
BackendRolePolicy {
    method: "POST",
    path_pattern: "/listings",
    required_roles: &[UserRole::Broker],
}
BackendRolePolicy {
    method: "PATCH",
    path_pattern: "/listings/:id",
    required_roles: &[UserRole::Broker],
}
'@
    }
    Write-File -Root $Root -RelativePath "services\api\src\traffic_auth_policy.rs" -Content @"
BACKEND_RATE_POLICIES
method: "GET"
method: "POST"
method: "PATCH"
path_pattern: "/listings"
path_pattern: "/listings/:listing_id/photos/:photo_id"
path_pattern: "/listings/:id"
key_prefix: "public-map:listing-marker-tile"
key_prefix: "public-map:listing-marker-count"
key_prefix: "public-map:listing-marker-filter"
key_prefix: "public-map:listing-marker-mask"
key_prefix: "api-proxy:authenticated-read"
key_prefix: "api-proxy:privileged-write"
limit: 600
limit: 120
limit: 60
limit: 240
window_seconds: 60
problem_type: "map/too-many-public-marker-requests"
problem_type: "proxy/too-many-requests"
$generatedBackendRolePolicies
"@
    Write-File -Root $Root -RelativePath "services\api\src\listing_marker_serving.rs" -Content @'
crate::listing_marker_policy
'@
    $protectedAuthLayer = if ($OmitBackendProtectedAuthLayer) { "" } else { "auth_layer" }
    $backendAuthorizationLayer = if ($OmitBackendAuthorizationLayer) {
        ""
    } else {
        @'
mod backend_authorization;
use crate::backend_authorization::{enforce_backend_roles, BackendAuthorizationState};
let backend_authorization_state = BackendAuthorizationState::new(traffic_auth_policy::BACKEND_ROLE_POLICIES);
'@
    }
    $backendAuthorizationLayerMount = if ($OmitBackendAuthorizationLayer) {
        ""
    } else {
        "    .layer(middleware::from_fn_with_state(backend_authorization_state.clone(), enforce_backend_roles))"
    }
    Write-File -Root $Root -RelativePath "services\api\src\main.rs" -Content @"
$backendAuthorizationLayer
mod backend_rate_limit;
mod traffic_auth_policy;
use crate::backend_rate_limit::{enforce_backend_rate_limit, RedisBackendRateLimiter};
let backend_rate_limiter = RedisBackendRateLimiter::new(redis_pool.clone());
let listing_marker_tiles_router: Router<()> = Router::new()
    .route("/map/v1/marker-tiles/listing/:z/:x/:y_pbf", get(routes::listing_marker_tiles::get_listing_marker_tile))
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit));
let listings_router: Router<()> = Router::new()
    .route("/listings", get(routes::listings::get_listings).post(routes::listings::create_listing))
    .route("/listings/:id", get(routes::listings::get_listing).patch(routes::listings::patch_listing))
    .route("/listings/:listing_id/photos/:photo_id", get(routes::listings::get_photo_download_redirect))
$backendAuthorizationLayerMount
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit))
    .layer(middleware::from_fn_with_state(auth_state.clone(), $protectedAuthLayer));
let internal_auth_secret = build_internal_auth_secret(is_production)?;
let internal: Router<()> = Router::new()
    .route("/internal/auth/event", axum::routing::post(routes::auth_event::post_auth_event))
    .with_state(auth_event_state);
"@
    Write-File -Root $Root -RelativePath "services\api\src\routes\health.rs" -Content @'
Router::new()
    .route("/healthz", get(liveness))
    .route("/healthz/ready", get(readiness))
'@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content @'
PLATFORM_CORE_SERVICE_TOKEN
PLATFORM_CORE_WEBHOOK_SECRET
direct_platform_core_database
'@
    if (!$OmitGeneratedEdgePolicy) {
        Write-File -Root $Root -RelativePath "infrastructure\security\traffic-auth-edge-policy.generated.json" -Content @'
{
  "schema_version": "gongzzang.traffic_auth_edge_policy_projection.v1",
  "source_registry": "docs/architecture/traffic-auth-policy-registry.v1.json",
  "projection_kind": "provider_neutral_edge_ingress",
  "generated_targets": ["cloudfront", "aws_wafv2", "alb", "service_mesh"],
  "public_route_rules": [
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_tile",
      "proxy_path": "/api/proxy/map/v1/marker-tiles/listing",
      "backend_route": "/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-tile",
        "limit": 600,
        "window_seconds": 60,
        "problem_type": "map/too-many-public-marker-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_count",
      "proxy_path": "/api/proxy/map/v1/marker-counts/listing",
      "backend_route": "/map/v1/marker-counts/listing",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-count",
        "limit": 120,
        "window_seconds": 60,
        "problem_type": "map/too-many-public-marker-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_filter",
      "proxy_path": "/api/proxy/map/v1/marker-filters/listing",
      "backend_route": "/map/v1/marker-filters/listing",
      "methods": ["POST"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-filter",
        "limit": 60,
        "window_seconds": 60,
        "problem_type": "map/too-many-public-marker-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_mask",
      "proxy_path": "/api/proxy/map/v1/marker-masks/listing",
      "backend_route": "/map/v1/marker-masks/listing/{z}/{x}/{y}",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-mask",
        "limit": 120,
        "window_seconds": 60,
        "problem_type": "map/too-many-public-marker-requests"
      }
    }
  ],
  "auth_route_rules": [
    {
      "source_policy_id": "gongzzang.auth.login",
      "path_source": "API.auth.login",
      "methods": ["POST"],
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "auth:login",
        "limit": 5,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.auth.callback",
      "path_source": "API.auth.callback",
      "methods": ["GET"],
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "auth:callback",
        "limit": 10,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.auth.refresh",
      "path_source": "API.auth.refresh",
      "methods": ["POST"],
      "rate": {
        "key_strategy": "session_or_anon",
        "key_prefix": "auth:refresh",
        "limit": 30,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    }
  ],
  "api_proxy_route_rules": [
    {
      "source_policy_id": "gongzzang.api_proxy.public_marker_tiles",
      "edge_path": "/api/proxy/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
      "target_path": "map/v1/marker-tiles/listing/:z/:x/:y_pbf",
      "target_path_kind": "template",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "required_roles": []
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listing_photo_read_delete",
      "edge_path": "/api/proxy/listings/:listing_id/photos/:photo_id",
      "target_path": "listings/:listing_id/photos/:photo_id",
      "target_path_kind": "template",
      "methods": ["GET"],
      "exposure_class": "authenticated_user",
      "required_roles": [],
      "rate": {
        "key_strategy": "session_sub",
        "key_prefix": "api-proxy:authenticated-read",
        "limit": 240,
        "window_seconds": 60,
        "problem_type": "proxy/too-many-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listings_collection_create",
      "edge_path": "/api/proxy/listings",
      "target_path": "listings",
      "target_path_kind": "exact",
      "methods": ["POST"],
      "exposure_class": "privileged",
      "required_roles": ["Broker"],
      "rate": {
        "key_strategy": "session_sub",
        "key_prefix": "api-proxy:privileged-write",
        "limit": 60,
        "window_seconds": 60,
        "problem_type": "proxy/too-many-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listing_detail_update",
      "edge_path": "/api/proxy/listings/:id",
      "target_path": "listings/:id",
      "target_path_kind": "template",
      "methods": ["PATCH"],
      "exposure_class": "privileged",
      "required_roles": ["Broker"],
      "rate": {
        "key_strategy": "session_sub",
        "key_prefix": "api-proxy:privileged-write",
        "limit": 60,
        "window_seconds": 60,
        "problem_type": "proxy/too-many-requests"
      }
    }
  ],
  "service_to_service_rules": [
    {
      "source_policy_id": "gongzzang_to_platform_core_catalog",
      "target_auth_method": "mtls_or_short_lived_service_identity",
      "service_identity": "gongzzang-api",
      "current_auth_env": "PLATFORM_CORE_SERVICE_TOKEN"
    },
    {
      "source_policy_id": "platform_core_to_gongzzang_events",
      "target_auth_method": "mtls_or_signed_event_envelope",
      "service_identity": "platform-core-outbox-publisher",
      "current_auth_env": "PLATFORM_CORE_WEBHOOK_SECRET"
    }
  ]
}
'@
    }
    if (!$OmitAwsWafEdgeManifest) {
        Write-File -Root $Root -RelativePath "infrastructure\security\aws-wafv2-edge-policy.generated.json" -Content @'
{
  "schema_version": "gongzzang.aws_wafv2_edge_policy_manifest.v1",
  "source_projection": "infrastructure/security/traffic-auth-edge-policy.generated.json",
  "managed_by": "pulumi",
  "scope_options": ["CLOUDFRONT", "REGIONAL"],
  "rate_based_rules": [
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_tile",
      "priority": 1000,
      "aggregate_key_type": "IP",
      "limit_per_5m": 3000,
      "match": {
        "path": "/api/proxy/map/v1/marker-tiles/listing",
        "path_match": "STARTS_WITH",
        "methods": ["GET"]
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_count",
      "priority": 1010,
      "aggregate_key_type": "IP",
      "limit_per_5m": 600,
      "match": {
        "path": "/api/proxy/map/v1/marker-counts/listing",
        "path_match": "EXACT",
        "methods": ["GET"]
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_filter",
      "priority": 1020,
      "aggregate_key_type": "IP",
      "limit_per_5m": 300,
      "match": {
        "path": "/api/proxy/map/v1/marker-filters/listing",
        "path_match": "EXACT",
        "methods": ["POST"]
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_mask",
      "priority": 1030,
      "aggregate_key_type": "IP",
      "limit_per_5m": 600,
      "match": {
        "path": "/api/proxy/map/v1/marker-masks/listing",
        "path_match": "STARTS_WITH",
        "methods": ["GET"]
      }
    },
    {
      "source_policy_id": "gongzzang.auth.login",
      "priority": 1040,
      "aggregate_key_type": "IP",
      "limit_per_5m": 25,
      "match": {
        "path": "/api/auth/login",
        "path_source": "API.auth.login",
        "path_match": "EXACT",
        "methods": ["POST"]
      }
    },
    {
      "source_policy_id": "gongzzang.auth.callback",
      "priority": 1050,
      "aggregate_key_type": "IP",
      "limit_per_5m": 50,
      "match": {
        "path": "/api/auth/callback",
        "path_source": "API.auth.callback",
        "path_match": "EXACT",
        "methods": ["GET"]
      }
    }
  ],
  "blocked_query_shape_rules": [],
  "identity_aware_application_rules": [
    {
      "source_policy_id": "gongzzang.auth.refresh",
      "reason": "key_strategy_not_representable_in_wafv2"
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listing_photo_read_delete",
      "reason": "key_strategy_not_representable_in_wafv2"
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listings_collection_create",
      "reason": "key_strategy_not_representable_in_wafv2"
    },
    {
      "source_policy_id": "gongzzang.api_proxy.listing_detail_update",
      "reason": "key_strategy_not_representable_in_wafv2"
    }
  ],
  "service_identity_rules": [
    {
      "source_policy_id": "gongzzang_to_platform_core_catalog",
      "target_auth_method": "mtls_or_short_lived_service_identity"
    },
    {
      "source_policy_id": "platform_core_to_gongzzang_events",
      "target_auth_method": "mtls_or_signed_event_envelope"
    }
  ]
}
'@
    }
    if (!$OmitPulumiWafConsumer) {
        $pulumiCliDependency = if ($OmitPulumiCliPackage) { "" } else { ', "pulumi": "3.244.0"' }
        Write-File -Root $Root -RelativePath "infrastructure\package.json" -Content @"
{
  "name": "@gongzzang/infrastructure",
  "private": true,
  "dependencies": {
    "@pulumi/aws": "7.31.0",
    "@pulumi/pulumi": "3.244.0"$pulumiCliDependency
  }
}
"@
        Write-File -Root $Root -RelativePath "infrastructure\Pulumi.yaml" -Content @'
name: gongzzang-infrastructure
runtime: nodejs
description: Gongzzang Pulumi infrastructure.
'@
        if (!$OmitPulumiLocalPreviewStack) {
            $pulumiLocalPreviewContent = @'
encryptionsalt: local-preview-test-salt
config:
  aws:region: ap-northeast-2
  aws:skipCredentialsValidation: "true"
  aws:skipRequestingAccountId: "true"
  aws:skipMetadataApiCheck: "true"
'@
            if ($PollutePulumiLocalPreviewStack) {
                $pulumiLocalPreviewContent += "`n  gongzzang-infrastructure:wafRegionalResourceArn: arn:aws:elasticloadbalancing:ap-northeast-2:123456789012:loadbalancer/app/gongzzang-ci/50dc6c495c0c9188"
            }
            Write-File -Root $Root -RelativePath "infrastructure\Pulumi.local-preview.yaml" -Content $pulumiLocalPreviewContent
        }
        $pulumiWafAssociationCode = if ($OmitPulumiWafAssociation) {
            ""
        } else {
            @'

new aws.wafv2.WebAclAssociation("gongzzang-edge-waf-regional-association", {
  resourceArn: wafRegionalResourceArn,
  webAclArn: "awsWafv2WebAclArn",
});
'@
        }
        Write-File -Root $Root -RelativePath "infrastructure\index.ts" -Content @"
import * as aws from "@pulumi/aws";

const manifestPath = "security/aws-wafv2-edge-policy.generated.json";
const wafRegionalResourceArn = "wafRegionalResourceArn";
const previewWafRegionalResourceArn = "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN";
const rateBasedRules = "rate_based_rules";
const blockedQueryShapeRules = "blocked_query_shape_rules";
const identityAwareApplicationRules = "identity_aware_application_rules";
const serviceIdentityRules = "service_identity_rules";

new aws.wafv2.WebAcl("gongzzang-edge-waf", {
  scope: "REGIONAL",
  defaultAction: { allow: {} },
  visibilityConfig: {
    cloudwatchMetricsEnabled: true,
    metricName: "gongzzang-edge-waf",
    sampledRequestsEnabled: true,
  },
  rules: [
    manifestPath,
    rateBasedRules,
    blockedQueryShapeRules,
    identityAwareApplicationRules,
    serviceIdentityRules,
  ].map((name, priority) => ({
    name,
    priority,
    action: { count: {} },
    statement: { rateBasedStatement: { aggregateKeyType: "IP", limit: 100 } },
    visibilityConfig: {
      cloudwatchMetricsEnabled: true,
      metricName: name,
      sampledRequestsEnabled: true,
    },
  })),
});
$pulumiWafAssociationCode
"@
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-Equals $success.ExitCode 0 "successful checker exit code mismatch output=$($success.Output)"
    Assert-Contains $success.Output "traffic-auth-policy-registry-ok"

    $coreOnlyRoot = Join-Path $TempRoot "core-without-production-edge"
    Write-MinimalRepo `
        -Root $coreOnlyRoot `
        -OmitGeneratedEdgePolicy `
        -OmitAwsWafEdgeManifest `
        -OmitPulumiWafConsumer `
        -OmitPulumiLocalPreviewStack
    $coreOnly = Invoke-Checker -Root $coreOnlyRoot
    Assert-Equals $coreOnly.ExitCode 0 "core traffic/auth checker must not require production edge artifacts output=$($coreOnly.Output)"
    Assert-Contains $coreOnly.Output "traffic-auth-policy-registry-ok"

    $missingExposureRoot = Join-Path $TempRoot "missing-data-exposure-policy"
    Write-MinimalRepo -Root $missingExposureRoot -OmitDataExposurePolicy
    $missingExposure = Invoke-Checker -Root $missingExposureRoot
    Assert-Equals $missingExposure.ExitCode 1 "missing data exposure policy exit code mismatch"
    Assert-Contains $missingExposure.Output "data_exposure_policy"

    $rawListingRoot = Join-Path $TempRoot "raw-listing-public"
    Write-MinimalRepo -Root $rawListingRoot -AllowRawListingDetail
    $rawListing = Invoke-Checker -Root $rawListingRoot
    Assert-Equals $rawListing.ExitCode 1 "raw listing detail public exit code mismatch"
    Assert-Contains $rawListing.Output "raw_listing_detail"

    $missingGeneratedExposureRoot = Join-Path $TempRoot "missing-generated-exposure"
    Write-MinimalRepo -Root $missingGeneratedExposureRoot -OmitGeneratedExposureMetadata
    $missingGeneratedExposure = Invoke-Checker -Root $missingGeneratedExposureRoot
    Assert-Equals $missingGeneratedExposure.ExitCode 1 "missing generated exposure exit code mismatch"
    Assert-Contains $missingGeneratedExposure.Output "generated TS exposure"

    $missingAuthRoutePoliciesRoot = Join-Path $TempRoot "missing-auth-route-policies"
    Write-MinimalRepo -Root $missingAuthRoutePoliciesRoot -OmitAuthRoutePolicies
    $missingAuthRoutePolicies = Invoke-Checker -Root $missingAuthRoutePoliciesRoot
    Assert-Equals $missingAuthRoutePolicies.ExitCode 1 "missing auth route policies exit code mismatch"
    Assert-Contains $missingAuthRoutePolicies.Output "auth_route_policies"

    $missingGeneratedAuthRateRoot = Join-Path $TempRoot "missing-generated-auth-rate"
    Write-MinimalRepo -Root $missingGeneratedAuthRateRoot -OmitGeneratedAuthRatePolicies
    $missingGeneratedAuthRate = Invoke-Checker -Root $missingGeneratedAuthRateRoot
    Assert-Equals $missingGeneratedAuthRate.ExitCode 1 "missing generated auth rate exit code mismatch"
    Assert-Contains $missingGeneratedAuthRate.Output "generated TS auth rate"

    $missingPageRoutePoliciesRoot = Join-Path $TempRoot "missing-page-route-policies"
    Write-MinimalRepo -Root $missingPageRoutePoliciesRoot -OmitPageRoutePolicies
    $missingPageRoutePolicies = Invoke-Checker -Root $missingPageRoutePoliciesRoot
    Assert-Equals $missingPageRoutePolicies.ExitCode 1 "missing page route policies exit code mismatch"
    Assert-Contains $missingPageRoutePolicies.Output "page_route_policies"

    $missingGeneratedPageRouteRoot = Join-Path $TempRoot "missing-generated-page-route"
    Write-MinimalRepo -Root $missingGeneratedPageRouteRoot -OmitGeneratedPageRoutePolicies
    $missingGeneratedPageRoute = Invoke-Checker -Root $missingGeneratedPageRouteRoot
    Assert-Equals $missingGeneratedPageRoute.ExitCode 1 "missing generated page route exit code mismatch"
    Assert-Contains $missingGeneratedPageRoute.Output "generated TS page route"

    $adminListingPageRoleRoot = Join-Path $TempRoot "admin-listing-page-role"
    Write-MinimalRepo -Root $adminListingPageRoleRoot -AllowAdminListingPageRole
    $adminListingPageRole = Invoke-Checker -Root $adminListingPageRoleRoot
    Assert-Equals $adminListingPageRole.ExitCode 1 "admin listing page role mismatch exit code mismatch"
    Assert-Contains $adminListingPageRole.Output "listing page route roles"

    $missingApiProxyPoliciesRoot = Join-Path $TempRoot "missing-api-proxy-route-policies"
    Write-MinimalRepo -Root $missingApiProxyPoliciesRoot -OmitApiProxyRoutePolicies
    $missingApiProxyPolicies = Invoke-Checker -Root $missingApiProxyPoliciesRoot
    Assert-Equals $missingApiProxyPolicies.ExitCode 1 "missing API proxy route policies exit code mismatch"
    Assert-Contains $missingApiProxyPolicies.Output "api_proxy_route_policies"

    $missingRouteRateProfilesRoot = Join-Path $TempRoot "missing-route-rate-profiles"
    Write-MinimalRepo -Root $missingRouteRateProfilesRoot -OmitRouteRateProfiles
    $missingRouteRateProfiles = Invoke-Checker -Root $missingRouteRateProfilesRoot
    Assert-Equals $missingRouteRateProfiles.ExitCode 1 "missing route rate profiles exit code mismatch"
    Assert-Contains $missingRouteRateProfiles.Output "route_rate_profiles"

    $missingAuthenticatedApiProxyRateRoot = Join-Path $TempRoot "missing-authenticated-api-proxy-rate"
    Write-MinimalRepo -Root $missingAuthenticatedApiProxyRateRoot -OmitAuthenticatedApiProxyRateProfile
    $missingAuthenticatedApiProxyRate = Invoke-Checker -Root $missingAuthenticatedApiProxyRateRoot
    Assert-Equals $missingAuthenticatedApiProxyRate.ExitCode 1 "missing authenticated API proxy rate exit code mismatch"
    Assert-Contains $missingAuthenticatedApiProxyRate.Output "rate_profile"

    $missingApiProxyExposureGateRoot = Join-Path $TempRoot "missing-api-proxy-exposure-gate"
    Write-MinimalRepo -Root $missingApiProxyExposureGateRoot -OmitApiProxyExposureGate
    $missingApiProxyExposureGate = Invoke-Checker -Root $missingApiProxyExposureGateRoot
    Assert-Equals $missingApiProxyExposureGate.ExitCode 1 "missing API proxy exposure gate exit code mismatch"
    Assert-Contains $missingApiProxyExposureGate.Output "API proxy exposure gate"

    $missingPrivilegedRolesRoot = Join-Path $TempRoot "missing-privileged-required-roles"
    Write-MinimalRepo -Root $missingPrivilegedRolesRoot -OmitPrivilegedRequiredRoles
    $missingPrivilegedRoles = Invoke-Checker -Root $missingPrivilegedRolesRoot
    Assert-Equals $missingPrivilegedRoles.ExitCode 1 "missing privileged required roles exit code mismatch"
    Assert-Contains $missingPrivilegedRoles.Output "required_roles"

    $missingBackendRoutesRoot = Join-Path $TempRoot "missing-backend-route-policies"
    Write-MinimalRepo -Root $missingBackendRoutesRoot -OmitBackendRoutePolicies
    $missingBackendRoutes = Invoke-Checker -Root $missingBackendRoutesRoot
    Assert-Equals $missingBackendRoutes.ExitCode 1 "missing backend route policies exit code mismatch"
    Assert-Contains $missingBackendRoutes.Output "backend_route_policies"

    $missingBackendRateRoot = Join-Path $TempRoot "missing-backend-rate-profile"
    Write-MinimalRepo -Root $missingBackendRateRoot -OmitBackendRateProfile
    $missingBackendRate = Invoke-Checker -Root $missingBackendRateRoot
    Assert-Equals $missingBackendRate.ExitCode 1 "missing backend rate profile exit code mismatch"
    Assert-Contains $missingBackendRate.Output "backend route policy rate_profile"

    $missingBackendAuthRoot = Join-Path $TempRoot "missing-backend-protected-auth"
    Write-MinimalRepo -Root $missingBackendAuthRoot -OmitBackendProtectedAuthLayer
    $missingBackendAuth = Invoke-Checker -Root $missingBackendAuthRoot
    Assert-Equals $missingBackendAuth.ExitCode 1 "missing backend protected auth exit code mismatch"
    Assert-Contains $missingBackendAuth.Output "backend protected route auth_layer"

    $missingGeneratedBackendRoleRoot = Join-Path $TempRoot "missing-generated-backend-role"
    Write-MinimalRepo -Root $missingGeneratedBackendRoleRoot -OmitGeneratedBackendRolePolicies
    $missingGeneratedBackendRole = Invoke-Checker -Root $missingGeneratedBackendRoleRoot
    Assert-Equals $missingGeneratedBackendRole.ExitCode 1 "missing generated backend role exit code mismatch"
    Assert-Contains $missingGeneratedBackendRole.Output "generated Rust backend role"

    $missingBackendAuthorizationRoot = Join-Path $TempRoot "missing-backend-authorization"
    Write-MinimalRepo -Root $missingBackendAuthorizationRoot -OmitBackendAuthorizationLayer
    $missingBackendAuthorization = Invoke-Checker -Root $missingBackendAuthorizationRoot
    Assert-Equals $missingBackendAuthorization.ExitCode 1 "missing backend authorization exit code mismatch"
    Assert-Contains $missingBackendAuthorization.Output "backend authorization"

    $missingGeneratedEdgePolicyRoot = Join-Path $TempRoot "missing-generated-edge-policy"
    Write-MinimalRepo -Root $missingGeneratedEdgePolicyRoot -OmitGeneratedEdgePolicy
    $missingGeneratedEdgePolicy = Invoke-Checker -Root $missingGeneratedEdgePolicyRoot -IncludeProductionEdge
    Assert-Equals $missingGeneratedEdgePolicy.ExitCode 1 "missing generated edge policy exit code mismatch"
    Assert-Contains $missingGeneratedEdgePolicy.Output "traffic-auth edge policy"

    $missingAwsWafManifestRoot = Join-Path $TempRoot "missing-aws-waf-manifest"
    Write-MinimalRepo -Root $missingAwsWafManifestRoot -OmitAwsWafEdgeManifest
    $missingAwsWafManifest = Invoke-Checker -Root $missingAwsWafManifestRoot -IncludeProductionEdge
    Assert-Equals $missingAwsWafManifest.ExitCode 1 "missing AWS WAF manifest exit code mismatch"
    Assert-Contains $missingAwsWafManifest.Output "AWS WAFv2 edge manifest"

    $missingPulumiWafConsumerRoot = Join-Path $TempRoot "missing-pulumi-waf-consumer"
    Write-MinimalRepo -Root $missingPulumiWafConsumerRoot -OmitPulumiWafConsumer
    $missingPulumiWafConsumer = Invoke-Checker -Root $missingPulumiWafConsumerRoot -IncludeProductionEdge
    Assert-Equals $missingPulumiWafConsumer.ExitCode 1 "missing Pulumi WAF consumer exit code mismatch"
    Assert-Contains $missingPulumiWafConsumer.Output "Pulumi AWS WAFv2 consumer"

    $missingPulumiCliPackageRoot = Join-Path $TempRoot "missing-pulumi-cli-package"
    Write-MinimalRepo -Root $missingPulumiCliPackageRoot -OmitPulumiCliPackage
    $missingPulumiCliPackage = Invoke-Checker -Root $missingPulumiCliPackageRoot -IncludeProductionEdge
    Assert-Equals $missingPulumiCliPackage.ExitCode 1 "missing Pulumi CLI package exit code mismatch"
    Assert-Contains $missingPulumiCliPackage.Output "Pulumi AWS WAFv2 package"

    $missingPulumiLocalPreviewRoot = Join-Path $TempRoot "missing-pulumi-local-preview-stack"
    Write-MinimalRepo -Root $missingPulumiLocalPreviewRoot -OmitPulumiLocalPreviewStack
    $missingPulumiLocalPreview = Invoke-Checker -Root $missingPulumiLocalPreviewRoot -IncludeProductionEdge
    Assert-Equals $missingPulumiLocalPreview.ExitCode 1 "missing Pulumi local preview stack exit code mismatch"
    Assert-Contains $missingPulumiLocalPreview.Output "Pulumi local-preview stack"

    $pollutedPulumiLocalPreviewRoot = Join-Path $TempRoot "polluted-pulumi-local-preview-stack"
    Write-MinimalRepo -Root $pollutedPulumiLocalPreviewRoot -PollutePulumiLocalPreviewStack
    $pollutedPulumiLocalPreview = Invoke-Checker -Root $pollutedPulumiLocalPreviewRoot -IncludeProductionEdge
    Assert-Equals $pollutedPulumiLocalPreview.ExitCode 1 "polluted Pulumi local preview stack exit code mismatch"
    Assert-Contains $pollutedPulumiLocalPreview.Output "wafRegionalResourceArn"

    $missingPulumiWafAssociationRoot = Join-Path $TempRoot "missing-pulumi-waf-association"
    Write-MinimalRepo -Root $missingPulumiWafAssociationRoot -OmitPulumiWafAssociation
    $missingPulumiWafAssociation = Invoke-Checker -Root $missingPulumiWafAssociationRoot -IncludeProductionEdge
    Assert-Equals $missingPulumiWafAssociation.ExitCode 1 "missing Pulumi WAF association exit code mismatch"
    Assert-Contains $missingPulumiWafAssociation.Output "Pulumi AWS WAFv2 association"

    Write-Host "traffic-auth-policy-registry-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
