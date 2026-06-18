Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-traffic-auth-policy-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-traffic-auth-policy-registry-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

. (Join-Path $PSScriptRoot "traffic-auth-policy-registry.tests.helpers.ps1")

function Assert-FileLineCountAtMost {
    param(
        [string] $Path,
        [int] $MaxLines
    )

    $lineCount = (Get-Content -LiteralPath $Path | Measure-Object -Line).Lines
    if ($lineCount -gt $MaxLines) {
        throw "$Path line count $lineCount exceeds $MaxLines"
    }
}

Assert-FileLineCountAtMost -Path $PSCommandPath -MaxLines 600
Assert-FileLineCountAtMost -Path $ScriptPath -MaxLines 600
$generatorScriptPath = Join-Path $PSScriptRoot "generate-traffic-auth-policy.ps1"
Assert-FileLineCountAtMost -Path $generatorScriptPath -MaxLines 600
$registryGeneratorScriptPath = Join-Path $PSScriptRoot "generate-traffic-auth-policy-registry.ps1"
Assert-FileLineCountAtMost -Path $registryGeneratorScriptPath -MaxLines 600

$checkerModuleRoot = Join-Path $PSScriptRoot "traffic-auth-policy-registry"
if (Test-Path -LiteralPath $checkerModuleRoot -PathType Container) {
    Get-ChildItem -LiteralPath $checkerModuleRoot -File -Filter "*.ps1" -Recurse |
        ForEach-Object {
            Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
        }
}

$generatorModuleRoot = Join-Path $PSScriptRoot "traffic-auth-policy-generator"
if (Test-Path -LiteralPath $generatorModuleRoot -PathType Container) {
    Get-ChildItem -LiteralPath $generatorModuleRoot -File -Filter "*.ps1" -Recurse |
        ForEach-Object {
            Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
        }
}

$registryFragmentRoot = Join-Path $RepoRoot "docs\architecture\traffic-auth-policy-registry"
if (!(Test-Path -LiteralPath $registryFragmentRoot -PathType Container)) {
    throw "traffic/auth registry fragment root is missing: $registryFragmentRoot"
}
Get-ChildItem -LiteralPath $registryFragmentRoot -File -Filter "*.json" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
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

    $missingCiWorkflowTrafficAuthRoot = Join-Path $TempRoot "missing-ci-workflow-traffic-auth"
    Write-MinimalRepo -Root $missingCiWorkflowTrafficAuthRoot -OmitCiWorkflowTrafficAuthPolicyGate
    $missingCiWorkflowTrafficAuth = Invoke-Checker -Root $missingCiWorkflowTrafficAuthRoot
    Assert-Equals $missingCiWorkflowTrafficAuth.ExitCode 1 "missing CI workflow traffic/auth gate exit code mismatch"
    Assert-Contains $missingCiWorkflowTrafficAuth.Output "CI traffic/auth policy registry gate"

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

    $missingAuthLogoutPolicyRoot = Join-Path $TempRoot "missing-auth-logout-policy"
    Write-MinimalRepo -Root $missingAuthLogoutPolicyRoot -OmitAuthLogoutPolicy
    $missingAuthLogoutPolicy = Invoke-Checker -Root $missingAuthLogoutPolicyRoot
    Assert-Equals $missingAuthLogoutPolicy.ExitCode 1 "missing auth logout policy exit code mismatch"
    Assert-Contains $missingAuthLogoutPolicy.Output "API.auth.logout"

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

    $unregisteredApiProxyClientUsageRoot = Join-Path $TempRoot "unregistered-api-proxy-client-usage"
    Write-MinimalRepo -Root $unregisteredApiProxyClientUsageRoot -AddUnregisteredApiProxyClientUsage
    $unregisteredApiProxyClientUsage = Invoke-Checker -Root $unregisteredApiProxyClientUsageRoot
    Assert-Equals $unregisteredApiProxyClientUsage.ExitCode 1 "unregistered API proxy client usage exit code mismatch"
    Assert-Contains $unregisteredApiProxyClientUsage.Output "me/notifications"

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

    $unregisteredBackendRouteRoot = Join-Path $TempRoot "unregistered-backend-route"
    Write-MinimalRepo -Root $unregisteredBackendRouteRoot -AddUnregisteredBackendRoute
    $unregisteredBackendRoute = Invoke-Checker -Root $unregisteredBackendRouteRoot
    Assert-Equals $unregisteredBackendRoute.ExitCode 1 "unregistered backend route exit code mismatch"
    Assert-Contains $unregisteredBackendRoute.Output "/unregistered"

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
    exit 0
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
