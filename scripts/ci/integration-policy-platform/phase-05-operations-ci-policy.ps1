$telemetryPolicy = $operationsPolicy.telemetry
foreach ($attribute in @(
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
)) {
    Assert-JsonArrayContains -Values @($telemetryPolicy.required_span_attributes) -Expected $attribute -Message "operations telemetry required span attributes"
}
foreach ($forbiddenAttribute in @(
    "authorization",
    "cookie",
    "set-cookie",
    "platform_core_service_token",
    "platform_core_webhook_secret"
)) {
    Assert-JsonArrayContains -Values @($telemetryPolicy.forbidden_attributes) -Expected $forbiddenAttribute -Message "operations telemetry forbidden attributes"
}

$sloPolicies = @($operationsPolicy.slos)
Assert-Equals -Actual $sloPolicies.Count -Expected 2 -Message "operations SLO count mismatch"
Assert-Unique -Values ($sloPolicies | ForEach-Object { $_.id }) -Message "operations SLO ids must be unique"
foreach ($slo in $sloPolicies) {
    Assert-NotEmptyString -Value $slo.alert_policy -Message "operations SLO alert_policy for $($slo.id)"
    Assert-FileExists -RelativePath ([string] $slo.runbook)
    if ([double] $slo.availability_percent -lt 99.9) {
        throw "operations SLO '$($slo.id)' availability_percent must be at least 99.9"
    }
    if ([int] $slo.p95_latency_ms -gt 300) {
        throw "operations SLO '$($slo.id)' p95_latency_ms must be 300ms or lower"
    }
    if ([int] $slo.p99_latency_ms -gt 1000) {
        throw "operations SLO '$($slo.id)' p99_latency_ms must be 1000ms or lower"
    }
}
foreach ($requiredSloId in @(
    "gongzzang_api_to_platform_core_catalog_read",
    "platform_core_outbox_to_gongzzang_webhook"
)) {
    Assert-JsonArrayContains -Values @($sloPolicies | ForEach-Object { [string] $_.id }) -Expected $requiredSloId -Message "operations SLO ids"
}

$alerts = @($operationsPolicy.alerts)
Assert-Unique -Values ($alerts | ForEach-Object { $_.id }) -Message "operations alert ids must be unique"
foreach ($alert in $alerts) {
    Assert-NotEmptyString -Value $alert.signal -Message "operations alert signal for $($alert.id)"
    Assert-NotEmptyString -Value $alert.severity -Message "operations alert severity for $($alert.id)"
    Assert-FileExists -RelativePath ([string] $alert.runbook)
}
foreach ($requiredAlertId in @(
    "platform_core_catalog_read_slo_burn",
    "platform_core_catalog_circuit_open",
    "platform_core_webhook_dead_letter_or_latency",
    "platform_core_webhook_replay_surge"
)) {
    Assert-JsonArrayContains -Values @($alerts | ForEach-Object { [string] $_.id }) -Expected $requiredAlertId -Message "operations alert ids"
}

$loadFaultTests = @($operationsPolicy.load_fault_tests)
Assert-Equals -Actual $loadFaultTests.Count -Expected 5 -Message "operations load/fault test count mismatch"
Assert-Unique -Values ($loadFaultTests | ForEach-Object { $_.id }) -Message "operations load/fault test ids must be unique"
foreach ($test in $loadFaultTests) {
    Assert-Equals -Actual $test.required -Expected $true -Message "operations load/fault test must be required for $($test.id)"
    Assert-FileExists -RelativePath ([string] $test.test_file)
    $testContent = Read-TextFile -RelativePath ([string] $test.test_file)
    Assert-Contains -Content $testContent -Needle ([string] $test.evidence) -Message "operations load/fault test evidence for $($test.id)"
}

$package = Read-JsonFile -RelativePath "package.json"
$overrides = $package.pnpm.overrides
foreach ($name in @("brace-expansion", "postcss", "vite")) {
    $expected = [string] (Get-JsonProperty -Object $supplyChainPolicy.npm.required_overrides -Name $name)
    $actual = [string] (Get-JsonProperty -Object $overrides -Name $name)
    Assert-Equals -Actual $actual -Expected $expected -Message "pnpm override mismatch for $name"
}

Assert-FileExists -RelativePath "deny.toml"
Assert-FileExists -RelativePath ".gitleaks.toml"

$ci = Read-TextFile -RelativePath ".github/workflows/ci.yml"
$requiredCiJobsOrSteps = @(Get-JsonProperty -Object $index -Name "required_ci_jobs_or_steps" | ForEach-Object { [string] $_ })
$productionPromotionCiJobsOrSteps = @(
    "check-production-edge-admission.ps1",
    "check-load-test-assets.ps1"
)
$requiredCiJobsOrSteps = @($requiredCiJobsOrSteps | Where-Object {
        $productionPromotionCiJobsOrSteps -notcontains ([string] $_)
    })
foreach ($requiredCiJobOrStep in $requiredCiJobsOrSteps) {
    Assert-Contains `
        -Content $ci `
        -Needle $requiredCiJobOrStep `
        -Message "CI required jobs or steps"
}
foreach ($needle in @(
    "//tools/bazel:ci_node_audit_transition",
    "//tools/bazel:ci_cargo_deny_transition",
    "gitleaks-action",
    "check-platform-integration-policy.ps1",
    "check-lakehouse-registry-integration.ps1",
    "supply-chain-provenance:",
    "id-token: write",
    "attestations: write",
    "artifact-metadata: write",
    "//:supply_chain_evidence_artifacts",
    "//:verify_supply_chain",
    "actions/attest@281a49d4cbb0a72c9575a50d18f6deb515a11deb",
    "sbom-path: bazel-bin/supply-chain/gongzzang-node-workspace-sbom.cdx.json",
    "sbom-path: bazel-bin/supply-chain/gongzzang-rust-workspace-sbom.cdx.json",
    "bazel-bin/supply-chain/evidence-manifest.json"
)) {
    Assert-Contains -Content $ci -Needle $needle -Message "CI platform integration gate"
}
foreach ($releaseArtifact in $releaseArtifacts) {
    Assert-Contains -Content $ci -Needle ([string] $releaseArtifact.subject_path) -Message "CI release artifact subject path"
}
foreach ($sbomArtifact in $sbomArtifacts) {
    Assert-Contains -Content $ci -Needle ([string] $sbomArtifact.output_file) -Message "CI SBOM output path"
}
foreach ($needle in @(
    [string] $evidenceManifest.artifact_group_target,
    [string] $evidenceManifest.contract_target,
    [string] $evidenceManifest.output_file
)) {
    Assert-Contains -Content $ci -Needle $needle -Message "CI supply chain evidence manifest wiring"
}

if ($IncludeProductionPromotion) {
    $deployGateRunbook = Read-TextFile -RelativePath ([string] $deployGate.runbook)
    foreach ($needle in @(
        "SBOM",
        "SLSA",
        "GitHub Artifact Attestations",
        "gh attestation verify",
        "approved workflow",
        "subject digest"
    )) {
        Assert-Contains -Content $deployGateRunbook -Needle $needle -Message "supply chain deploy gate runbook"
    }
    foreach ($needle in @(
        "load-test capacity evidence",
        "load-evidence-run-id",
        "verify-load-test-capacity-evidence.ps1",
        "production edge admission",
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "wafRegionalResourceArn",
        "regional_association=planned",
        "WebAclAssociation"
    )) {
        Assert-Contains -Content $deployGateRunbook -Needle $needle -Message "supply chain production promotion runbook"
    }
}

$lefthook = Read-TextFile -RelativePath "lefthook.yml"
Assert-Contains -Content $lefthook -Needle "check-platform-integration-policy.ps1" -Message "lefthook platform integration gate"
Assert-Contains -Content $lefthook -Needle "check-lakehouse-registry-integration.ps1" -Message "lefthook lakehouse registry integration gate"
Assert-Contains -Content $lefthook -Needle "check-migration-version-prefixes.ps1" -Message "lefthook migration prefix gate"
Assert-Contains -Content $lefthook -Needle "check-platform-core-anchor-inbox-db-approval.ps1" -Message "lefthook anchor inbox DB approval gate"
Assert-Contains -Content $lefthook -Needle "gitleaks protect --staged --redact -v" -Message "lefthook gitleaks gate"

$ssotMatrix = Read-TextFile -RelativePath "docs/ssot-matrix.md"
Assert-Contains -Content $ssotMatrix -Needle "Platform Integration Policy" -Message "SSOT matrix platform integration row"
