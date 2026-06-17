Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-integration-policy.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-integration-policy-tests") `
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
        [switch] $IncludeProductionPromotion
    )
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    if ($IncludeProductionPromotion) {
        $arguments += "-IncludeProductionPromotion"
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

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitCiWiring,
        [switch] $OmitViteOverride,
        [switch] $OmitSupplyChainProvenance,
        [switch] $OmitSupplyChainCi,
        [switch] $OmitMigrationPrefixCi,
        [switch] $OmitDeployCandidateVerifier,
        [switch] $OmitDeployGateRunbook,
        [switch] $OmitDeployAdmissionWorkflow,
        [switch] $OmitProductionEdgeAdmission,
        [switch] $OmitLoadEvidenceAdmission,
        [switch] $OmitMigrationPrefixGuardrail,
        [switch] $OmitMigrationPrefixLefthook,
        [switch] $OmitDefaultDenyIdentityRuntime,
        [switch] $OmitCatalogRuntimeSurfaces,
        [switch] $OmitWorkloadIdentityTokenFileSupport,
        [switch] $ExpiredException
    )

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
    "anchore/sbom-action@e22c389904149dbc22b58101806040fa8d37a610",
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
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\webhook-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.webhook_policy.v1",
  "repo_slug": "gongzzang",
  "receiver": {
    "accepted_events": [
      "catalog.industrial_complex.gold_pointer.published.v1",
      "catalog.parcel_marker_anchor.snapshot.published.v1"
    ],
    "runtime_safety": {
      "idempotency_key": "event_id",
      "replay_ledger": "apps/web/lib/platform-core/event-inbox.ts",
      "duplicate_policy": "ack_without_side_effects",
      "dead_letter_queue": "redis_event_inbox_dead_letter_status",
      "schema_compatibility": "accepted_events_must_have_zod_handler"
    }
  }
}
'@
    $provenanceRequired = if ($OmitSupplyChainProvenance) { "false" } else { "true" }
    $loadTestCapacityAdmissionJson = if ($OmitLoadEvidenceAdmission) {
        ""
    } else {
        @'
    ,
    "load_test_capacity_admission": {
      "required": true,
      "verification_script": "scripts/ci/verify-load-test-capacity-evidence.ps1",
      "evidence_artifact_name": "load-test-capacity-evidence",
      "workflow_input_run_id": "load-evidence-run-id",
      "workflow_input_artifact_name": "load-evidence-artifact-name",
      "required_scenarios": ["api-read-mix", "map-marker-mix", "platform-core-events"],
      "required_environments": ["perf", "staging"],
      "target_host_by_environment": {
        "perf": "perf.gongzzang.internal",
        "staging": "staging.gongzzang.internal"
      },
      "forbidden_environments": ["local", "ci"],
      "required_classification": "healthy",
      "forbidden": [
        "production_deploy_without_perf_or_staging_load_evidence",
        "local_or_ci_smoke_used_as_launch_capacity_evidence",
        "capacity_evidence_from_production_target"
      ]
    }
'@
    }
    $productionEdgeAdmissionJson = if ($OmitProductionEdgeAdmission) {
        ""
    } else {
        @'
    ,
    "edge_admission": {
      "required": true,
      "policy_source": "docs/architecture/traffic-auth-policy-registry.v1.json",
      "generated_waf_manifest": "infrastructure/security/aws-wafv2-edge-policy.generated.json",
      "pulumi_project": "infrastructure/Pulumi.yaml",
      "pulumi_program": "infrastructure/index.ts",
      "verification_script": "scripts/ci/check-production-edge-admission.ps1",
      "regional_attachment": {
        "supported": true,
        "config_key": "wafRegionalResourceArn",
        "required_env": "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "pulumi_association_preview": {
          "required": true,
          "preview_script": "scripts/ci/check-pulumi-local-preview.ps1",
          "evidence": "regional_association=planned"
        }
      },
      "cloudfront_attachment": {
        "supported": false,
        "required_before_production": true
      },
      "forbidden": [
        "production_deploy_without_waf_attachment",
        "edge_policy_not_from_traffic_auth_registry",
        "manual_waf_console_change"
      ]
    }
'@
    }
    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\supply-chain-policy.v1.json" -Content @"
{
  "schema_version": "gongzzang.platform_integration.supply_chain_policy.v1",
  "repo_slug": "gongzzang",
  "npm": {
    "audit_bazel_target": "//tools/bazel:ci_node_audit_transition",
    "required_overrides": {
      "brace-expansion": "5.0.6",
      "postcss": "8.5.15",
      "vite": "6.4.2"
    }
  },
  "rust": {
    "sca": "cargo-deny",
    "config": "deny.toml",
    "bazel_target": "//tools/bazel:ci_cargo_deny_transition"
  },
  "sbom": {
    "required": true,
    "format": "cyclonedx-json",
    "generator": {
      "tool": "syft",
      "ci_action": "anchore/sbom-action",
      "upstream_tag": "v0.24.0",
      "pinned_ref": "e22c389904149dbc22b58101806040fa8d37a610"
    },
    "artifacts": [
      {
        "id": "gongzzang_node_workspace_sbom",
        "ecosystem": "node",
        "source_path": "pnpm-lock.yaml",
        "output_file": "target/supply-chain/gongzzang-node-workspace-sbom.cdx.json",
        "subject_path": "target/supply-chain/gongzzang-web-next-build.tgz"
      },
      {
        "id": "gongzzang_rust_workspace_sbom",
        "ecosystem": "rust",
        "source_path": "Cargo.lock",
        "output_file": "target/supply-chain/gongzzang-rust-workspace-sbom.cdx.json",
        "subject_path": "target/release/api"
      }
    ]
  },
  "provenance": {
    "required": $provenanceRequired,
    "provider": "github_artifact_attestations",
    "predicate": "slsa_build_provenance",
    "ci_action": "actions/attest",
    "upstream_tag": "v4",
    "pinned_ref": "281a49d4cbb0a72c9575a50d18f6deb515a11deb",
    "required_permissions": [
      "contents: read",
      "id-token: write",
      "attestations: write",
      "artifact-metadata: write"
    ],
    "production_subjects": [
      "target/supply-chain/gongzzang-web-next-build.tgz",
      "target/release/api"
    ]
  },
  "deploy_gate": {
    "required": true,
    "approved_workflow": ".github/workflows/ci.yml",
    "approved_job": "supply-chain-provenance",
    "candidate_policy": "production_candidates_must_be_built_on_main_by_approved_workflow",
    "verification_command": "gh attestation verify <artifact> -R <owner>/gongzzang",
    "verification_script": "scripts/ci/verify-production-deploy-candidate.ps1",
    "admission_workflow": ".github/workflows/production-deploy-admission.yml",
    "admission_job": "verify-production-deploy-candidates",
    "admission_environment": "production",
    "download_artifact_action": {
      "ci_action": "actions/download-artifact",
      "upstream_tag": "v4.3.0",
      "pinned_ref": "d3f86a106a0bac45b974a628896c90dbdf5c8093"
    },
    "runbook": "docs/runbooks/supply-chain-provenance-and-deploy-gate.md",
    "forbidden": [
      "deploy_without_attestation",
      "deploy_from_unapproved_workflow",
      "deploy_from_unverified_subject_digest",
      "mutable_image_tag_without_digest"
    ]
$loadTestCapacityAdmissionJson
$productionEdgeAdmissionJson
  }
}
"@
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
    $viteOverride = if ($OmitViteOverride) { "" } else { ',"vite":"6.4.2"' }
    Write-File -Root $Root -RelativePath "package.json" -Content @"
{
  "pnpm": {
    "overrides": {
      "brace-expansion": "5.0.6",
      "postcss": "8.5.15"$viteOverride
    }
  }
}
"@
    Write-File -Root $Root -RelativePath "deny.toml" -Content "deny"
    Write-File -Root $Root -RelativePath ".gitleaks.toml" -Content "gitleaks"
    foreach ($guardrailScript in @(
        "scripts\ci\check-platform-integration-policy.ps1",
        "scripts\ci\check-lakehouse-registry-integration.ps1",
        "scripts\ci\check-traffic-auth-policy-registry.ps1",
        "scripts\ci\check-platform-core-boundary.ps1",
        "scripts\ci\check-platform-core-event-receiver-contract.ps1",
        "scripts\ci\check-platform-core-catalog-api-contract.ps1",
        "scripts\ci\check-platform-core-dependency-boundary.ps1",
        "scripts\ci\check-pnu-anchor-pbf-marker-contract.ps1",
        "scripts\ci\check-migration-version-prefixes.ps1",
        "scripts\ci\check-platform-core-anchor-inbox-db-approval.ps1",
        "scripts\ci\check-load-test-assets.ps1",
        "scripts\ci\verify-load-test-capacity-evidence.ps1"
    )) {
        Write-File -Root $Root -RelativePath $guardrailScript -Content "guardrail"
    }
    $integrationCi = if ($OmitCiWiring) { "" } else { "check-platform-integration-policy.ps1" }
    $lakehouseRegistryCi = if ($OmitCiWiring) { "" } else {
        "check-lakehouse-registry-integration.ps1`ncheck-lakehouse-registry-integration.tests.ps1"
    }
    $migrationPrefixCi = if ($OmitMigrationPrefixCi) { "" } else { "check-migration-version-prefixes.ps1" }
    $supplyChainCi = if ($OmitSupplyChainCi) {
        ""
    } else {
        @"
supply-chain-provenance:
id-token: write
attestations: write
artifact-metadata: write
anchore/sbom-action@e22c389904149dbc22b58101806040fa8d37a610
actions/attest@281a49d4cbb0a72c9575a50d18f6deb515a11deb
format: cyclonedx-json
output-file: target/supply-chain/gongzzang-node-workspace-sbom.cdx.json
output-file: target/supply-chain/gongzzang-rust-workspace-sbom.cdx.json
subject-path: target/supply-chain/gongzzang-web-next-build.tgz
subject-path: target/release/api
sbom-path: target/supply-chain/gongzzang-node-workspace-sbom.cdx.json
sbom-path: target/supply-chain/gongzzang-rust-workspace-sbom.cdx.json
check-production-edge-admission.ps1
$migrationPrefixCi
check-platform-core-anchor-inbox-db-approval.ps1
check-load-test-assets.ps1
verify-load-test-capacity-evidence.tests.ps1
"@
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
//tools/bazel:ci_node_audit_transition
//tools/bazel:ci_cargo_deny_transition
gitleaks-action
$integrationCi
$lakehouseRegistryCi
$supplyChainCi
"@
    if (!$OmitDeployAdmissionWorkflow) {
        $productionEdgeAdmissionWorkflow = if ($OmitProductionEdgeAdmission) {
            ""
        } else {
            @'
Production edge admission
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
check-production-edge-admission.ps1
-RequirePulumiAssociationPreview
pnpm install --frozen-lockfile
'@
        }
        $loadEvidenceAdmissionWorkflow = if ($OmitLoadEvidenceAdmission) {
            ""
        } else {
            @'
load-evidence-run-id
load-evidence-artifact-name
Download load-test capacity evidence
Verify load-test capacity evidence
verify-load-test-capacity-evidence.ps1
target/admission/load-test-capacity-evidence
'@
        }
        Write-File -Root $Root -RelativePath ".github\workflows\production-deploy-admission.yml" -Content (@'
workflow_call:
workflow_dispatch:
verify-production-deploy-candidates:
environment: production
actions: read
attestations: read
actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093
verify-production-deploy-candidate.ps1
-PredicateType https://cyclonedx.org/bom
run-id
'@ + "`n$loadEvidenceAdmissionWorkflow`n$productionEdgeAdmissionWorkflow`n")
    }
    if (!$OmitDeployCandidateVerifier) {
        Write-File -Root $Root -RelativePath "scripts\ci\verify-production-deploy-candidate.ps1" -Content @'
gh
attestation
verify
RequiredWorkflow
RequiredRef
--predicate-type
production-deploy-candidate-ok
'@
    }
    if (!$OmitProductionEdgeAdmission) {
        Write-File -Root $Root -RelativePath "scripts\ci\check-production-edge-admission.ps1" -Content @'
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
wafRegionalResourceArn
aws-wafv2-edge-policy.generated.json
RequirePulumiAssociationPreview
check-pulumi-local-preview.ps1
regional_association=planned
production-edge-admission-ok
must be an AWS ARN
'@
        Write-File -Root $Root -RelativePath "scripts\ci\check-pulumi-local-preview.ps1" -Content @'
regional_association=planned
'@
        Write-File -Root $Root -RelativePath "infrastructure\security\aws-wafv2-edge-policy.generated.json" -Content @'
{"schema_version":"gongzzang.aws_wafv2_edge_policy_manifest.v1"}
'@
        Write-File -Root $Root -RelativePath "infrastructure\Pulumi.yaml" -Content @'
runtime: nodejs
'@
        Write-File -Root $Root -RelativePath "infrastructure\index.ts" -Content @'
wafRegionalResourceArn
aws.wafv2.WebAclAssociation
'@
    }
    if (!$OmitLoadEvidenceAdmission) {
        Write-File -Root $Root -RelativePath "scripts\ci\verify-load-test-capacity-evidence.ps1" -Content @'
EvidenceRoot
run.json
spec.json
k6-summary.json
Classification: healthy
load-test-capacity-evidence-ok
capacity evidence environment must be perf or staging
profile must be baseline, stress, spike, or soak
production targets are not valid load-test capacity evidence
target host must match capacity evidence environment
missing required load-test capacity scenario
RequiredScenarios
'@
    }
    $migrationPrefixLefthook = if ($OmitMigrationPrefixLefthook) { "" } else { "check-migration-version-prefixes.ps1" }
    Write-File -Root $Root -RelativePath "lefthook.yml" -Content @"
check-platform-integration-policy.ps1
check-lakehouse-registry-integration.ps1
$migrationPrefixLefthook
check-platform-core-anchor-inbox-db-approval.ps1
gitleaks protect --staged --redact -v
"@
    $defaultDenyIdentityRuntime = if ($OmitDefaultDenyIdentityRuntime) {
        ""
    } else {
        @'
x-gongzzang-service-auth-source
x-gongzzang-service-auth-target
x-gongzzang-allowed-call-id
gongzzang-api
platform-core-api
gongzzang_api_to_platform_core_catalog_read
'@
    }
    $workloadIdentityTokenFileRuntime = if ($OmitWorkloadIdentityTokenFileSupport) {
        ""
    } else {
        @'
new_from_workload_identity_token_file
WorkloadIdentityTokenFile
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
read_before_each_request
'@
    }
    Write-File -Root $Root -RelativePath "crates\auth\src\platform_core_service.rs" -Content @"
pub fn apply(request: reqwest::RequestBuilder) { request.bearer_auth("token"); }
pub fn fmt() { "<redacted>"; }
new_for_environment
$workloadIdentityTokenFileRuntime
MAX_TOKEN_TTL_DAYS
MetadataIssuedInFuture
MetadataTtlTooLong
x-gongzzang-service-auth-policy-id
$defaultDenyIdentityRuntime
x-gongzzang-service-auth-scope
x-gongzzang-service-auth-issued-at
x-gongzzang-service-auth-expires-at
x-gongzzang-service-auth-rotation-owner
"@
    Write-File -Root $Root -RelativePath "services\api\src\startup.rs" -Content @'
PLATFORM_CORE_SERVICE_TOKEN must be set
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
PlatformCoreServiceAuthMetadataConfig
PLATFORM_CORE_SERVICE_TOKEN_SCOPE
PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT
PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT
PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER
'@
    Write-File -Root $Root -RelativePath "services\api\src\platform_core_parcel_lookup.rs" -Content @'
catalog/v1/parcels/by-pnu/
'@
    Write-File -Root $Root -RelativePath "services\api\src\building_reader.rs" -Content @'
catalog/v1/parcels/by-pnu/{}/buildings
'@
    Write-File -Root $Root -RelativePath "docs\runbooks\platform-core-service-token-rotation.md" -Content @'
catalog:read
90 days
SPIFFE/SPIRE
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
read before each Platform Core request
default-deny
x-gongzzang-service-auth-policy-id
gongzzang_api_to_platform_core_catalog_read
'@
    if (!$OmitDeployGateRunbook) {
        Write-File -Root $Root -RelativePath "docs\runbooks\supply-chain-provenance-and-deploy-gate.md" -Content @'
SBOM
SLSA
GitHub Artifact Attestations
gh attestation verify
approved workflow
subject digest
load-test capacity evidence
load-evidence-run-id
verify-load-test-capacity-evidence.ps1
production edge admission
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
wafRegionalResourceArn
regional_association=planned
WebAclAssociation
'@
    }
    Write-File -Root $Root -RelativePath "docs\runbooks\platform-core-integration-operations.md" -Content @'
platform_core_catalog_read_slo_burn
platform_core_catalog_circuit_open
platform_core_webhook_dead_letter_or_latency
platform_core_webhook_replay_surge
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\env.ts" -Content @'
PLATFORM_CORE_WEBHOOK_SECRET
'@
    Write-File -Root $Root -RelativePath "apps\web\app\platform-core\events\route.ts" -Content @'
createHmac
timingSafeEqual
x-platform-core-signature
x-platform-core-timestamp
WEBHOOK_MAX_SKEW_SECONDS
x-platform-core-event-id
x-platform-core-event-type
x-platform-core-outbox-scope
reservePlatformCoreEvent
recordPlatformCoreEventAccepted
recordPlatformCoreEventDeadLetter
catalog.industrial_complex.gold_pointer.published.v1
catalog.parcel_marker_anchor.snapshot.published.v1
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\platform-core\event-inbox.ts" -Content @'
PlatformCoreEventInboxRecord
reservePlatformCoreEvent
recordPlatformCoreEventAccepted
recordPlatformCoreEventDeadLetter
dead_letter
"NX"
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-events.test.ts" -Content @'
acknowledges duplicate platform-core event bursts without repeated side effects
dead_letter
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\platform_core_anchor_import_integration.rs" -Content @'
processing_inbox_event_can_be_reclaimed_for_retry_after_worker_exit
'@
    Write-File -Root $Root -RelativePath "crates\circuit-breaker\src\execute.rs" -Content @'
execute_timeout_records_failure
execute_returns_open_when_breaker_open
'@
    Write-File -Root $Root -RelativePath "docs\ssot-matrix.md" -Content "Platform Integration Policy"
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    if ($success.ExitCode -ne 0) {
        throw "successful checker exit code mismatch expected='0' actual='$($success.ExitCode)': $($success.Output)"
    }
    Assert-Contains $success.Output "platform-integration-policy-ok"

    $coreOnlyRoot = Join-Path $TempRoot "core-without-production-promotion"
    Write-MinimalRepo `
        -Root $coreOnlyRoot `
        -OmitDeployCandidateVerifier `
        -OmitDeployGateRunbook `
        -OmitDeployAdmissionWorkflow `
        -OmitProductionEdgeAdmission `
        -OmitLoadEvidenceAdmission
    $coreOnly = Invoke-Checker -Root $coreOnlyRoot
    Assert-Equals $coreOnly.ExitCode 0 "core platform integration checker must not require production promotion gates"
    Assert-Contains $coreOnly.Output "platform-integration-policy-ok"

    $missingCiRoot = Join-Path $TempRoot "missing-ci"
    Write-MinimalRepo -Root $missingCiRoot -OmitCiWiring
    $missingCi = Invoke-Checker -Root $missingCiRoot
    Assert-Equals $missingCi.ExitCode 1 "missing CI wiring exit code mismatch"
    Assert-Contains $missingCi.Output "check-platform-integration-policy.ps1"

    $missingOverrideRoot = Join-Path $TempRoot "missing-override"
    Write-MinimalRepo -Root $missingOverrideRoot -OmitViteOverride
    $missingOverride = Invoke-Checker -Root $missingOverrideRoot
    Assert-Equals $missingOverride.ExitCode 1 "missing override exit code mismatch"
    Assert-Contains $missingOverride.Output "pnpm override mismatch for vite"

    $missingSupplyChainRoot = Join-Path $TempRoot "missing-supply-chain-provenance"
    Write-MinimalRepo -Root $missingSupplyChainRoot -OmitSupplyChainProvenance
    $missingSupplyChain = Invoke-Checker -Root $missingSupplyChainRoot
    Assert-Equals $missingSupplyChain.ExitCode 1 "missing supply chain provenance exit code mismatch"
    Assert-Contains $missingSupplyChain.Output "supply chain provenance requirement mismatch"

    $missingSupplyChainCiRoot = Join-Path $TempRoot "missing-supply-chain-ci"
    Write-MinimalRepo -Root $missingSupplyChainCiRoot -OmitSupplyChainCi
    $missingSupplyChainCi = Invoke-Checker -Root $missingSupplyChainCiRoot
    Assert-Equals $missingSupplyChainCi.ExitCode 1 "missing supply chain CI exit code mismatch"
    Assert-Contains $missingSupplyChainCi.Output "CI required jobs or steps"

    $missingMigrationPrefixCiRoot = Join-Path $TempRoot "missing-migration-prefix-ci"
    Write-MinimalRepo -Root $missingMigrationPrefixCiRoot -OmitMigrationPrefixCi
    $missingMigrationPrefixCi = Invoke-Checker -Root $missingMigrationPrefixCiRoot
    Assert-Equals $missingMigrationPrefixCi.ExitCode 1 "missing migration prefix CI exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixCi.Output `
        "CI required jobs or steps missing 'check-migration-version-prefixes.ps1'"

    $missingDeployAdmissionRoot = Join-Path $TempRoot "missing-deploy-admission"
    Write-MinimalRepo -Root $missingDeployAdmissionRoot -OmitDeployAdmissionWorkflow
    $missingDeployAdmission = Invoke-Checker -Root $missingDeployAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingDeployAdmission.ExitCode 1 "missing deploy admission exit code mismatch"
    Assert-Contains $missingDeployAdmission.Output "production-deploy-admission.yml"

    $missingProductionEdgeAdmissionRoot = Join-Path $TempRoot "missing-production-edge-admission"
    Write-MinimalRepo -Root $missingProductionEdgeAdmissionRoot -OmitProductionEdgeAdmission
    $missingProductionEdgeAdmission = Invoke-Checker -Root $missingProductionEdgeAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingProductionEdgeAdmission.ExitCode 1 "missing production edge admission exit code mismatch"
    Assert-Contains $missingProductionEdgeAdmission.Output "production edge admission"

    $missingLoadEvidenceAdmissionRoot = Join-Path $TempRoot "missing-load-evidence-admission"
    Write-MinimalRepo -Root $missingLoadEvidenceAdmissionRoot -OmitLoadEvidenceAdmission
    $missingLoadEvidenceAdmission = Invoke-Checker -Root $missingLoadEvidenceAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingLoadEvidenceAdmission.ExitCode 1 "missing load evidence admission exit code mismatch"
    Assert-Contains $missingLoadEvidenceAdmission.Output "load-test capacity admission"

    $missingMigrationPrefixGuardrailRoot = Join-Path $TempRoot "missing-migration-prefix-guardrail"
    Write-MinimalRepo -Root $missingMigrationPrefixGuardrailRoot -OmitMigrationPrefixGuardrail
    $missingMigrationPrefixGuardrail = Invoke-Checker -Root $missingMigrationPrefixGuardrailRoot
    Assert-Equals `
        $missingMigrationPrefixGuardrail.ExitCode `
        1 `
        "missing migration prefix guardrail exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixGuardrail.Output `
        "index required guardrails missing 'scripts/ci/check-migration-version-prefixes.ps1'"

    $missingMigrationPrefixLefthookRoot = Join-Path $TempRoot "missing-migration-prefix-lefthook"
    Write-MinimalRepo -Root $missingMigrationPrefixLefthookRoot -OmitMigrationPrefixLefthook
    $missingMigrationPrefixLefthook = Invoke-Checker -Root $missingMigrationPrefixLefthookRoot
    Assert-Equals $missingMigrationPrefixLefthook.ExitCode 1 "missing migration prefix lefthook exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixLefthook.Output `
        "lefthook migration prefix gate"

    $expiredExceptionRoot = Join-Path $TempRoot "expired-exception"
    Write-MinimalRepo -Root $expiredExceptionRoot -ExpiredException
    $expiredException = Invoke-Checker -Root $expiredExceptionRoot
    Assert-Equals $expiredException.ExitCode 1 "expired exception exit code mismatch"
    Assert-Contains $expiredException.Output "expired_at"

    $missingDefaultDenyIdentityRuntimeRoot = Join-Path $TempRoot "missing-default-deny-identity-runtime"
    Write-MinimalRepo -Root $missingDefaultDenyIdentityRuntimeRoot -OmitDefaultDenyIdentityRuntime
    $missingDefaultDenyIdentityRuntime = Invoke-Checker -Root $missingDefaultDenyIdentityRuntimeRoot
    Assert-Equals $missingDefaultDenyIdentityRuntime.ExitCode 1 "missing default-deny identity runtime exit code mismatch"
    Assert-Contains $missingDefaultDenyIdentityRuntime.Output "default-deny identity runtime"

    $missingCatalogRuntimeSurfacesRoot = Join-Path $TempRoot "missing-catalog-runtime-surfaces"
    Write-MinimalRepo -Root $missingCatalogRuntimeSurfacesRoot -OmitCatalogRuntimeSurfaces
    $missingCatalogRuntimeSurfaces = Invoke-Checker -Root $missingCatalogRuntimeSurfacesRoot
    Assert-Equals $missingCatalogRuntimeSurfaces.ExitCode 1 "missing catalog runtime surfaces exit code mismatch"
    Assert-Contains $missingCatalogRuntimeSurfaces.Output "catalog runtime surface"

    $missingWorkloadIdentityTokenFileRoot = Join-Path $TempRoot "missing-workload-identity-token-file"
    Write-MinimalRepo -Root $missingWorkloadIdentityTokenFileRoot -OmitWorkloadIdentityTokenFileSupport
    $missingWorkloadIdentityTokenFile = Invoke-Checker -Root $missingWorkloadIdentityTokenFileRoot
    Assert-Equals $missingWorkloadIdentityTokenFile.ExitCode 1 "missing workload identity token file exit code mismatch"
    Assert-Contains $missingWorkloadIdentityTokenFile.Output "workload identity token file"

    Write-Host "platform-integration-policy-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
