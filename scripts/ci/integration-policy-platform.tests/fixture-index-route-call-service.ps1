    $migrationPrefixGuardrail = if ($OmitMigrationPrefixGuardrail) { "" } else {
        '    "scripts/ci/check-migration-version-prefixes.ps1",'
    }

    $indexContent = @'
{
  "schema_version": "gongzzang.platform_integration.index.v1",
  "repo_slug": "gongzzang",
  "components": [
    {"id":"platform_integration.route_exposure","path":"docs/architecture/platform-integration/route-exposure-policy.v1.json","schema_version":"gongzzang.platform_integration.route_exposure_policy.v1"},
    {"id":"platform_integration.traffic_auth","path":"docs/architecture/traffic-auth-policy-registry.v1.json","schema_version":"gongzzang.traffic_auth_policy_registry.v1"},
    {"id":"platform_integration.platform_core_boundary","path":"docs/architecture/platform-core-boundary.v1.json","schema_version":"gongzzang.platform_core_boundary.v1"},
    {"id":"platform_integration.allowed_call_matrix","path":"docs/architecture/platform-integration/allowed-call-matrix.v1.json","schema_version":"gongzzang.platform_integration.allowed_call_matrix.v1"},
    {"id":"platform_integration.service_auth","path":"docs/architecture/platform-integration/service-auth-policy.v1.json","schema_version":"gongzzang.platform_integration.service_auth_policy.v1"},
    {"id":"platform_integration.webhook","path":"docs/architecture/platform-integration/webhook-policy.v1.json","schema_version":"gongzzang.platform_integration.webhook_policy.v1"},
    {"id":"platform_integration.supply_chain","path":"docs/architecture/platform-integration/supply-chain-policy.v1.json","schema_version":"gongzzang.platform_integration.supply_chain_policy.v1"},
    {"id":"platform_integration.operations","path":"docs/architecture/platform-integration/operations-policy.v1.json","schema_version":"gongzzang.platform_integration.operations_policy.v1"},
    {"id":"platform_integration.exception_policy","path":"docs/architecture/platform-integration/exception-policy.v1.json","schema_version":"gongzzang.platform_integration.exception_policy.v1"},
    {"id":"platform_integration.lakehouse_registry","path":"docs/architecture/platform-integration/lakehouse-registry-policy.v1.json","schema_version":"gongzzang.platform_integration.lakehouse_registry_policy.v1"}
  ],
  "required_guardrails": [
    "scripts/ci/check-platform-integration-policy.ps1",
    "scripts/ci/check-lakehouse-registry-integration.ps1",
    "scripts/ci/check-traffic-auth-policy-registry.ps1",
    "scripts/ci/check-platform-core-boundary.ps1",
    "scripts/ci/check-platform-core-event-receiver-contract.ps1",
    "scripts/ci/check-platform-core-catalog-api-contract.ps1",
    "scripts/ci/check-platform-core-dependency-boundary.ps1",
    "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1",
'@
    if (![string]::IsNullOrWhiteSpace($migrationPrefixGuardrail)) {
        $indexContent += "`n$migrationPrefixGuardrail"
    }
    $indexContent += @'
    "scripts/ci/check-platform-core-anchor-inbox-db-approval.ps1"
  ],
  "production_promotion_guardrails": [
    "scripts/ci/check-production-edge-admission.ps1",
    "scripts/ci/check-pulumi-local-preview.ps1",
    "scripts/ci/check-load-test-assets.ps1",
    "scripts/ci/verify-load-test-capacity-evidence.ps1"
  ],
  "required_ci_jobs_or_steps": [
    "//tools/bazel:ci_node_audit_transition",
    "//tools/bazel:ci_cargo_deny_transition",
    "gitleaks-action",
    "check-platform-integration-policy.ps1",
    "check-lakehouse-registry-integration.ps1",
    "check-lakehouse-registry-integration.tests.ps1",
    "check-migration-version-prefixes.ps1",
    "check-platform-core-anchor-inbox-db-approval.ps1",
    "supply-chain-provenance",
    "//:supply_chain_evidence_artifacts",
    "//:verify_supply_chain",
    "actions/attest@281a49d4cbb0a72c9575a50d18f6deb515a11deb"
  ],
  "production_promotion_jobs_or_steps": [
    "check-production-edge-admission.ps1",
    "check-load-test-assets.ps1",
    "verify-load-test-capacity-evidence.tests.ps1"
  ]
}
'@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\index.v1.json" -Content $indexContent
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\route-exposure-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.route_exposure_policy.v1",
  "repo_slug": "gongzzang",
  "surfaces": [
    {"id":"gongzzang.public_map.listing_marker_tile","policy_source":"docs/architecture/traffic-auth-policy-registry.v1.json"},
    {"id":"gongzzang.public_map.listing_marker_count","policy_source":"docs/architecture/traffic-auth-policy-registry.v1.json"},
    {"id":"gongzzang.public_map.listing_marker_filter","policy_source":"docs/architecture/traffic-auth-policy-registry.v1.json"},
    {"id":"gongzzang.public_map.listing_marker_mask","policy_source":"docs/architecture/traffic-auth-policy-registry.v1.json"}
  ],
  "forbidden_public_shapes": ["bbox", "bounds", "south", "west", "north", "east"]
}
'@
    $catalogAllowedSurfaces = if ($OmitCatalogRuntimeSurfaces) {
        '["/catalog/v1/vector-tiles/manifest"]'
    } else {
        '["/catalog/v1/vector-tiles/manifest", "/map/v1/marker-tiles/contract", "/catalog/v1/parcels/by-pnu/:pnu", "/catalog/v1/parcels/by-pnu/:pnu/buildings"]'
    }
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\allowed-call-matrix.v1.json" -Content @"
{
  "schema_version": "gongzzang.platform_integration.allowed_call_matrix.v1",
  "repo_slug": "gongzzang",
  "default_decision": "deny",
  "allowed_calls": [
    {
      "id": "gongzzang_api_to_platform_core_catalog_read",
      "status": "active",
      "source_repo": "gongzzang",
      "source_service": "gongzzang-api",
      "target_repo": "platform-core",
      "target_service": "platform-core-api",
      "allowed_surfaces": $catalogAllowedSurfaces,
      "traffic_policy_id": "gongzzang_to_platform_core_catalog",
      "service_auth_policy_id": "gongzzang_api_to_platform_core_api",
      "decision_reference": "docs/adr/0034-catalog-ownership-handover-to-platform-core.md",
      "current_required_controls": ["bearer_service_token", "no_direct_database"],
      "target_required_controls": ["short_lived_workload_identity"]
    },
    {
      "id": "platform_core_outbox_to_gongzzang_webhook",
      "status": "active",
      "source_repo": "platform-core",
      "source_service": "platform-core-outbox-publisher",
      "target_repo": "gongzzang",
      "target_service": "gongzzang-web",
      "allowed_surfaces": ["/platform-core/events"],
      "traffic_policy_id": "platform_core_to_gongzzang_events",
      "webhook_policy_id": "platform_core_event_receiver",
      "service_auth_policy_id": "platform_core_outbox_to_gongzzang_webhook",
      "decision_reference": "docs/adr/0034-catalog-ownership-handover-to-platform-core.md",
      "current_required_controls": ["timestamped_hmac_sha256"],
      "target_required_controls": ["event_replay_ledger"]
    },
    {
      "id": "dawneer_to_platform_core_catalog_read",
      "status": "planned",
      "source_repo": "dawneer",
      "source_service": "dawneer-web",
      "target_repo": "platform-core",
      "target_service": "platform-core-api",
      "allowed_surfaces": ["platform-core-catalog-read-api"],
      "current_required_controls": ["not_yet_launched"],
      "target_required_controls": ["short_lived_workload_identity"]
    }
  ],
  "prohibited_calls": [
    {"id":"gongzzang_to_platform_core_database","decision":"deny"},
    {"id":"dawneer_to_platform_core_database","decision":"deny"},
    {"id":"platform_core_to_gongzzang_listing_write","decision":"deny"}
  ]
}
"@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\service-auth-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.service_auth_policy.v1",
  "repo_slug": "gongzzang",
  "outbound_identities": [
    {
      "id":"gongzzang_api_to_platform_core_api",
      "workload_identity_token_file": {
        "preferred_in_production": true,
        "required_env": "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE",
        "refresh_behavior": "read_before_each_request",
        "fallback_static_token_policy": "allowed_only_with_metadata_until_cutover"
      },
      "token_metadata": {
        "required_in_production": true,
        "required_scope": "catalog:read",
        "max_ttl_days": 90,
        "required_env": [
          "PLATFORM_CORE_SERVICE_TOKEN_SCOPE",
          "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT",
          "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT",
          "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER"
        ],
        "runtime_headers": [
          "x-gongzzang-service-auth-policy-id",
          "x-gongzzang-service-auth-source",
          "x-gongzzang-service-auth-target",
          "x-gongzzang-allowed-call-id",
          "x-gongzzang-service-auth-scope",
          "x-gongzzang-service-auth-issued-at",
          "x-gongzzang-service-auth-expires-at",
          "x-gongzzang-service-auth-rotation-owner"
        ],
        "rotation_runbook": "docs/runbooks/platform-core-service-token-rotation.md",
        "production_dev_token_policy": "forbidden"
      },
      "authorization_policy": {
        "default_decision": "deny",
        "allow_source": "docs/architecture/platform-integration/allowed-call-matrix.v1.json"
      },
      "workload_identity_cutover": {
        "target": "spiffe_spire_or_cloud_workload_identity"
      }
    }
  ],
  "inbound_identities": [{"id":"platform_core_outbox_to_gongzzang_webhook"}]
}
'@
    if ($OmitWorkloadIdentityTokenFileSupport) {
        $serviceAuthPolicyPath = Join-Path $Root "docs\architecture\platform-integration\service-auth-policy.v1.json"
        $serviceAuthPolicy = Get-Content -LiteralPath $serviceAuthPolicyPath -Raw -Encoding UTF8
        $serviceAuthPolicy = $serviceAuthPolicy -replace '(?s)\s+"workload_identity_token_file": \{.*?\},\r?\n', "`n"
        Set-Content -LiteralPath $serviceAuthPolicyPath -Encoding UTF8 -Value $serviceAuthPolicy
    }
