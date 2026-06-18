$indexPath = "docs/architecture/platform-integration/index.v1.json"
$routePath = "docs/architecture/platform-integration/route-exposure-policy.v1.json"
$callMatrixPath = "docs/architecture/platform-integration/allowed-call-matrix.v1.json"
$serviceAuthPath = "docs/architecture/platform-integration/service-auth-policy.v1.json"
$webhookPath = "docs/architecture/platform-integration/webhook-policy.v1.json"
$supplyChainPath = "docs/architecture/platform-integration/supply-chain-policy.v1.json"
$operationsPath = "docs/architecture/platform-integration/operations-policy.v1.json"
$exceptionPath = "docs/architecture/platform-integration/exception-policy.v1.json"
$lakehouseRegistryPath = "docs/architecture/platform-integration/lakehouse-registry-policy.v1.json"
$trafficPath = "docs/architecture/traffic-auth-policy-registry.v1.json"
$boundaryPath = "docs/architecture/platform-core-boundary.v1.json"

$index = Read-JsonFile -RelativePath $indexPath
$routePolicy = Read-JsonFile -RelativePath $routePath
$callMatrix = Read-JsonFile -RelativePath $callMatrixPath
$serviceAuthPolicy = Read-JsonFile -RelativePath $serviceAuthPath
$webhookPolicy = Read-JsonFile -RelativePath $webhookPath
$supplyChainPolicy = Read-JsonFile -RelativePath $supplyChainPath
$operationsPolicy = Read-JsonFile -RelativePath $operationsPath
$exceptionPolicy = Read-JsonFile -RelativePath $exceptionPath
$lakehouseRegistryPolicy = Read-JsonFile -RelativePath $lakehouseRegistryPath
$trafficPolicy = Read-JsonFile -RelativePath $trafficPath
$boundary = Read-JsonFile -RelativePath $boundaryPath

Assert-Equals -Actual $index.schema_version -Expected "gongzzang.platform_integration.index.v1" -Message "index schema_version mismatch"
Assert-Equals -Actual $routePolicy.schema_version -Expected "gongzzang.platform_integration.route_exposure_policy.v1" -Message "route policy schema_version mismatch"
Assert-Equals -Actual $callMatrix.schema_version -Expected "gongzzang.platform_integration.allowed_call_matrix.v1" -Message "allowed call matrix schema_version mismatch"
Assert-Equals -Actual $serviceAuthPolicy.schema_version -Expected "gongzzang.platform_integration.service_auth_policy.v1" -Message "service auth policy schema_version mismatch"
Assert-Equals -Actual $webhookPolicy.schema_version -Expected "gongzzang.platform_integration.webhook_policy.v1" -Message "webhook policy schema_version mismatch"
Assert-Equals -Actual $supplyChainPolicy.schema_version -Expected "gongzzang.platform_integration.supply_chain_policy.v1" -Message "supply chain policy schema_version mismatch"
Assert-Equals -Actual $operationsPolicy.schema_version -Expected "gongzzang.platform_integration.operations_policy.v1" -Message "operations policy schema_version mismatch"
Assert-Equals -Actual $exceptionPolicy.schema_version -Expected "gongzzang.platform_integration.exception_policy.v1" -Message "exception policy schema_version mismatch"
Assert-Equals -Actual $lakehouseRegistryPolicy.schema_version -Expected "gongzzang.platform_integration.lakehouse_registry_policy.v1" -Message "lakehouse registry policy schema_version mismatch"
Assert-Equals -Actual $trafficPolicy.schema_version -Expected "gongzzang.traffic_auth_policy_registry.v1" -Message "traffic policy schema_version mismatch"
Assert-Equals -Actual $boundary.schema_version -Expected "gongzzang.platform_core_boundary.v1" -Message "boundary schema_version mismatch"

$components = @($index.components)
Assert-Equals -Actual $components.Count -Expected 10 -Message "platform integration component count mismatch"
Assert-Unique -Values ($components | ForEach-Object { $_.id }) -Message "platform integration component ids must be unique"
foreach ($component in $components) {
    Assert-FileExists -RelativePath ([string] $component.path)
    $componentPolicy = Read-JsonFile -RelativePath ([string] $component.path)
    Assert-Equals -Actual $componentPolicy.schema_version -Expected ([string] $component.schema_version) -Message "component schema mismatch for $($component.id)"
}

foreach ($required in @(
    "platform_integration.route_exposure",
    "platform_integration.traffic_auth",
    "platform_integration.platform_core_boundary",
    "platform_integration.allowed_call_matrix",
    "platform_integration.service_auth",
    "platform_integration.webhook",
    "platform_integration.supply_chain",
    "platform_integration.operations",
    "platform_integration.exception_policy",
    "platform_integration.lakehouse_registry"
)) {
    if (!(@($components | ForEach-Object { [string] $_.id }) -contains $required)) {
        throw "platform integration index missing component '$required'"
    }
}

$requiredIndexGuardrails = @(
    "scripts/ci/check-platform-integration-policy.ps1",
    "scripts/ci/check-lakehouse-registry-integration.ps1",
    "scripts/ci/check-traffic-auth-policy-registry.ps1",
    "scripts/ci/check-platform-core-boundary.ps1",
    "scripts/ci/check-platform-core-event-receiver-contract.ps1",
    "scripts/ci/check-platform-core-catalog-api-contract.ps1",
    "scripts/ci/check-platform-core-dependency-boundary.ps1",
    "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1",
    "scripts/ci/check-migration-version-prefixes.ps1",
    "scripts/ci/check-platform-core-anchor-inbox-db-approval.ps1"
)
$requiredProductionPromotionGuardrails = @(
    "scripts/ci/check-production-edge-admission.ps1",
    "scripts/ci/check-pulumi-local-preview.ps1",
    "scripts/ci/check-load-test-assets.ps1",
    "scripts/ci/verify-load-test-capacity-evidence.ps1"
)
$indexGuardrails = @(Get-JsonProperty -Object $index -Name "required_guardrails" | ForEach-Object { [string] $_ })
$indexProductionPromotionGuardrails = @(
    Get-JsonProperty -Object $index -Name "production_promotion_guardrails" |
        ForEach-Object { [string] $_ }
)
$guardrailsToRequire = @($requiredIndexGuardrails)
if ($IncludeProductionPromotion) {
    foreach ($guardrail in $requiredProductionPromotionGuardrails) {
        Assert-JsonArrayContains `
            -Values $indexProductionPromotionGuardrails `
            -Expected $guardrail `
            -Message "index production promotion guardrails"
    }
}
foreach ($guardrail in $guardrailsToRequire) {
    Assert-JsonArrayContains `
        -Values $indexGuardrails `
        -Expected $guardrail `
        -Message "index required guardrails"
    Assert-FileExists -RelativePath $guardrail
}
