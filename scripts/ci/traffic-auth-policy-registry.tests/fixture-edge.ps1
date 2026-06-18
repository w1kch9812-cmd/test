function Write-TrafficAuthEdgeFixtures {
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
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_tombstone",
      "proxy_path": "/api/proxy/map/v1/marker-tombstones/listing",
      "backend_route": "/map/v1/marker-tombstones/listing/{z}/{x}/{y}",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-tombstone",
        "limit": 120,
        "window_seconds": 60,
        "problem_type": "map/too-many-public-marker-requests"
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_delta",
      "proxy_path": "/api/proxy/map/v1/marker-deltas/listing",
      "backend_route": "/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf",
      "methods": ["GET"],
      "exposure_class": "public_derived",
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "public-map:listing-marker-delta",
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
    },
    {
      "source_policy_id": "gongzzang.auth.logout",
      "path_source": "API.auth.logout",
      "methods": ["POST", "GET"],
      "rate": {
        "key_strategy": "client_ip",
        "key_prefix": "auth:logout",
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
      "source_policy_id": "gongzzang.public_map.listing_marker_tombstone",
      "priority": 1031,
      "aggregate_key_type": "IP",
      "limit_per_5m": 600,
      "match": {
        "path": "/api/proxy/map/v1/marker-tombstones/listing",
        "path_match": "STARTS_WITH",
        "methods": ["GET"]
      }
    },
    {
      "source_policy_id": "gongzzang.public_map.listing_marker_delta",
      "priority": 1032,
      "aggregate_key_type": "IP",
      "limit_per_5m": 600,
      "match": {
        "path": "/api/proxy/map/v1/marker-deltas/listing",
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
    },
    {
      "source_policy_id": "gongzzang.auth.logout",
      "priority": 1060,
      "aggregate_key_type": "IP",
      "limit_per_5m": 150,
      "match": {
        "path": "/api/auth/logout",
        "path_source": "API.auth.logout",
        "path_match": "EXACT",
        "methods": ["POST", "GET"]
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
}
