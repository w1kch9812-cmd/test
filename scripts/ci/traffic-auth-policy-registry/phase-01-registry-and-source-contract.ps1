$registry = Read-JsonFile -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
Assert-Equals -Actual $registry.schema_version -Expected $ExpectedSchemaVersion -Message "schema_version mismatch"
Assert-Equals -Actual $registry.repo_slug -Expected "gongzzang" -Message "repo_slug mismatch"

function Assert-JsonEquivalent {
    param([object] $Actual, [object] $Expected, [string] $Message)

    $actualJson = $Actual | ConvertTo-Json -Depth 32 -Compress
    $expectedJson = $Expected | ConvertTo-Json -Depth 32 -Compress
    if ($actualJson -ne $expectedJson) {
        throw "$Message is out of sync with registry fragments"
    }
}

function Assert-RegistryFragmentsMatchAggregate {
    $fragmentRoot = Resolve-RepoPath -RelativePath "docs/architecture/traffic-auth-policy-registry"
    if (!(Test-Path -LiteralPath $fragmentRoot -PathType Container)) {
        return
    }

    $metadata = Read-JsonFile -RelativePath "docs/architecture/traffic-auth-policy-registry/00-metadata.json"
    Assert-JsonEquivalent -Actual $registry.schema_version -Expected $metadata.schema_version -Message "schema_version"
    Assert-JsonEquivalent -Actual $registry.repo_slug -Expected $metadata.repo_slug -Message "repo_slug"
    Assert-JsonEquivalent -Actual @($registry.decision_sources) -Expected @($metadata.decision_sources) -Message "decision_sources"
    Assert-JsonEquivalent -Actual @($registry.policy_principles) -Expected @($metadata.policy_principles) -Message "policy_principles"

    $fragmentSpecs = @(
        @{ Path = "10-exposure-classes.json"; Property = "exposure_classes" },
        @{ Path = "20-public-route-policies.json"; Property = "public_route_policies" },
        @{ Path = "30-auth-route-policies.json"; Property = "auth_route_policies" },
        @{ Path = "40-route-rate-profiles.json"; Property = "route_rate_profiles" },
        @{ Path = "50-page-route-policies.json"; Property = "page_route_policies" },
        @{ Path = "60-api-proxy-route-policies.json"; Property = "api_proxy_route_policies" },
        @{ Path = "70-backend-route-policies.json"; Property = "backend_route_policies" },
        @{ Path = "80-service-call-policies.json"; Property = "service_call_policies" }
    )

    foreach ($spec in $fragmentSpecs) {
        $relativePath = "docs/architecture/traffic-auth-policy-registry/$($spec.Path)"
        $propertyName = [string] $spec.Property
        $fragment = Read-JsonFile -RelativePath $relativePath
        Assert-JsonEquivalent `
            -Actual @($registry.PSObject.Properties[$propertyName].Value) `
            -Expected @($fragment.PSObject.Properties[$propertyName].Value) `
            -Message $propertyName
    }
}

Assert-RegistryFragmentsMatchAggregate

$requiredPublicForbiddenDataClasses = @(
    "raw_listing_detail",
    "private_listing",
    "business_verified_listing_detail",
    "contact_data",
    "raw_platform_core_catalog",
    "bulk_listing_export"
)

$exposureClasses = @($registry.exposure_classes)
Assert-ArrayContains `
    -Values ($exposureClasses | ForEach-Object { $_.class }) `
    -Expected "public_derived" `
    -Message "exposure_classes"
Assert-ArrayContains `
    -Values ($exposureClasses | ForEach-Object { $_.class }) `
    -Expected "authenticated_user" `
    -Message "exposure_classes"
Assert-ArrayContains `
    -Values ($exposureClasses | ForEach-Object { $_.class }) `
    -Expected "privileged" `
    -Message "exposure_classes"
Assert-ArrayContains `
    -Values ($exposureClasses | ForEach-Object { $_.class }) `
    -Expected "service_to_service" `
    -Message "exposure_classes"

$publicDerivedClass = Get-ExposureClass -Classes $exposureClasses -Class "public_derived"
Assert-Equals `
    -Actual $publicDerivedClass.direct_browser_access `
    -Expected "allowed" `
    -Message "public_derived direct browser access"
Assert-Equals `
    -Actual $publicDerivedClass.confidentiality_guarantee `
    -Expected "none" `
    -Message "public_derived confidentiality guarantee"
foreach ($control in @("data_minimization", "rate_limit", "response_budget_or_aggregate_only", "abuse_telemetry")) {
    Assert-ArrayContains `
        -Values @($publicDerivedClass.required_controls) `
        -Expected $control `
        -Message "public_derived required controls"
}
foreach ($dataClass in $requiredPublicForbiddenDataClasses) {
    Assert-ArrayContains `
        -Values @($publicDerivedClass.forbidden_data_classes) `
        -Expected $dataClass `
        -Message "public_derived forbidden data classes"
}

$serviceToServiceClass = Get-ExposureClass -Classes $exposureClasses -Class "service_to_service"
Assert-Equals `
    -Actual $serviceToServiceClass.browser_visible `
    -Expected $false `
    -Message "service_to_service browser visibility"
Assert-Equals `
    -Actual $serviceToServiceClass.direct_browser_access `
    -Expected "forbidden" `
    -Message "service_to_service direct browser access"
Assert-ArrayContains `
    -Values @($serviceToServiceClass.target_required_controls) `
    -Expected "mtls_or_short_lived_service_identity" `
    -Message "service_to_service target controls"

$publicRoutes = @($registry.public_route_policies)
Assert-Equals -Actual $publicRoutes.Count -Expected 6 -Message "public_route_policies count mismatch"
Assert-Unique -Values ($publicRoutes | ForEach-Object { $_.id }) -Message "public route policy ids must be unique"

$proxy = Read-TextFile -RelativePath "apps/web/proxy.ts"
$routesTs = Read-TextFile -RelativePath "apps/web/lib/routes.ts"
$apiProxyRoute = Read-TextFile -RelativePath "apps/web/app/api/proxy/[...path]/route.ts"
$tsGenerated = Read-TextFile -RelativePath "apps/web/lib/policies/traffic-auth-policy.generated.ts"
$rustGenerated = Read-TextFile -RelativePath "services/api/src/listing_marker_policy.rs"
$rustTrafficGenerated = Read-TextFile -RelativePath "services/api/src/traffic_auth_policy.rs"
$serving = Read-ListingMarkerServingSources
$apiMain = (Read-TextFile -RelativePath "services/api/src/main.rs") + "`n" +
    (Read-TextFile -RelativePath "services/api/src/app.rs")
$apiRouteSources = $apiMain + "`n" + (Read-TextFile -RelativePath "services/api/src/routes/health.rs")
$boundary = Read-TextFile -RelativePath "docs/architecture/platform-core-boundary.v1.json"
$ciWorkflow = Read-TextFile -RelativePath ".github/workflows/ci.yml"

Assert-Contains `
    -Content $ciWorkflow `
    -Needle "./scripts/ci/check-traffic-auth-policy-registry.ps1" `
    -Message "CI traffic/auth policy registry gate"
Assert-Contains `
    -Content $ciWorkflow `
    -Needle "./scripts/ci/check-traffic-auth-policy-registry.tests.ps1" `
    -Message "CI traffic/auth policy registry tests gate"
Assert-Contains `
    -Content $ciWorkflow `
    -Needle "-IncludeProductionEdge" `
    -Message "CI traffic/auth production edge policy gate"
