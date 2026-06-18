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


. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests\fixture-registry-and-ci.ps1")
. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests\fixture-web.ps1")
. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests\fixture-backend.ps1")
. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests\fixture-edge.ps1")
. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests\fixture-pulumi.ps1")

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitDataExposurePolicy,
        [switch] $AllowRawListingDetail,
        [switch] $OmitGeneratedExposureMetadata,
        [switch] $OmitGeneratedAuthRatePolicies,
        [switch] $OmitGeneratedPageRoutePolicies,
        [switch] $OmitAuthRoutePolicies,
        [switch] $OmitAuthLogoutPolicy,
        [switch] $OmitPageRoutePolicies,
        [switch] $AllowAdminListingPageRole,
        [switch] $OmitApiProxyRoutePolicies,
        [switch] $AddUnregisteredApiProxyClientUsage,
        [switch] $OmitRouteRateProfiles,
        [switch] $OmitAuthenticatedApiProxyRateProfile,
        [switch] $OmitApiProxyExposureGate,
        [switch] $OmitPrivilegedRequiredRoles,
        [switch] $OmitBackendRoutePolicies,
        [switch] $OmitBackendRateProfile,
        [switch] $OmitBackendProtectedAuthLayer,
        [switch] $AddUnregisteredBackendRoute,
        [switch] $OmitGeneratedBackendRolePolicies,
        [switch] $OmitBackendAuthorizationLayer,
        [switch] $OmitGeneratedEdgePolicy,
        [switch] $OmitAwsWafEdgeManifest,
        [switch] $OmitPulumiWafConsumer,
        [switch] $OmitPulumiCliPackage,
        [switch] $OmitPulumiLocalPreviewStack,
        [switch] $PollutePulumiLocalPreviewStack,
        [switch] $OmitPulumiWafAssociation,
        [switch] $OmitCiWorkflowTrafficAuthPolicyGate
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
    $authLogoutRoutePolicy = if ($OmitAuthLogoutPolicy) {
        ""
    } else {
        @'
,
    {
      "id": "gongzzang.auth.logout",
      "path_source": "API.auth.logout",
      "methods": ["POST", "GET"],
      "rate_policy": {
        "key_prefix": "auth:logout",
        "key_strategy": "client_ip",
        "limit": 30,
        "window_seconds": 60,
        "problem_type": "auth/too-many-requests"
      }
    }
'@
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
'@ + $authLogoutRoutePolicy + @'
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
      "id": "gongzzang.backend.health_liveness",
      "path": "/healthz",
      "methods": ["GET"],
      "router_group": "public_health",
      "exposure_class": "public_health",
      "auth_policy": "anonymous_public"
    },
    {
      "id": "gongzzang.backend.health_readiness",
      "path": "/healthz/ready",
      "methods": ["GET"],
      "router_group": "public_health",
      "exposure_class": "public_health",
      "auth_policy": "anonymous_public"
    },
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

    Write-TrafficAuthRegistryAndCiFixtures
    Write-TrafficAuthWebFixtures
    Write-TrafficAuthBackendFixtures
    Write-TrafficAuthEdgeFixtures
    Write-TrafficAuthPulumiFixtures
}
