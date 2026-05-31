[CmdletBinding()]
param(
    [string] $Root = "",
    [switch] $IncludeProductionEdge
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ExpectedSchemaVersion = "gongzzang.traffic_auth_policy_registry.v1"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)
    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-TextFile {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)
    $content = Read-TextFile -RelativePath $RelativePath
    return $content | ConvertFrom-Json
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Content, [string] $Needle, [string] $Message)
    if (!$Content.Contains($Needle)) {
        throw "$Message missing '$Needle'"
    }
}

function Assert-RegexContains {
    param([string] $Content, [string] $Pattern, [string] $Message)
    if (![regex]::IsMatch($Content, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)) {
        throw "$Message missing pattern '$Pattern'"
    }
}

function Assert-NotContains {
    param([string] $Content, [string] $Needle, [string] $Message)
    if ($Content.Contains($Needle)) {
        throw "$Message must not contain '$Needle'"
    }
}

function Assert-Unique {
    param([object[]] $Values, [string] $Message)
    $seen = @{}
    foreach ($value in $Values) {
        $key = [string] $value
        if ($seen.ContainsKey($key)) {
            throw "$Message duplicate '$key'"
        }
        $seen[$key] = $true
    }
}

function Assert-ArrayContains {
    param([object[]] $Values, [string] $Expected, [string] $Message)
    foreach ($value in $Values) {
        if ([string] $value -eq $Expected) {
            return
        }
    }
    throw "$Message missing '$Expected'"
}

function Assert-ArrayNotContains {
    param([object[]] $Values, [string] $Forbidden, [string] $Message)
    foreach ($value in $Values) {
        if ([string] $value -eq $Forbidden) {
            throw "$Message must not contain '$Forbidden'"
        }
    }
}

function Assert-StringSetEquals {
    param([object[]] $Actual, [object[]] $Expected, [string] $Message)
    $actualValues = @($Actual | ForEach-Object { [string] $_ } | Sort-Object)
    $expectedValues = @($Expected | ForEach-Object { [string] $_ } | Sort-Object)
    $actualJoined = $actualValues -join ","
    $expectedJoined = $expectedValues -join ","
    if ($actualJoined -ne $expectedJoined) {
        throw "$Message expected=[$expectedJoined] actual=[$actualJoined]"
    }
}

function Get-RequiredProperty {
    param([object] $Object, [string] $Name, [string] $Message)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        throw "$Message missing '$Name'"
    }
    if ($null -eq $property.Value) {
        throw "$Message missing '$Name'"
    }
    return $property.Value
}

function Get-ExposureClass {
    param([object[]] $Classes, [string] $Class)
    foreach ($entry in $Classes) {
        if ($entry.class -eq $Class) {
            return $entry
        }
    }
    throw "Missing exposure class '$Class'"
}

function Get-RegexInt {
    param([string] $Content, [string] $Pattern, [string] $Field)
    $match = [regex]::Match($Content, $Pattern)
    if (!$match.Success) {
        throw "Could not find $Field with pattern $Pattern"
    }
    return [int64] (($match.Groups[1].Value) -replace "_", "")
}

function Assert-RegexInt {
    param([string] $Content, [string] $Pattern, [int64] $Expected, [string] $Field)
    $actual = Get-RegexInt -Content $Content -Pattern $Pattern -Field $Field
    Assert-Equals -Actual $actual -Expected $Expected -Message $Field
}

function Get-RouteById {
    param([object[]] $Routes, [string] $Id)
    foreach ($route in $Routes) {
        if ($route.id -eq $Id) {
            return $route
        }
    }
    throw "Missing public route policy id=$Id"
}

function Get-RuleBySourcePolicyId {
    param([object[]] $Rules, [string] $Id, [string] $Message)
    foreach ($rule in $Rules) {
        if ([string] $rule.source_policy_id -eq $Id) {
            return $rule
        }
    }
    throw "$Message missing source_policy_id=$Id"
}

function Assert-EdgeRateProjection {
    param([object] $ActualRate, [object] $ExpectedRate, [string] $ExpectedKeyStrategy, [string] $Message)
    Assert-Equals -Actual ([string] $ActualRate.key_strategy) -Expected $ExpectedKeyStrategy -Message "$Message key_strategy"
    Assert-Equals -Actual ([string] $ActualRate.key_prefix) -Expected ([string] $ExpectedRate.key_prefix) -Message "$Message key_prefix"
    Assert-Equals -Actual ([int64] $ActualRate.limit) -Expected ([int64] $ExpectedRate.limit) -Message "$Message limit"
    Assert-Equals -Actual ([int64] $ActualRate.window_seconds) -Expected ([int64] $ExpectedRate.window_seconds) -Message "$Message window_seconds"
    Assert-Equals -Actual ([string] $ActualRate.problem_type) -Expected ([string] $ExpectedRate.problem_type) -Message "$Message problem_type"
}

function Convert-PathKindToAwsWafPathMatch {
    param([string] $Kind)
    switch ($Kind) {
        "exact" { return "EXACT" }
        "prefix" { return "STARTS_WITH" }
        default { throw "Unsupported AWS WAFv2 path kind '$Kind'" }
    }
}

function Convert-RateToFiveMinuteLimit {
    param([object] $Rate)
    $limit = [int64] $Rate.limit
    $windowSeconds = [int64] $Rate.window_seconds
    if ($windowSeconds -le 0) {
        throw "Rate window_seconds must be positive for $($Rate.key_prefix)"
    }
    return [int64] [Math]::Ceiling(([double] $limit) * 300.0 / ([double] $windowSeconds))
}

function Resolve-AuthPathSource {
    param([string] $PathSource)
    switch ($PathSource) {
        "API.auth.login" { return "/api/auth/login" }
        "API.auth.callback" { return "/api/auth/callback" }
        "API.auth.refresh" { return "/api/auth/refresh" }
        default { throw "Unsupported auth path source '$PathSource'" }
    }
}

function Format-TsStringArray {
    param([object[]] $Values)
    $quotedValues = @($Values | ForEach-Object {
            $escaped = ([string] $_).Replace("\", "\\").Replace('"', '\"')
            "`"$escaped`""
        })
    return "[$($quotedValues -join ", ")]"
}

function Format-RustUserRoleArray {
    param([object[]] $Values)
    $roleValues = @($Values | ForEach-Object {
            $role = [string] $_
            if (!(@("Admin", "Broker", "Buyer", "Developer", "Enterprise", "Operator", "Seller") -contains $role)) {
                throw "invalid generated backend role: $role"
            }
            "UserRole::$role"
        })
    return "&[$($roleValues -join ", ")]"
}

$registry = Read-JsonFile -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
Assert-Equals -Actual $registry.schema_version -Expected $ExpectedSchemaVersion -Message "schema_version mismatch"
Assert-Equals -Actual $registry.repo_slug -Expected "gongzzang" -Message "repo_slug mismatch"

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
$apiProxyRoute = Read-TextFile -RelativePath "apps/web/app/api/proxy/[...path]/route.ts"
$tsGenerated = Read-TextFile -RelativePath "apps/web/lib/policies/traffic-auth-policy.generated.ts"
$rustGenerated = Read-TextFile -RelativePath "services/api/src/listing_marker_policy.rs"
$rustTrafficGenerated = Read-TextFile -RelativePath "services/api/src/traffic_auth_policy.rs"
$serving = Read-TextFile -RelativePath "services/api/src/listing_marker_serving.rs"
$apiMain = Read-TextFile -RelativePath "services/api/src/main.rs"
$apiRouteSources = $apiMain + "`n" + (Read-TextFile -RelativePath "services/api/src/routes/health.rs")
$boundary = Read-TextFile -RelativePath "docs/architecture/platform-core-boundary.v1.json"

$edgeProjection = $null
$awsWafRateRules = @()
$awsWafIdentityAwareRules = @()
$awsWafServiceIdentityRules = @()
$awsWafBlockedQueryShapeRules = @()

if ($IncludeProductionEdge) {
    $edgeGeneratedPath = Resolve-RepoPath -RelativePath "infrastructure/security/traffic-auth-edge-policy.generated.json"
    if (!(Test-Path -LiteralPath $edgeGeneratedPath -PathType Leaf)) {
        throw "traffic-auth edge policy projection missing: infrastructure/security/traffic-auth-edge-policy.generated.json"
    }
    $edgeGenerated = Get-Content -LiteralPath $edgeGeneratedPath -Raw -Encoding UTF8
    $edgeProjection = $edgeGenerated | ConvertFrom-Json
    $awsWafManifestPath = Resolve-RepoPath -RelativePath "infrastructure/security/aws-wafv2-edge-policy.generated.json"
    if (!(Test-Path -LiteralPath $awsWafManifestPath -PathType Leaf)) {
        throw "AWS WAFv2 edge manifest missing: infrastructure/security/aws-wafv2-edge-policy.generated.json"
    }
    $awsWafManifest = (Get-Content -LiteralPath $awsWafManifestPath -Raw -Encoding UTF8) | ConvertFrom-Json
    $pulumiProjectPath = Resolve-RepoPath -RelativePath "infrastructure/Pulumi.yaml"
    if (!(Test-Path -LiteralPath $pulumiProjectPath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 consumer missing: infrastructure/Pulumi.yaml"
    }
    $pulumiIndexPath = Resolve-RepoPath -RelativePath "infrastructure/index.ts"
    if (!(Test-Path -LiteralPath $pulumiIndexPath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 consumer missing: infrastructure/index.ts"
    }
    $pulumiPackagePath = Resolve-RepoPath -RelativePath "infrastructure/package.json"
    if (!(Test-Path -LiteralPath $pulumiPackagePath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 package missing: infrastructure/package.json"
    }
    $pulumiLocalPreviewPath = Resolve-RepoPath -RelativePath "infrastructure/Pulumi.local-preview.yaml"
    if (!(Test-Path -LiteralPath $pulumiLocalPreviewPath -PathType Leaf)) {
        throw "Pulumi local-preview stack missing: infrastructure/Pulumi.local-preview.yaml"
    }
    $pulumiProject = Read-TextFile -RelativePath "infrastructure/Pulumi.yaml"
    $pulumiIndex = Read-TextFile -RelativePath "infrastructure/index.ts"
    $pulumiPackage = Read-JsonFile -RelativePath "infrastructure/package.json"
    $pulumiLocalPreviewStack = Read-TextFile -RelativePath "infrastructure/Pulumi.local-preview.yaml"

    Assert-Equals `
        -Actual ([string] $edgeProjection.schema_version) `
        -Expected "gongzzang.traffic_auth_edge_policy_projection.v1" `
        -Message "traffic-auth edge policy schema_version"
    Assert-Equals `
        -Actual ([string] $edgeProjection.source_registry) `
        -Expected "docs/architecture/traffic-auth-policy-registry.v1.json" `
        -Message "traffic-auth edge policy source_registry"
    Assert-Equals `
        -Actual ([string] $edgeProjection.projection_kind) `
        -Expected "provider_neutral_edge_ingress" `
        -Message "traffic-auth edge policy projection_kind"
    foreach ($target in @("cloudfront", "aws_wafv2", "alb", "service_mesh")) {
        Assert-ArrayContains `
            -Values @($edgeProjection.generated_targets) `
            -Expected $target `
            -Message "traffic-auth edge policy generated_targets"
    }
    Assert-Equals `
        -Actual ([string] $awsWafManifest.schema_version) `
        -Expected "gongzzang.aws_wafv2_edge_policy_manifest.v1" `
        -Message "AWS WAFv2 edge manifest schema_version"
    Assert-Equals `
        -Actual ([string] $awsWafManifest.source_projection) `
        -Expected "infrastructure/security/traffic-auth-edge-policy.generated.json" `
        -Message "AWS WAFv2 edge manifest source_projection"
    Assert-Equals `
        -Actual ([string] $awsWafManifest.managed_by) `
        -Expected "pulumi" `
        -Message "AWS WAFv2 edge manifest managed_by"
    foreach ($scope in @("CLOUDFRONT", "REGIONAL")) {
        Assert-ArrayContains `
            -Values @($awsWafManifest.scope_options) `
            -Expected $scope `
            -Message "AWS WAFv2 edge manifest scope_options"
    }
    $awsWafRateRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "rate_based_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafIdentityAwareRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "identity_aware_application_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafServiceIdentityRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "service_identity_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafBlockedQueryShapeRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "blocked_query_shape_rules" `
            -Message "AWS WAFv2 edge manifest")
    Assert-Unique -Values ($awsWafRateRules | ForEach-Object { $_.source_policy_id }) -Message "AWS WAFv2 edge rate rule source_policy_id must be unique"
    Assert-Unique -Values ($awsWafRateRules | ForEach-Object { $_.priority }) -Message "AWS WAFv2 edge rate rule priority must be unique"
    Assert-Contains -Content $pulumiProject -Needle "runtime: nodejs" -Message "Pulumi AWS WAFv2 consumer project runtime"
    Assert-Contains -Content $pulumiIndex -Needle "@pulumi/aws" -Message "Pulumi AWS WAFv2 consumer AWS provider import"
    Assert-Contains -Content $pulumiIndex -Needle "aws.wafv2.WebAcl" -Message "Pulumi AWS WAFv2 consumer WebAcl resource"
    Assert-Contains -Content $pulumiIndex -Needle "wafRegionalResourceArn" -Message "Pulumi AWS WAFv2 association regional resource config"
    Assert-Contains -Content $pulumiIndex -Needle "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN" -Message "Pulumi AWS WAFv2 association preview env fallback"
    Assert-Contains -Content $pulumiIndex -Needle "aws.wafv2.WebAclAssociation" -Message "Pulumi AWS WAFv2 association resource"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:region: ap-northeast-2" -Message "Pulumi local-preview stack AWS region"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipCredentialsValidation" -Message "Pulumi local-preview stack credential validation skip"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipRequestingAccountId" -Message "Pulumi local-preview stack account ID request skip"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipMetadataApiCheck" -Message "Pulumi local-preview stack metadata check skip"
    Assert-NotContains `
        -Content $pulumiLocalPreviewStack `
        -Needle "wafRegionalResourceArn" `
        -Message "Pulumi local-preview stack must not persist regional association target"
    Assert-Contains `
        -Content $pulumiIndex `
        -Needle "security/aws-wafv2-edge-policy.generated.json" `
        -Message "Pulumi AWS WAFv2 consumer generated manifest input"
    foreach ($requiredManifestMember in @(
            "rate_based_rules",
            "blocked_query_shape_rules",
            "identity_aware_application_rules",
            "service_identity_rules"
        )) {
        Assert-Contains `
            -Content $pulumiIndex `
            -Needle $requiredManifestMember `
            -Message "Pulumi AWS WAFv2 consumer manifest member"
    }
    $pulumiDependencyNames = @()
    if ($null -ne $pulumiPackage.PSObject.Properties["dependencies"]) {
        $pulumiDependencyNames += @($pulumiPackage.dependencies.PSObject.Properties.Name)
    }
    if ($null -ne $pulumiPackage.PSObject.Properties["devDependencies"]) {
        $pulumiDependencyNames += @($pulumiPackage.devDependencies.PSObject.Properties.Name)
    }
    foreach ($dependencyName in @("@pulumi/aws", "@pulumi/pulumi", "pulumi")) {
        Assert-ArrayContains `
            -Values $pulumiDependencyNames `
            -Expected $dependencyName `
            -Message "Pulumi AWS WAFv2 package dependency"
    }
}

Assert-Contains -Content $proxy -Needle "GENERATED_PUBLIC_MAP_ROUTE_POLICIES" -Message "proxy generated public map policy import"
Assert-Contains -Content $proxy -Needle "GENERATED_AUTH_RATE_ROUTE_POLICIES" -Message "proxy generated auth rate policy import"
Assert-Contains -Content $proxy -Needle "GENERATED_PAGE_ROUTE_POLICIES" -Message "proxy generated page route policy import"
Assert-Contains -Content $proxy -Needle "getAuthRateRoutePolicy" -Message "proxy generated auth rate policy lookup"
Assert-Contains -Content $proxy -Needle "resolveAuthRateKey" -Message "proxy generated auth rate key resolver"
Assert-Contains -Content $proxy -Needle "getPageRoutePolicy" -Message "proxy generated page route policy lookup"
Assert-NotContains -Content $proxy -Needle "ADMIN_ROLES" -Message "proxy must not hardcode page admin roles"
Assert-NotContains -Content $proxy -Needle "BROKER_ALLOWED_ROLES" -Message "proxy must not hardcode broker page roles"
Assert-NotContains -Content $proxy -Needle "BROKER_GATED_RULES" -Message "proxy must not hardcode broker page routes"
Assert-Contains -Content $proxy -Needle "exposure: policy.exposure" -Message "proxy must carry generated public exposure metadata"
Assert-Contains -Content $apiProxyRoute -Needle "GENERATED_API_PROXY_ROUTE_POLICIES" -Message "API proxy route policy import"
Assert-Contains -Content $apiProxyRoute -Needle "getApiProxyRoutePolicy" -Message "API proxy route allow-list check"
Assert-Contains -Content $apiProxyRoute -Needle "proxy/route-not-allowed" -Message "API proxy route deny problem"
Assert-Contains -Content $apiProxyRoute -Needle "enforceApiProxyExposure" -Message "API proxy exposure gate"
Assert-Contains -Content $apiProxyRoute -Needle "sessionRequiredProblem" -Message "API proxy exposure gate session denial"
Assert-Contains -Content $apiProxyRoute -Needle "insufficientRoleProblem" -Message "API proxy exposure gate role denial"
Assert-Contains -Content $apiProxyRoute -Needle "policy.requiredRoles.includes" -Message "API proxy exposure gate generated privileged roles"
Assert-Contains -Content $apiProxyRoute -Needle "checkApiProxyRateLimit" -Message "API proxy generated rate limit gate"
Assert-Contains -Content $apiProxyRoute -Needle "resolveApiProxyRateKey" -Message "API proxy generated rate key resolver"
Assert-Contains -Content $apiProxyRoute -Needle "checkRate(" -Message "API proxy rate limiter"
Assert-Contains -Content $serving -Needle "crate::listing_marker_policy" -Message "listing marker serving generated policy import"
Assert-Contains -Content $apiMain -Needle "backend_authorization" -Message "backend authorization module"
Assert-Contains -Content $apiMain -Needle "backend_rate_limit" -Message "backend rate limit module"
Assert-Contains -Content $apiMain -Needle "traffic_auth_policy" -Message "backend traffic auth generated policy module"
Assert-Contains -Content $apiMain -Needle "BackendAuthorizationState" -Message "backend generated role state"
Assert-Contains -Content $apiMain -Needle "enforce_backend_roles" -Message "backend generated role middleware mount"
Assert-Contains -Content $apiMain -Needle "enforce_backend_rate_limit" -Message "backend rate limit middleware mount"
Assert-Contains -Content $apiMain -Needle "RedisBackendRateLimiter" -Message "backend Redis rate limiter"
Assert-Contains -Content $rustTrafficGenerated -Needle "BACKEND_RATE_POLICIES" -Message "generated Rust backend rate policies"
Assert-Contains -Content $rustTrafficGenerated -Needle "BACKEND_ROLE_POLICIES" -Message "generated Rust backend role policies"

$backendRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "backend_route_policies" `
        -Message "backend_route_policies")
if ($backendRoutePolicies.Count -eq 0) {
    throw "backend_route_policies must not be empty"
}
Assert-Unique -Values ($backendRoutePolicies | ForEach-Object { $_.id }) -Message "backend route policy ids must be unique"

$routeRateProfiles = @(Get-RequiredProperty `
        -Object $registry `
        -Name "route_rate_profiles" `
        -Message "route_rate_profiles")
if ($routeRateProfiles.Count -eq 0) {
    throw "route_rate_profiles must not be empty"
}
Assert-Unique -Values ($routeRateProfiles | ForEach-Object { $_.id }) -Message "route rate profile ids must be unique"
$routeRateProfileById = @{}
foreach ($profile in $routeRateProfiles) {
    $profileId = [string] $profile.id
    if ([string]::IsNullOrWhiteSpace($profileId)) {
        throw "route rate profile id invalid"
    }
    $keyPrefix = [string] $profile.key_prefix
    if ([string]::IsNullOrWhiteSpace($keyPrefix) -or $keyPrefix.Contains("..")) {
        throw "route rate profile key_prefix invalid for $($profileId): $keyPrefix"
    }
    $keyStrategy = [string] $profile.key_strategy
    if ($keyStrategy -ne "session_sub") {
        throw "route rate profile key_strategy invalid for $($profileId): $keyStrategy"
    }
    if ([int64] $profile.limit -le 0) {
        throw "route rate profile limit must be positive for $profileId"
    }
    if ([int64] $profile.window_seconds -le 0) {
        throw "route rate profile window_seconds must be positive for $profileId"
    }
    if ([string] $profile.problem_type -ne "proxy/too-many-requests") {
        throw "route rate profile problem_type invalid for $($profileId): $($profile.problem_type)"
    }
    $routeRateProfileById[$profileId] = $profile
}

$authRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "auth_route_policies" `
        -Message "auth_route_policies")
if ($authRoutePolicies.Count -eq 0) {
    throw "auth_route_policies must not be empty"
}
Assert-Unique -Values ($authRoutePolicies | ForEach-Object { $_.id }) -Message "auth route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_AUTH_RATE_ROUTE_POLICIES" -Message "generated TS auth rate policies"
$edgeAuthRules = @()
if ($IncludeProductionEdge) {
    $edgeAuthRules = @($edgeProjection.auth_route_rules)
    Assert-Equals -Actual $edgeAuthRules.Count -Expected $authRoutePolicies.Count -Message "traffic-auth edge auth_route_rules count"
}

foreach ($routePolicy in $authRoutePolicies) {
    if ($IncludeProductionEdge) {
        $edgeAuthRule = Get-RuleBySourcePolicyId `
            -Rules $edgeAuthRules `
            -Id ([string] $routePolicy.id) `
            -Message "traffic-auth edge auth route rule"
        Assert-Equals `
            -Actual ([string] $edgeAuthRule.path_source) `
            -Expected ([string] $routePolicy.path_source) `
            -Message "traffic-auth edge auth route path_source for $($routePolicy.id)"
        Assert-StringSetEquals `
            -Actual @($edgeAuthRule.methods) `
            -Expected @($routePolicy.methods) `
            -Message "traffic-auth edge auth route methods for $($routePolicy.id)"
    }

    $pathSource = [string] $routePolicy.path_source
    if (!(@("API.auth.login", "API.auth.callback", "API.auth.refresh") -contains $pathSource)) {
        throw "auth route policy path_source invalid for $($routePolicy.id): $pathSource"
    }

    $methods = @($routePolicy.methods)
    if ($methods.Count -eq 0) {
        throw "auth route policy methods missing for $($routePolicy.id)"
    }
    foreach ($method in $methods) {
        $methodString = [string] $method
        if (!(@("GET", "POST") -contains $methodString)) {
            throw "auth route policy method invalid for $($routePolicy.id): $methodString"
        }
    }

    $ratePolicy = Get-RequiredProperty `
        -Object $routePolicy `
        -Name "rate_policy" `
        -Message "auth route rate_policy for $($routePolicy.id)"
    $keyPrefix = [string] $ratePolicy.key_prefix
    if ([string]::IsNullOrWhiteSpace($keyPrefix) -or $keyPrefix.Contains("..")) {
        throw "auth route rate_policy key_prefix invalid for $($routePolicy.id): $keyPrefix"
    }
    $keyStrategy = [string] $ratePolicy.key_strategy
    if (!(@("client_ip", "session_or_anon") -contains $keyStrategy)) {
        throw "auth route rate_policy key_strategy invalid for $($routePolicy.id): $keyStrategy"
    }
    if ([int64] $ratePolicy.limit -le 0) {
        throw "auth route rate_policy limit must be positive for $($routePolicy.id)"
    }
    if ([int64] $ratePolicy.window_seconds -le 0) {
        throw "auth route rate_policy window_seconds must be positive for $($routePolicy.id)"
    }
    if ([string] $ratePolicy.problem_type -ne "auth/too-many-requests") {
        throw "auth route rate_policy problem_type invalid for $($routePolicy.id): $($ratePolicy.problem_type)"
    }
    if ($IncludeProductionEdge) {
        Assert-EdgeRateProjection `
            -ActualRate $edgeAuthRule.rate `
            -ExpectedRate $ratePolicy `
            -ExpectedKeyStrategy $keyStrategy `
            -Message "traffic-auth edge auth route rate for $($routePolicy.id)"
        if ($keyStrategy -eq "client_ip") {
            $awsWafRateRule = Get-RuleBySourcePolicyId `
                -Rules $awsWafRateRules `
                -Id ([string] $routePolicy.id) `
                -Message "AWS WAFv2 edge auth rate rule"
            Assert-Equals `
                -Actual ([string] $awsWafRateRule.aggregate_key_type) `
                -Expected "IP" `
                -Message "AWS WAFv2 edge auth rate aggregate_key_type for $($routePolicy.id)"
            Assert-Equals `
                -Actual ([int64] $awsWafRateRule.limit_per_5m) `
                -Expected (Convert-RateToFiveMinuteLimit -Rate $ratePolicy) `
                -Message "AWS WAFv2 edge auth rate limit_per_5m for $($routePolicy.id)"
            Assert-Equals `
                -Actual ([string] $awsWafRateRule.match.path_source) `
                -Expected $pathSource `
                -Message "AWS WAFv2 edge auth rate path_source for $($routePolicy.id)"
            Assert-Equals `
                -Actual ([string] $awsWafRateRule.match.path) `
                -Expected (Resolve-AuthPathSource -PathSource $pathSource) `
                -Message "AWS WAFv2 edge auth rate path for $($routePolicy.id)"
            Assert-Equals `
                -Actual ([string] $awsWafRateRule.match.path_match) `
                -Expected "EXACT" `
                -Message "AWS WAFv2 edge auth rate path_match for $($routePolicy.id)"
            Assert-StringSetEquals `
                -Actual @($awsWafRateRule.match.methods) `
                -Expected @($routePolicy.methods) `
                -Message "AWS WAFv2 edge auth rate methods for $($routePolicy.id)"
        } else {
            $identityAwareRule = Get-RuleBySourcePolicyId `
                -Rules $awsWafIdentityAwareRules `
                -Id ([string] $routePolicy.id) `
                -Message "AWS WAFv2 identity-aware application rule"
            Assert-Equals `
                -Actual ([string] $identityAwareRule.reason) `
                -Expected "key_strategy_not_representable_in_wafv2" `
                -Message "AWS WAFv2 identity-aware application reason for $($routePolicy.id)"
        }
    }

    Assert-Contains -Content $tsGenerated -Needle $pathSource -Message "generated TS auth rate path source for $($routePolicy.id)"
    Assert-Contains -Content $tsGenerated -Needle $keyPrefix -Message "generated TS auth rate key prefix for $($routePolicy.id)"
    Assert-Contains -Content $tsGenerated -Needle "keyStrategy: `"$keyStrategy`"" -Message "generated TS auth rate key strategy for $($routePolicy.id)"
    Assert-Contains -Content $tsGenerated -Needle "limit: $($ratePolicy.limit)" -Message "generated TS auth rate limit for $($routePolicy.id)"
    Assert-Contains -Content $tsGenerated -Needle "windowSec: $($ratePolicy.window_seconds)" -Message "generated TS auth rate window for $($routePolicy.id)"
    Assert-Contains -Content $tsGenerated -Needle "problemType: `"$($ratePolicy.problem_type)`"" -Message "generated TS auth rate problem type for $($routePolicy.id)"
    Assert-NotContains -Content $proxy -Needle $keyPrefix -Message "proxy must not hardcode generated auth rate key prefix for $($routePolicy.id)"
}

$pageRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "page_route_policies" `
        -Message "page_route_policies")
if ($pageRoutePolicies.Count -eq 0) {
    throw "page_route_policies must not be empty"
}
Assert-Unique -Values ($pageRoutePolicies | ForEach-Object { $_.id }) -Message "page route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_PAGE_ROUTE_POLICIES" -Message "generated TS page route policies"
$apiProxyRoutePoliciesForPageAlignment = @(Get-RequiredProperty `
        -Object $registry `
        -Name "api_proxy_route_policies" `
        -Message "api_proxy_route_policies")
$listingPageToApiPolicyIds = @{
    "gongzzang.page.listing_create" = "gongzzang.api_proxy.listings_collection_create"
    "gongzzang.page.listing_edit"   = "gongzzang.api_proxy.listing_detail_update"
}

foreach ($routePolicy in $pageRoutePolicies) {
    $kind = [string] $routePolicy.path_kind
    if (!(@("exact", "prefix", "prefix_suffix") -contains $kind)) {
        throw "page route policy path_kind invalid for $($routePolicy.id): $kind"
    }

    $pathProperty = $routePolicy.PSObject.Properties["path"]
    $pathSourceProperty = $routePolicy.PSObject.Properties["path_source"]
    $prefixProperty = $routePolicy.PSObject.Properties["prefix"]
    $prefixSourceProperty = $routePolicy.PSObject.Properties["prefix_source"]
    $suffixProperty = $routePolicy.PSObject.Properties["suffix"]

    if ($kind -eq "prefix_suffix") {
        if ($null -eq $suffixProperty -or [string]::IsNullOrWhiteSpace([string] $suffixProperty.Value)) {
            throw "page route policy suffix missing for $($routePolicy.id)"
        }
        if ($null -eq $prefixProperty -and $null -eq $prefixSourceProperty) {
            throw "page route policy prefix or prefix_source missing for $($routePolicy.id)"
        }
    } else {
        if ($null -eq $pathProperty -and $null -eq $pathSourceProperty) {
            throw "page route policy path or path_source missing for $($routePolicy.id)"
        }
    }

    $requiredRoles = @($routePolicy.required_roles)
    if ($requiredRoles.Count -eq 0) {
        throw "page route policy required_roles missing for $($routePolicy.id)"
    }
    foreach ($role in $requiredRoles) {
        $roleString = [string] $role
        if (!(@("Admin", "Broker", "Operator", "Buyer") -contains $roleString)) {
            throw "page route policy required role invalid for $($routePolicy.id): $roleString"
        }
    }

    Assert-Contains -Content $tsGenerated -Needle "kind: `"$kind`"" -Message "generated TS page route kind for $($routePolicy.id)"
    Assert-Contains `
        -Content $tsGenerated `
        -Needle "requiredRoles: $(Format-TsStringArray -Values $requiredRoles)" `
        -Message "generated TS page route required roles for $($routePolicy.id)"
    if ($null -ne $pathProperty) {
        Assert-Contains -Content $tsGenerated -Needle "path: `"$([string] $pathProperty.Value)`"" -Message "generated TS page route path for $($routePolicy.id)"
    }
    if ($null -ne $pathSourceProperty) {
        Assert-Contains -Content $tsGenerated -Needle "pathSource: `"$([string] $pathSourceProperty.Value)`"" -Message "generated TS page route path source for $($routePolicy.id)"
    }
    if ($null -ne $prefixProperty) {
        Assert-Contains -Content $tsGenerated -Needle "prefix: `"$([string] $prefixProperty.Value)`"" -Message "generated TS page route prefix for $($routePolicy.id)"
    }
    if ($null -ne $prefixSourceProperty) {
        Assert-Contains -Content $tsGenerated -Needle "prefixSource: `"$([string] $prefixSourceProperty.Value)`"" -Message "generated TS page route prefix source for $($routePolicy.id)"
    }
    if ($null -ne $suffixProperty) {
        Assert-Contains -Content $tsGenerated -Needle "suffix: `"$([string] $suffixProperty.Value)`"" -Message "generated TS page route suffix for $($routePolicy.id)"
    }

    if ($listingPageToApiPolicyIds.ContainsKey([string] $routePolicy.id)) {
        $apiPolicyId = [string] $listingPageToApiPolicyIds[[string] $routePolicy.id]
        $matchingApiPolicies = @($apiProxyRoutePoliciesForPageAlignment | Where-Object { [string] $_.id -eq $apiPolicyId })
        if ($matchingApiPolicies.Count -ne 1) {
            throw "listing page route roles cannot find matching API proxy route policy for $($routePolicy.id): $apiPolicyId"
        }
        $apiPolicy = $matchingApiPolicies[0]
        if ([string] $apiPolicy.exposure_class -ne "privileged") {
            throw "listing page route roles must map to privileged API route for $($routePolicy.id): $apiPolicyId"
        }
        $apiRequiredRolesProperty = $apiPolicy.PSObject.Properties["required_roles"]
        if ($null -eq $apiRequiredRolesProperty) {
            throw "listing page route roles required_roles missing on API proxy route for $($routePolicy.id): $apiPolicyId"
        }
        $apiRequiredRoles = @($apiRequiredRolesProperty.Value)
        $pageRoleSet = @($requiredRoles | Sort-Object) -join ","
        $apiRoleSet = @($apiRequiredRoles | Sort-Object) -join ","
        if ($pageRoleSet -ne $apiRoleSet) {
            throw "listing page route roles must match API proxy route roles for $($routePolicy.id): page=[$pageRoleSet] api=[$apiRoleSet]"
        }
    }
}

foreach ($routePolicy in $backendRoutePolicies) {
    $path = [string] $routePolicy.path
    if ([string]::IsNullOrWhiteSpace($path) -or !$path.StartsWith("/") -or $path.Contains("..")) {
        throw "backend route policy path must be absolute and normalized for $($routePolicy.id): $path"
    }
    Assert-Contains -Content $apiRouteSources -Needle $path -Message "backend route policy Rust path for $($routePolicy.id)"

    $routerGroup = [string] $routePolicy.router_group
    if (!(@("public_health", "public_marker", "protected", "internal") -contains $routerGroup)) {
        throw "backend route policy router_group invalid for $($routePolicy.id): $routerGroup"
    }

    $exposureClass = [string] $routePolicy.exposure_class
    if (!(@("public_health", "public_derived", "authenticated_user", "privileged", "service_to_service") -contains $exposureClass)) {
        throw "backend route policy exposure_class invalid for $($routePolicy.id): $exposureClass"
    }

    $authPolicy = [string] $routePolicy.auth_policy
    if ($routerGroup -eq "protected") {
        Assert-Equals -Actual $authPolicy -Expected "bearer_jwt" -Message "backend protected auth policy for $($routePolicy.id)"
        Assert-Contains -Content $apiMain -Needle "auth_layer" -Message "backend protected route auth_layer"
    } elseif ($routerGroup -eq "internal") {
        Assert-Equals -Actual $authPolicy -Expected "internal_shared_secret" -Message "backend internal auth policy for $($routePolicy.id)"
        Assert-Contains -Content $apiMain -Needle "build_internal_auth_secret" -Message "backend internal shared secret builder"
        Assert-Contains -Content $apiMain -Needle "internal_auth_secret" -Message "backend internal shared secret state"
    } else {
        Assert-Equals -Actual $authPolicy -Expected "anonymous_public" -Message "backend public auth policy for $($routePolicy.id)"
        if ($exposureClass -ne "public_health" -and $exposureClass -ne "public_derived") {
            throw "backend public route exposure_class invalid for $($routePolicy.id): $exposureClass"
        }
    }

    $methods = @($routePolicy.methods)
    if ($methods.Count -eq 0) {
        throw "backend route policy methods missing for $($routePolicy.id)"
    }
    foreach ($method in $methods) {
        $methodString = [string] $method
        if (!(@("GET", "POST", "PUT", "PATCH", "DELETE") -contains $methodString)) {
            throw "backend route policy method invalid for $($routePolicy.id): $methodString"
        }
    }

    $requiredRoles = @()
    $requiredRolesProperty = $routePolicy.PSObject.Properties["required_roles"]
    if ($null -ne $requiredRolesProperty) {
        $requiredRoles = @($requiredRolesProperty.Value)
    }
    if ($exposureClass -eq "privileged") {
        if ($requiredRoles.Count -eq 0) {
            throw "backend route policy required_roles missing for privileged route $($routePolicy.id)"
        }
        $requiredRoleLiteral = Format-RustUserRoleArray -Values $requiredRoles
        Assert-Contains -Content $rustTrafficGenerated -Needle $requiredRoleLiteral -Message "generated Rust backend required roles for $($routePolicy.id)"
        foreach ($method in $methods) {
            $rolePolicyPattern = "BackendRolePolicy\s*\{\s*method:\s*`"$([regex]::Escape([string] $method))`",\s*path_pattern:\s*`"$([regex]::Escape($path))`",\s*required_roles:\s*$([regex]::Escape($requiredRoleLiteral)),\s*\}"
            Assert-RegexContains -Content $rustTrafficGenerated -Pattern $rolePolicyPattern -Message "generated Rust backend role policy for $($routePolicy.id)"
        }
    } elseif ($requiredRoles.Count -ne 0) {
        throw "backend route policy required_roles only valid for privileged routes: $($routePolicy.id)"
    }

    $backendRateProfileProperty = $routePolicy.PSObject.Properties["rate_profile"]
    if ($routerGroup -eq "protected") {
        if ($null -eq $backendRateProfileProperty -or [string]::IsNullOrWhiteSpace([string] $backendRateProfileProperty.Value)) {
            throw "backend route policy rate_profile missing for protected route $($routePolicy.id)"
        }
        $backendRateProfileId = [string] $backendRateProfileProperty.Value
        if (!$routeRateProfileById.ContainsKey($backendRateProfileId)) {
            throw "backend route policy rate_profile unknown for $($routePolicy.id): $backendRateProfileId"
        }
        $backendRateProfile = $routeRateProfileById[$backendRateProfileId]
        foreach ($method in $methods) {
            Assert-Contains -Content $rustTrafficGenerated -Needle "method: `"$method`"" -Message "generated Rust backend rate method for $($routePolicy.id)"
        }
        Assert-Contains -Content $rustTrafficGenerated -Needle "path_pattern: `"$path`"" -Message "generated Rust backend rate path for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "key_prefix: `"$($backendRateProfile.key_prefix)`"" -Message "generated Rust backend rate key prefix for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "limit: $($backendRateProfile.limit)" -Message "generated Rust backend rate limit for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "window_seconds: $($backendRateProfile.window_seconds)" -Message "generated Rust backend rate window for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "problem_type: `"$($backendRateProfile.problem_type)`"" -Message "generated Rust backend rate problem type for $($routePolicy.id)"
    } elseif ($null -ne $backendRateProfileProperty) {
        throw "backend route policy rate_profile only valid for protected routes: $($routePolicy.id)"
    }
}

$apiProxyRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "api_proxy_route_policies" `
        -Message "api_proxy_route_policies")
if ($apiProxyRoutePolicies.Count -eq 0) {
    throw "api_proxy_route_policies must not be empty"
}
Assert-Unique -Values ($apiProxyRoutePolicies | ForEach-Object { $_.id }) -Message "API proxy route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_API_PROXY_ROUTE_POLICIES" -Message "generated TS API proxy route policies"
$edgeApiProxyRules = @()
if ($IncludeProductionEdge) {
    $edgeApiProxyRules = @($edgeProjection.api_proxy_route_rules)
    Assert-Equals -Actual $edgeApiProxyRules.Count -Expected $apiProxyRoutePolicies.Count -Message "traffic-auth edge api_proxy_route_rules count"
}

foreach ($routePolicy in $apiProxyRoutePolicies) {
    if ($IncludeProductionEdge) {
        $edgeApiProxyRule = Get-RuleBySourcePolicyId `
            -Rules $edgeApiProxyRules `
            -Id ([string] $routePolicy.id) `
            -Message "traffic-auth edge API proxy route rule"
    }
    $kind = [string] $routePolicy.target_path_kind
    if ($kind -ne "exact" -and $kind -ne "prefix" -and $kind -ne "template") {
        throw "API proxy route policy target_path_kind invalid for $($routePolicy.id): $kind"
    }
    $targetPath = [string] $routePolicy.target_path
    if ([string]::IsNullOrWhiteSpace($targetPath)) {
        throw "API proxy route policy target_path missing for $($routePolicy.id)"
    }
    if ($targetPath.StartsWith("/") -or $targetPath.Contains("..")) {
        throw "API proxy route policy target_path must be relative and normalized for $($routePolicy.id): $targetPath"
    }
    if ($targetPath.StartsWith("internal/") -or $targetPath.Contains("raw-listing-export")) {
        throw "API proxy route policy must not expose internal or raw export paths for $($routePolicy.id): $targetPath"
    }
    if ($IncludeProductionEdge) {
        Assert-Equals `
            -Actual ([string] $edgeApiProxyRule.edge_path) `
            -Expected "/api/proxy/$targetPath" `
            -Message "traffic-auth edge API proxy route edge_path for $($routePolicy.id)"
        Assert-Equals `
            -Actual ([string] $edgeApiProxyRule.target_path) `
            -Expected $targetPath `
            -Message "traffic-auth edge API proxy route target_path for $($routePolicy.id)"
        Assert-Equals `
            -Actual ([string] $edgeApiProxyRule.target_path_kind) `
            -Expected $kind `
            -Message "traffic-auth edge API proxy route target_path_kind for $($routePolicy.id)"
    }

    $methods = @($routePolicy.methods)
    if ($methods.Count -eq 0) {
        throw "API proxy route policy methods missing for $($routePolicy.id)"
    }
    foreach ($method in $methods) {
        $methodString = [string] $method
        if (!(@("GET", "POST", "PUT", "PATCH", "DELETE") -contains $methodString)) {
            throw "API proxy route policy method invalid for $($routePolicy.id): $methodString"
        }
    }
    if ($IncludeProductionEdge) {
        Assert-StringSetEquals `
            -Actual @($edgeApiProxyRule.methods) `
            -Expected $methods `
            -Message "traffic-auth edge API proxy route methods for $($routePolicy.id)"
    }

    $exposureClass = [string] $routePolicy.exposure_class
    if (!(@("public_derived", "authenticated_user", "privileged") -contains $exposureClass)) {
        throw "API proxy route policy exposure_class invalid for $($routePolicy.id): $exposureClass"
    }
    $requiredRoles = @()
    $requiredRolesProperty = $routePolicy.PSObject.Properties["required_roles"]
    if ($null -ne $requiredRolesProperty) {
        $requiredRoles = @($requiredRolesProperty.Value)
    }
    if ($exposureClass -eq "privileged") {
        if ($requiredRoles.Count -eq 0) {
            throw "API proxy route policy required_roles missing for privileged route $($routePolicy.id)"
        }
        Assert-ArrayContains `
            -Values $requiredRoles `
            -Expected "Broker" `
            -Message "API proxy route policy privileged required roles for $($routePolicy.id)"
    } elseif ($requiredRoles.Count -ne 0) {
        throw "API proxy route policy required_roles only valid for privileged routes: $($routePolicy.id)"
    }
    if ($IncludeProductionEdge) {
        Assert-Equals `
            -Actual ([string] $edgeApiProxyRule.exposure_class) `
            -Expected $exposureClass `
            -Message "traffic-auth edge API proxy route exposure_class for $($routePolicy.id)"
        Assert-StringSetEquals `
            -Actual @($edgeApiProxyRule.required_roles) `
            -Expected $requiredRoles `
            -Message "traffic-auth edge API proxy route required_roles for $($routePolicy.id)"
    }

    $rateProfileProperty = $routePolicy.PSObject.Properties["rate_profile"]
    if ($exposureClass -eq "public_derived") {
        if ($null -ne $rateProfileProperty) {
            throw "API proxy public_derived route policy must use public_route_policies rate_policy, not rate_profile: $($routePolicy.id)"
        }
        if ($IncludeProductionEdge -and $null -ne $edgeApiProxyRule.PSObject.Properties["rate"]) {
            throw "traffic-auth edge API proxy public route must not carry route_rate_profiles rate: $($routePolicy.id)"
        }
    } else {
        if ($null -eq $rateProfileProperty -or [string]::IsNullOrWhiteSpace([string] $rateProfileProperty.Value)) {
            throw "API proxy route policy rate_profile missing for $($routePolicy.id)"
        }
        $rateProfileId = [string] $rateProfileProperty.Value
        if (!$routeRateProfileById.ContainsKey($rateProfileId)) {
            throw "API proxy route policy rate_profile unknown for $($routePolicy.id): $rateProfileId"
        }
        $rateProfile = $routeRateProfileById[$rateProfileId]
        Assert-Contains -Content $tsGenerated -Needle "keyPrefix: `"$($rateProfile.key_prefix)`"" -Message "generated TS API proxy rate key prefix for $($routePolicy.id)"
        Assert-Contains -Content $tsGenerated -Needle "keyStrategy: `"$($rateProfile.key_strategy)`"" -Message "generated TS API proxy rate key strategy for $($routePolicy.id)"
        Assert-Contains -Content $tsGenerated -Needle "limit: $($rateProfile.limit)" -Message "generated TS API proxy rate limit for $($routePolicy.id)"
        Assert-Contains -Content $tsGenerated -Needle "windowSec: $($rateProfile.window_seconds)" -Message "generated TS API proxy rate window for $($routePolicy.id)"
        Assert-Contains -Content $tsGenerated -Needle "problemType: `"$($rateProfile.problem_type)`"" -Message "generated TS API proxy rate problem type for $($routePolicy.id)"
        if ($IncludeProductionEdge) {
            Assert-EdgeRateProjection `
                -ActualRate $edgeApiProxyRule.rate `
                -ExpectedRate $rateProfile `
                -ExpectedKeyStrategy ([string] $rateProfile.key_strategy) `
                -Message "traffic-auth edge API proxy route rate for $($routePolicy.id)"
            if ([string] $rateProfile.key_strategy -ne "client_ip") {
                $identityAwareRule = Get-RuleBySourcePolicyId `
                    -Rules $awsWafIdentityAwareRules `
                    -Id ([string] $routePolicy.id) `
                    -Message "AWS WAFv2 identity-aware application rule"
                Assert-Equals `
                    -Actual ([string] $identityAwareRule.reason) `
                    -Expected "key_strategy_not_representable_in_wafv2" `
                    -Message "AWS WAFv2 identity-aware application reason for $($routePolicy.id)"
            }
        }
    }

    Assert-Contains -Content $tsGenerated -Needle $targetPath -Message "generated TS API proxy target path for $($routePolicy.id)"
    Assert-Contains `
        -Content $tsGenerated `
        -Needle "requiredRoles: $(Format-TsStringArray -Values $requiredRoles)" `
        -Message "generated TS API proxy required roles for $($routePolicy.id)"

    foreach ($method in $methods) {
        $backendPath = "/$targetPath"
        $matchingBackend = @($backendRoutePolicies | Where-Object {
                [string] $_.path -eq $backendPath -and
                @($_.methods | ForEach-Object { [string] $_ }) -contains ([string] $method) -and
                [string] $_.exposure_class -eq $exposureClass
            })
        if ($matchingBackend.Count -eq 0) {
            throw "API proxy route policy has no matching backend route policy: $($routePolicy.id) $method $backendPath $exposureClass"
        }
    }
}

$edgePublicRules = @()
if ($IncludeProductionEdge) {
    $edgePublicRules = @($edgeProjection.public_route_rules)
    Assert-Equals -Actual $edgePublicRules.Count -Expected $publicRoutes.Count -Message "traffic-auth edge public_route_rules count"
}

foreach ($route in $publicRoutes) {
    if ($IncludeProductionEdge) {
        $edgePublicRule = Get-RuleBySourcePolicyId `
            -Rules $edgePublicRules `
            -Id ([string] $route.id) `
            -Message "traffic-auth edge public route rule"
        Assert-Equals `
            -Actual ([string] $edgePublicRule.proxy_path) `
            -Expected ([string] $route.proxy_path) `
            -Message "traffic-auth edge public route proxy_path for $($route.id)"
        Assert-Equals `
            -Actual ([string] $edgePublicRule.backend_route) `
            -Expected ([string] $route.backend_route) `
            -Message "traffic-auth edge public route backend_route for $($route.id)"
        Assert-Equals `
            -Actual ([string] $edgePublicRule.exposure_class) `
            -Expected "public_derived" `
            -Message "traffic-auth edge public route exposure_class for $($route.id)"
        Assert-StringSetEquals `
            -Actual @($edgePublicRule.methods) `
            -Expected @($route.methods) `
            -Message "traffic-auth edge public route methods for $($route.id)"
        Assert-EdgeRateProjection `
            -ActualRate $edgePublicRule.rate `
            -ExpectedRate $route.rate_policy `
            -ExpectedKeyStrategy "client_ip" `
            -Message "traffic-auth edge public route rate for $($route.id)"
        $awsWafRateRule = Get-RuleBySourcePolicyId `
            -Rules $awsWafRateRules `
            -Id ([string] $route.id) `
            -Message "AWS WAFv2 edge public rate rule"
        Assert-Equals `
            -Actual ([string] $awsWafRateRule.aggregate_key_type) `
            -Expected "IP" `
            -Message "AWS WAFv2 edge public rate aggregate_key_type for $($route.id)"
        Assert-Equals `
            -Actual ([int64] $awsWafRateRule.limit_per_5m) `
            -Expected (Convert-RateToFiveMinuteLimit -Rate $route.rate_policy) `
            -Message "AWS WAFv2 edge public rate limit_per_5m for $($route.id)"
        Assert-Equals `
            -Actual ([string] $awsWafRateRule.match.path) `
            -Expected ([string] $route.proxy_path) `
            -Message "AWS WAFv2 edge public rate path for $($route.id)"
        Assert-Equals `
            -Actual ([string] $awsWafRateRule.match.path_match) `
            -Expected (Convert-PathKindToAwsWafPathMatch -Kind ([string] $route.proxy_path_kind) ) `
            -Message "AWS WAFv2 edge public rate path_match for $($route.id)"
        Assert-StringSetEquals `
            -Actual @($awsWafRateRule.match.methods) `
            -Expected @($route.methods) `
            -Message "AWS WAFv2 edge public rate methods for $($route.id)"
    }
    $forbiddenShapeProperty = $route.PSObject.Properties["forbidden_request_shapes"]
    $forbiddenShapes = @()
    if ($null -ne $forbiddenShapeProperty) {
        $forbiddenShapes = @($forbiddenShapeProperty.Value)
    }
    foreach ($forbiddenShape in $forbiddenShapes) {
        if (!$IncludeProductionEdge) {
            continue
        }
        Assert-ArrayContains `
            -Values @($edgePublicRule.forbidden_request_shapes) `
            -Expected ([string] $forbiddenShape) `
            -Message "traffic-auth edge public route forbidden_request_shapes for $($route.id)"
    }
    if ($IncludeProductionEdge -and $forbiddenShapes.Count -ne 0) {
        $blockedQueryShapeRule = Get-RuleBySourcePolicyId `
            -Rules $awsWafBlockedQueryShapeRules `
            -Id ([string] $route.id) `
            -Message "AWS WAFv2 blocked query shape rule"
        Assert-Equals `
            -Actual ([string] $blockedQueryShapeRule.action) `
            -Expected "BLOCK" `
            -Message "AWS WAFv2 blocked query shape action for $($route.id)"
        Assert-Equals `
            -Actual ([string] $blockedQueryShapeRule.match.path) `
            -Expected ([string] $route.proxy_path) `
            -Message "AWS WAFv2 blocked query shape path for $($route.id)"
        foreach ($forbiddenShape in $forbiddenShapes) {
            Assert-ArrayContains `
                -Values @($blockedQueryShapeRule.match.query_parameters) `
                -Expected ([string] $forbiddenShape) `
                -Message "AWS WAFv2 blocked query shape parameter for $($route.id)"
        }
    }

    Assert-Equals -Actual $route.exposure -Expected "public_anonymous" -Message "public route exposure for $($route.id)"
    $publicMethods = @($route.methods)
    if ($publicMethods.Count -eq 0) {
        throw "public route methods missing for $($route.id)"
    }
    foreach ($method in $publicMethods) {
        $methodString = [string] $method
        if (!(@("GET", "POST") -contains $methodString)) {
            throw "public route method invalid for $($route.id): $methodString"
        }
        Assert-Contains -Content $rustTrafficGenerated -Needle "method: `"$methodString`"" -Message "generated Rust public backend rate method for $($route.id)"
    }
    $authPolicy = Get-RequiredProperty -Object $route -Name "auth_policy" -Message "public route auth_policy for $($route.id)"
    Assert-Equals -Actual $authPolicy.method -Expected "anonymous_public" -Message "public route auth method for $($route.id)"
    Assert-Equals -Actual $authPolicy.session_required -Expected $false -Message "public route session policy for $($route.id)"

    $dataExposurePolicy = Get-RequiredProperty `
        -Object $route `
        -Name "data_exposure_policy" `
        -Message "public route data_exposure_policy for $($route.id)"
    Assert-Equals `
        -Actual $dataExposurePolicy.exposure_class `
        -Expected "public_derived" `
        -Message "public route data exposure class for $($route.id)"
    Assert-Equals `
        -Actual $dataExposurePolicy.client_confidentiality_claim `
        -Expected "none" `
        -Message "public route confidentiality claim for $($route.id)"
    Assert-Equals `
        -Actual $dataExposurePolicy.raw_record_access `
        -Expected "forbidden" `
        -Message "public route raw record access for $($route.id)"
    Assert-Equals `
        -Actual $dataExposurePolicy.bulk_export `
        -Expected "forbidden" `
        -Message "public route bulk export for $($route.id)"

    $allowedDataClasses = @($dataExposurePolicy.allowed_data_classes)
    if ($allowedDataClasses.Count -eq 0) {
        throw "public route allowed_data_classes missing for $($route.id)"
    }
    foreach ($dataClass in $requiredPublicForbiddenDataClasses) {
        Assert-ArrayContains `
            -Values @($dataExposurePolicy.forbidden_data_classes) `
            -Expected $dataClass `
            -Message "public route forbidden data classes for $($route.id)"
        Assert-ArrayNotContains `
            -Values $allowedDataClasses `
            -Forbidden $dataClass `
            -Message "public route allowed data classes for $($route.id)"
    }

    Assert-Contains -Content $proxy -Needle $route.proxy_path_source -Message "proxy policy path source for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle $route.proxy_path_source -Message "generated TS policy path source for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle $route.rate_policy.key_prefix -Message "generated TS rate key prefix for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle "limit: $($route.rate_policy.limit)" -Message "generated TS rate limit for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle "windowSec: $($route.rate_policy.window_seconds)" -Message "generated TS rate window for $($route.id)"
    Assert-Contains -Content $rustTrafficGenerated -Needle $route.rate_policy.key_prefix -Message "generated Rust public backend rate key prefix for $($route.id)"
    Assert-Contains -Content $rustTrafficGenerated -Needle "limit: $($route.rate_policy.limit)" -Message "generated Rust public backend rate limit for $($route.id)"
    Assert-Contains -Content $rustTrafficGenerated -Needle "window_seconds: $($route.rate_policy.window_seconds)" -Message "generated Rust public backend rate window for $($route.id)"
    Assert-Contains -Content $rustTrafficGenerated -Needle "problem_type: `"$($route.rate_policy.problem_type)`"" -Message "generated Rust public backend rate problem type for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle 'class: "public_derived"' -Message "generated TS exposure class for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle 'rawRecordAccess: "forbidden"' -Message "generated TS raw access policy for $($route.id)"
    Assert-Contains -Content $tsGenerated -Needle 'bulkExport: "forbidden"' -Message "generated TS bulk export policy for $($route.id)"
    Assert-Contains `
        -Content $tsGenerated `
        -Needle "allowedDataClasses: $(Format-TsStringArray -Values @($route.data_exposure_policy.allowed_data_classes))" `
        -Message "generated TS exposure allowed data classes for $($route.id)"
    Assert-NotContains -Content $proxy -Needle $route.rate_policy.key_prefix -Message "proxy must not hardcode generated rate key prefix for $($route.id)"
}

$tileRoute = Get-RouteById -Routes $publicRoutes -Id "gongzzang.public_map.listing_marker_tile"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "MAX_LISTING_MARKER_TILE_BYTES:\s+usize\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.response_budget.max_tile_bytes `
    -Field "listing marker tile max bytes"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "MAX_LISTING_MARKER_TILE_FEATURES:\s+i64\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.response_budget.max_features `
    -Field "listing marker tile max features"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "LISTING_MARKER_CACHE_TTL_SECONDS:\s+u64\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.cache_policy.ttl_seconds `
    -Field "listing marker cache ttl seconds"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS:\s+u64\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.single_flight_policy.lock_seconds `
    -Field "listing marker single-flight lock seconds"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS:\s+usize\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.single_flight_policy.wait_attempts `
    -Field "listing marker single-flight wait attempts"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS:\s+u64\s+=\s+([0-9_]+);" `
    -Expected $tileRoute.single_flight_policy.wait_milliseconds `
    -Field "listing marker single-flight wait milliseconds"

$maskRoute = Get-RouteById -Routes $publicRoutes -Id "gongzzang.public_map.listing_marker_mask"
Assert-RegexInt `
    -Content $rustGenerated `
    -Pattern "MAX_LISTING_MARKER_MASK_IDS:\s+usize\s+=\s+([0-9_]+);" `
    -Expected $maskRoute.response_budget.max_mask_ids `
    -Field "listing marker mask max ids"

$servicePolicies = @($registry.service_call_policies)
Assert-Equals -Actual $servicePolicies.Count -Expected 2 -Message "service_call_policies count mismatch"
Assert-Unique -Values ($servicePolicies | ForEach-Object { $_.id }) -Message "service call policy ids must be unique"
$edgeServiceRules = @()
if ($IncludeProductionEdge) {
    $edgeServiceRules = @($edgeProjection.service_to_service_rules)
    Assert-Equals -Actual $edgeServiceRules.Count -Expected $servicePolicies.Count -Message "traffic-auth edge service_to_service_rules count"
}

foreach ($servicePolicy in $servicePolicies) {
    $edgeServiceRule = $null
    if ($IncludeProductionEdge) {
        $edgeServiceRule = Get-RuleBySourcePolicyId `
            -Rules $edgeServiceRules `
            -Id ([string] $servicePolicy.id) `
            -Message "traffic-auth edge service rule"
    }
    $targetAuthPolicy = Get-RequiredProperty `
        -Object $servicePolicy `
        -Name "target_auth_policy" `
        -Message "service call target_auth_policy for $($servicePolicy.id)"
    $targetMethod = [string] $targetAuthPolicy.method
    if (
        $targetMethod -ne "mtls_or_short_lived_service_identity" -and
        $targetMethod -ne "mtls_or_signed_event_envelope"
    ) {
        throw "service call target_auth_policy method must be mTLS-capable for $($servicePolicy.id): $targetMethod"
    }
    $serviceIdentity = [string] $targetAuthPolicy.service_identity
    if ([string]::IsNullOrWhiteSpace($serviceIdentity)) {
        throw "service call target_auth_policy service_identity missing for $($servicePolicy.id)"
    }
    if ($IncludeProductionEdge) {
        Assert-Equals `
            -Actual ([string] $edgeServiceRule.target_auth_method) `
            -Expected $targetMethod `
            -Message "traffic-auth edge service target_auth_method for $($servicePolicy.id)"
        Assert-Equals `
            -Actual ([string] $edgeServiceRule.service_identity) `
            -Expected $serviceIdentity `
            -Message "traffic-auth edge service identity for $($servicePolicy.id)"
        $awsWafServiceIdentityRule = Get-RuleBySourcePolicyId `
            -Rules $awsWafServiceIdentityRules `
            -Id ([string] $servicePolicy.id) `
            -Message "AWS WAFv2 service identity rule"
        Assert-Equals `
            -Actual ([string] $awsWafServiceIdentityRule.target_auth_method) `
            -Expected $targetMethod `
            -Message "AWS WAFv2 service identity target_auth_method for $($servicePolicy.id)"
    }
    if ($IncludeProductionEdge -and $null -ne $servicePolicy.PSObject.Properties["source_service"]) {
        Assert-Equals `
            -Actual ([string] $edgeServiceRule.source_service) `
            -Expected ([string] $servicePolicy.source_service) `
            -Message "traffic-auth edge service source_service for $($servicePolicy.id)"
    }
    if ($IncludeProductionEdge -and $null -ne $servicePolicy.PSObject.Properties["target_service"]) {
        Assert-Equals `
            -Actual ([string] $edgeServiceRule.target_service) `
            -Expected ([string] $servicePolicy.target_service) `
            -Message "traffic-auth edge service target_service for $($servicePolicy.id)"
    }
    $currentAuthPolicyProperty = $servicePolicy.PSObject.Properties["current_auth_policy"]
    if ($IncludeProductionEdge -and $null -ne $currentAuthPolicyProperty -and $null -ne $currentAuthPolicyProperty.Value.PSObject.Properties["env"]) {
        Assert-Equals `
            -Actual ([string] $edgeServiceRule.current_auth_env) `
            -Expected ([string] $currentAuthPolicyProperty.Value.env) `
            -Message "traffic-auth edge service current_auth_env for $($servicePolicy.id)"
    }
}

Assert-Contains -Content $boundary -Needle "PLATFORM_CORE_SERVICE_TOKEN" -Message "boundary service token contract"
Assert-Contains -Content $boundary -Needle "PLATFORM_CORE_WEBHOOK_SECRET" -Message "boundary webhook secret contract"
Assert-Contains -Content $boundary -Needle "direct_platform_core_database" -Message "boundary direct database prohibition"

Write-Host "traffic-auth-policy-registry-ok routes=$($publicRoutes.Count) service_policies=$($servicePolicies.Count)"
