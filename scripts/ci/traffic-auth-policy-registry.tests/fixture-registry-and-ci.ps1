function Write-TrafficAuthRegistryAndCiFixtures {
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
    },
    {
      "id": "gongzzang.public_map.listing_marker_tombstone",
      "exposure": "public_anonymous",
      "proxy_path_kind": "prefix",
      "proxy_path_source": "API.proxy.listingMarkerTombstonesPrefix",
      "proxy_path": "/api/proxy/map/v1/marker-tombstones/listing",
      "backend_route": "/map/v1/marker-tombstones/listing/{z}/{x}/{y}",
      "methods": ["GET"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-tombstone","limit":120,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "response_budget": {"max_mask_ids":20000}$maskExposure
    },
    {
      "id": "gongzzang.public_map.listing_marker_delta",
      "exposure": "public_anonymous",
      "proxy_path_kind": "prefix",
      "proxy_path_source": "API.proxy.listingMarkerDeltasPrefix",
      "proxy_path": "/api/proxy/map/v1/marker-deltas/listing",
      "backend_route": "/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf",
      "methods": ["GET"],
      "auth_policy": {"method":"anonymous_public","session_required":false},
      "rate_policy": {"key_prefix":"public-map:listing-marker-delta","limit":120,"window_seconds":60,"problem_type":"map/too-many-public-marker-requests"},
      "response_budget": {"max_tile_bytes":262144,"max_features":10000}$tileExposure
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
    $ciWorkflowTrafficAuthPolicyGate = if ($OmitCiWorkflowTrafficAuthPolicyGate) {
        @'
name: CI
jobs:
  lint-format:
    runs-on: ubuntu-latest
    steps:
      - name: Platform Core boundary guardrail
        shell: pwsh
        run: ./scripts/ci/check-platform-core-boundary.ps1
'@
    } else {
        @'
name: CI
jobs:
  lint-format:
    runs-on: ubuntu-latest
    steps:
      - name: Traffic/auth policy registry guardrail
        shell: pwsh
        run: ./scripts/ci/check-traffic-auth-policy-registry.ps1
      - name: Traffic/auth policy registry guardrail tests
        shell: pwsh
        run: ./scripts/ci/check-traffic-auth-policy-registry.tests.ps1
      - name: Traffic/auth production edge policy guardrail
        shell: pwsh
        run: ./scripts/ci/check-traffic-auth-policy-registry.ps1 -IncludeProductionEdge
'@
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content $ciWorkflowTrafficAuthPolicyGate
}
