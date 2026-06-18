    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\operations-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.operations_policy.v1",
  "repo_slug": "gongzzang",
  "telemetry": {
    "required_span_attributes": [
      "service.name",
      "peer.service",
      "http.request.method",
      "url.path",
      "platform_integration.call_id",
      "platform_integration.policy_id",
      "platform_integration.direction",
      "platform_integration.decision",
      "platform_core.event_id",
      "platform_core.event_type",
      "correlation_id"
    ],
    "forbidden_attributes": [
      "authorization",
      "cookie",
      "set-cookie",
      "platform_core_service_token",
      "platform_core_webhook_secret"
    ]
  },
  "slos": [
    {
      "id": "gongzzang_api_to_platform_core_catalog_read",
      "availability_percent": 99.9,
      "p95_latency_ms": 300,
      "p99_latency_ms": 1000,
      "alert_policy": "platform_core_catalog_read_slo_burn",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    },
    {
      "id": "platform_core_outbox_to_gongzzang_webhook",
      "availability_percent": 99.9,
      "p95_latency_ms": 250,
      "p99_latency_ms": 1000,
      "alert_policy": "platform_core_webhook_dead_letter_or_latency",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    }
  ],
  "alerts": [
    {
      "id": "platform_core_catalog_read_slo_burn",
      "signal": "latency_or_error_budget_burn",
      "severity": "page",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    },
    {
      "id": "platform_core_catalog_circuit_open",
      "signal": "circuit_breaker_open",
      "severity": "page",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    },
    {
      "id": "platform_core_webhook_dead_letter_or_latency",
      "signal": "dead_letter_count_or_handler_latency",
      "severity": "page",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    },
    {
      "id": "platform_core_webhook_replay_surge",
      "signal": "duplicate_event_rate",
      "severity": "ticket",
      "runbook": "docs/runbooks/platform-core-integration-operations.md"
    }
  ],
  "load_fault_tests": [
    {
      "id": "webhook_duplicate_burst_ack",
      "test_file": "apps/web/tests/unit/platform-core-events.test.ts",
      "evidence": "acknowledges duplicate platform-core event bursts without repeated side effects",
      "required": true
    },
    {
      "id": "webhook_dead_letter_poison_event",
      "test_file": "apps/web/tests/unit/platform-core-events.test.ts",
      "evidence": "dead_letter",
      "required": true
    },
    {
      "id": "anchor_import_processing_reclaim",
      "test_file": "crates/db/tests/platform_core_anchor_import_integration.rs",
      "evidence": "processing_inbox_event_can_be_reclaimed_for_retry_after_worker_exit",
      "required": true
    },
    {
      "id": "catalog_circuit_breaker_timeout_fault",
      "test_file": "crates/circuit-breaker/src/execute.rs",
      "evidence": "execute_timeout_records_failure",
      "required": true
    },
    {
      "id": "catalog_circuit_breaker_open_fault",
      "test_file": "crates/circuit-breaker/src/execute.rs",
      "evidence": "execute_returns_open_when_breaker_open",
      "required": true
    }
  ]
}
'@
    $activeExceptions = if ($ExpiredException) {
        @'
[
    {
      "id": "expired-exception",
      "owner": "platform",
      "reason": "fixture",
      "risk_level": "low",
      "expires_at": "2020-01-01T00:00:00Z",
      "approval_reference": "docs/adr/fixture.md",
      "compensating_controls": ["fixture-control"]
    }
  ]
'@
    } else {
        "[]"
    }
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\exception-policy.v1.json" -Content @"
{
  "schema_version": "gongzzang.platform_integration.exception_policy.v1",
  "repo_slug": "gongzzang",
  "default_exception_decision": "deny",
  "exception_contract": {
    "max_ttl_days": 90,
    "required_fields": [
      "id",
      "owner",
      "reason",
      "risk_level",
      "expires_at",
      "approval_reference",
      "compensating_controls"
    ],
    "forbidden_risk_levels": ["critical"]
  },
  "active_exceptions": $activeExceptions,
  "forbidden_exception_types": [
    "direct_platform_core_database_access",
    "anonymous_platform_core_service_call_in_production",
    "unsigned_platform_core_webhook_in_production",
    "unattested_production_artifact_deploy"
  ]
}
"@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\lakehouse-registry-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.lakehouse_registry_policy.v1"
}
'@
    Write-File -Root $Root -RelativePath "docs\architecture\traffic-auth-policy-registry.v1.json" -Content @'
{
  "schema_version": "gongzzang.traffic_auth_policy_registry.v1",
  "public_route_policies": [
    {"id":"gongzzang.public_map.listing_marker_tile"},
    {"id":"gongzzang.public_map.listing_marker_count"},
    {"id":"gongzzang.public_map.listing_marker_filter"},
    {"id":"gongzzang.public_map.listing_marker_mask"}
  ],
  "service_call_policies": [
    {"id":"gongzzang_to_platform_core_catalog","current_auth_policy":{"env":"PLATFORM_CORE_SERVICE_TOKEN"}},
    {"id":"platform_core_to_gongzzang_events","current_auth_policy":{"env":"PLATFORM_CORE_WEBHOOK_SECRET"}}
  ]
}
'@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_core_boundary.v1",
  "root_env_example_contract": {
    "required_service_auth_env": ["PLATFORM_CORE_SERVICE_TOKEN", "PLATFORM_CORE_WEBHOOK_SECRET"]
  }
}
'@
    Write-File -Root $Root -RelativePath ".env.example" -Content @'
PLATFORM_CORE_SERVICE_TOKEN=fixture-platform-core-service-token
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE=
PLATFORM_CORE_SERVICE_TOKEN_SCOPE=
PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT=
PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT=
PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER=
PLATFORM_CORE_WEBHOOK_SECRET=fixture-platform-core-webhook-secret
'@
