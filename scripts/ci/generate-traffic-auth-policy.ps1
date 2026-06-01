[CmdletBinding()]
param(
    [string] $Root = ""
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

function Read-JsonFile {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return (Get-Content -LiteralPath $path -Raw -Encoding UTF8) | ConvertFrom-Json
}

function Format-NumberLiteral {
    param([int64] $Value)
    $digits = [string] $Value
    if ($digits.Length -le 3) {
        return $digits
    }
    $groups = New-Object System.Collections.Generic.List[string]
    while ($digits.Length -gt 3) {
        $groups.Insert(0, $digits.Substring($digits.Length - 3))
        $digits = $digits.Substring(0, $digits.Length - 3)
    }
    $groups.Insert(0, $digits)
    return ($groups -join "_")
}

function Convert-PathSourceToTs {
    param([string] $Source)
    return $Source.Replace("\", "\\").Replace('"', '\"')
}

function Convert-StringToTs {
    param([string] $Value)
    return $Value.Replace("\", "\\").Replace('"', '\"')
}

function Convert-StringArrayToTs {
    param([object[]] $Values)
    $quotedValues = @($Values | ForEach-Object { "`"$(Convert-StringToTs -Value ([string] $_))`"" })
    return "[$($quotedValues -join ", ")]"
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

function Get-OptionalPropertyValue {
    param([object] $Object, [string] $Name)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return @()
    }
    return @($property.Value)
}

function Get-OptionalStringPropertyValue {
    param([object] $Object, [string] $Name)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return [string] $property.Value
}

function Get-RouteRateProfile {
    param([object[]] $Profiles, [string] $Id)
    foreach ($profile in $Profiles) {
        if ([string] $profile.id -eq $Id) {
            return $profile
        }
    }
    throw "Missing API proxy rate profile id=$Id"
}

$registry = Read-JsonFile -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
if ($registry.schema_version -ne $ExpectedSchemaVersion) {
    throw "Unsupported schema_version '$($registry.schema_version)'"
}

$publicRoutes = @($registry.public_route_policies)
$authRoutes = @($registry.auth_route_policies)
$pageRoutes = @($registry.page_route_policies)
$routeRateProfiles = @($registry.route_rate_profiles)
$apiProxyRoutes = @($registry.api_proxy_route_policies)
$backendRoutes = @($registry.backend_route_policies)

$tsLines = New-Object System.Collections.Generic.List[string]
$tsLines.Add("// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.")
$tsLines.Add("// Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.")
$tsLines.Add("")
$tsLines.Add("export type GeneratedAuthRateRoutePolicy = {")
$tsLines.Add("  readonly pathSource: string;")
$tsLines.Add('  readonly methods: readonly ("GET" | "POST")[];')
$tsLines.Add("  readonly rate: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add('    readonly keyStrategy: "client_ip" | "session_or_anon";')
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("    readonly problemType: string;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_AUTH_RATE_ROUTE_POLICIES: readonly GeneratedAuthRateRoutePolicy[] = [")
foreach ($route in $authRoutes) {
    $pathSource = Convert-PathSourceToTs -Source ([string] $route.path_source)
    $methods = Convert-StringArrayToTs -Values @($route.methods)
    $keyPrefix = Convert-StringToTs -Value ([string] $route.rate_policy.key_prefix)
    $keyStrategy = Convert-StringToTs -Value ([string] $route.rate_policy.key_strategy)
    $limit = [int64] $route.rate_policy.limit
    $windowSec = [int64] $route.rate_policy.window_seconds
    $problemType = Convert-StringToTs -Value ([string] $route.rate_policy.problem_type)
    $tsLines.Add("  {")
    $tsLines.Add("    pathSource: `"$pathSource`",")
    $tsLines.Add("    methods: $methods,")
    $tsLines.Add("    rate: {")
    $tsLines.Add("      keyPrefix: `"$keyPrefix`",")
    $tsLines.Add("      keyStrategy: `"$keyStrategy`",")
    $tsLines.Add("      limit: $limit,")
    $tsLines.Add("      windowSec: $windowSec,")
    $tsLines.Add("      problemType: `"$problemType`",")
    $tsLines.Add("    },")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedPageRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix" | "prefix_suffix";')
$tsLines.Add("  readonly path?: string;")
$tsLines.Add("  readonly pathSource?: string;")
$tsLines.Add("  readonly prefix?: string;")
$tsLines.Add("  readonly prefixSource?: string;")
$tsLines.Add("  readonly suffix?: string;")
$tsLines.Add("  readonly requiredRoles: readonly string[];")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_PAGE_ROUTE_POLICIES: readonly GeneratedPageRoutePolicy[] = [")
foreach ($route in $pageRoutes) {
    $kind = Convert-StringToTs -Value ([string] $route.path_kind)
    $requiredRoles = Convert-StringArrayToTs -Values @($route.required_roles)
    $path = Get-OptionalStringPropertyValue -Object $route -Name "path"
    $pathSource = Get-OptionalStringPropertyValue -Object $route -Name "path_source"
    $prefix = Get-OptionalStringPropertyValue -Object $route -Name "prefix"
    $prefixSource = Get-OptionalStringPropertyValue -Object $route -Name "prefix_source"
    $suffix = Get-OptionalStringPropertyValue -Object $route -Name "suffix"

    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    if ($null -ne $path) {
        $tsLines.Add("    path: `"$(Convert-StringToTs -Value $path)`",")
    }
    if ($null -ne $pathSource) {
        $tsLines.Add("    pathSource: `"$(Convert-StringToTs -Value $pathSource)`",")
    }
    if ($null -ne $prefix) {
        $tsLines.Add("    prefix: `"$(Convert-StringToTs -Value $prefix)`",")
    }
    if ($null -ne $prefixSource) {
        $tsLines.Add("    prefixSource: `"$(Convert-StringToTs -Value $prefixSource)`",")
    }
    if ($null -ne $suffix) {
        $tsLines.Add("    suffix: `"$(Convert-StringToTs -Value $suffix)`",")
    }
    $tsLines.Add("    requiredRoles: $requiredRoles,")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedPublicMapRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix";')
$tsLines.Add("  readonly pathSource: string;")
$tsLines.Add("  readonly exposure: {")
$tsLines.Add('    readonly class: "public_derived";')
$tsLines.Add("    readonly allowedDataClasses: readonly string[];")
$tsLines.Add('    readonly rawRecordAccess: "forbidden";')
$tsLines.Add('    readonly bulkExport: "forbidden";')
$tsLines.Add("  };")
$tsLines.Add("  readonly rate: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = [")
foreach ($route in $publicRoutes) {
    $kind = [string] $route.proxy_path_kind
    $pathSource = Convert-PathSourceToTs -Source ([string] $route.proxy_path_source)
    $keyPrefix = Convert-StringToTs -Value ([string] $route.rate_policy.key_prefix)
    $limit = [int64] $route.rate_policy.limit
    $windowSec = [int64] $route.rate_policy.window_seconds
    $allowedDataClasses = Convert-StringArrayToTs -Values @($route.data_exposure_policy.allowed_data_classes)
    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    $tsLines.Add("    pathSource: `"$pathSource`",")
    $tsLines.Add("    exposure: {")
    $tsLines.Add("      class: `"public_derived`",")
    $tsLines.Add("      allowedDataClasses: $allowedDataClasses,")
    $tsLines.Add("      rawRecordAccess: `"forbidden`",")
    $tsLines.Add("      bulkExport: `"forbidden`",")
    $tsLines.Add("    },")
    $tsLines.Add("    rate: { keyPrefix: `"$keyPrefix`", limit: $limit, windowSec: $windowSec },")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedApiProxyRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix" | "template";')
$tsLines.Add("  readonly targetPath: string;")
$tsLines.Add('  readonly methods: readonly ("GET" | "POST" | "PUT" | "PATCH" | "DELETE")[];')
$tsLines.Add('  readonly exposureClass: "public_derived" | "authenticated_user" | "privileged";')
$tsLines.Add("  readonly requiredRoles: readonly string[];")
$tsLines.Add("  readonly rate?: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add('    readonly keyStrategy: "session_sub";')
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("    readonly problemType: string;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_API_PROXY_ROUTE_POLICIES: readonly GeneratedApiProxyRoutePolicy[] = [")
foreach ($route in $apiProxyRoutes) {
    $kind = Convert-StringToTs -Value ([string] $route.target_path_kind)
    $targetPath = Convert-StringToTs -Value ([string] $route.target_path)
    $methods = Convert-StringArrayToTs -Values @($route.methods)
    $exposureClass = Convert-StringToTs -Value ([string] $route.exposure_class)
    $requiredRoles = Convert-StringArrayToTs -Values @(Get-OptionalPropertyValue -Object $route -Name "required_roles")
    $rateProfileId = Get-OptionalStringPropertyValue -Object $route -Name "rate_profile"
    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    $tsLines.Add("    targetPath: `"$targetPath`",")
    $tsLines.Add("    methods: $methods,")
    $tsLines.Add("    exposureClass: `"$exposureClass`",")
    $tsLines.Add("    requiredRoles: $requiredRoles,")
    if ($null -ne $rateProfileId) {
        $rateProfile = Get-RouteRateProfile -Profiles $routeRateProfiles -Id $rateProfileId
        $keyPrefix = Convert-StringToTs -Value ([string] $rateProfile.key_prefix)
        $keyStrategy = Convert-StringToTs -Value ([string] $rateProfile.key_strategy)
        $limit = [int64] $rateProfile.limit
        $windowSec = [int64] $rateProfile.window_seconds
        $problemType = Convert-StringToTs -Value ([string] $rateProfile.problem_type)
        $tsLines.Add("    rate: {")
        $tsLines.Add("      keyPrefix: `"$keyPrefix`",")
        $tsLines.Add("      keyStrategy: `"$keyStrategy`",")
        $tsLines.Add("      limit: $limit,")
        $tsLines.Add("      windowSec: $windowSec,")
        $tsLines.Add("      problemType: `"$problemType`",")
        $tsLines.Add("    },")
    }
    $tsLines.Add("  },")
}
$tsLines.Add("];")

$tsPath = Resolve-RepoPath -RelativePath "apps/web/lib/policies/traffic-auth-policy.generated.ts"
New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($tsPath)) | Out-Null
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($tsPath, (($tsLines -join "`n") + "`n"), $utf8NoBom)

function Convert-BackendPolicyPathToRustPattern {
    param([string] $Path)
    return ([regex]::Replace($Path, "\{([^}]+)\}", ':$1'))
}

function Convert-KeyStrategyToRust {
    param([string] $Strategy)
    switch ($Strategy) {
        "client_ip" { return "BackendRateKeyStrategy::ClientIp" }
        "session_sub" { return "BackendRateKeyStrategy::SessionSub" }
        default { throw "Unsupported route rate key_strategy '$Strategy'" }
    }
}

function Convert-RoleToRust {
    param([string] $Role)
    switch ($Role) {
        "Admin" { return "UserRole::Admin" }
        "Broker" { return "UserRole::Broker" }
        "Buyer" { return "UserRole::Buyer" }
        "Developer" { return "UserRole::Developer" }
        "Enterprise" { return "UserRole::Enterprise" }
        "Operator" { return "UserRole::Operator" }
        "Seller" { return "UserRole::Seller" }
        default { throw "Unsupported backend required role '$Role'" }
    }
}

function Convert-MethodsToArray {
    param([object[]] $Methods)
    $values = @($Methods | ForEach-Object { [string] $_ })
    return , $values
}

function Convert-RequiredRolesToArray {
    param([object] $Route)
    $values = @(Get-OptionalPropertyValue -Object $Route -Name "required_roles" | ForEach-Object { [string] $_ })
    return , $values
}

function New-RateProjection {
    param([object] $Rate, [string] $KeyStrategy = "")
    $projectedKeyStrategy = if ([string]::IsNullOrWhiteSpace($KeyStrategy)) {
        $property = $Rate.PSObject.Properties["key_strategy"]
        if ($null -eq $property) { "client_ip" } else { [string] $property.Value }
    } else {
        $KeyStrategy
    }
    return [ordered]@{
        key_strategy   = $projectedKeyStrategy
        key_prefix     = [string] $Rate.key_prefix
        limit          = [int64] $Rate.limit
        window_seconds = [int64] $Rate.window_seconds
        problem_type   = [string] $Rate.problem_type
    }
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

function New-AwsWafRateRule {
    param(
        [string] $SourcePolicyId,
        [int] $Priority,
        [object] $Rate,
        [string] $Path = "",
        [string] $PathSource = "",
        [string] $PathMatch,
        [object[]] $Methods
    )
    $match = [ordered]@{
        path_match = $PathMatch
        methods    = Convert-MethodsToArray -Methods $Methods
    }
    if (![string]::IsNullOrWhiteSpace($Path)) {
        $match.path = $Path
    }
    if (![string]::IsNullOrWhiteSpace($PathSource)) {
        $match.path_source = $PathSource
    }
    return [ordered]@{
        source_policy_id  = $SourcePolicyId
        priority          = $Priority
        aggregate_key_type = "IP"
        limit_per_5m      = Convert-RateToFiveMinuteLimit -Rate $Rate
        match             = $match
    }
}

function Resolve-AuthPathSource {
    param([string] $PathSource)
    switch ($PathSource) {
        "API.auth.login" { return "/api/auth/login" }
        "API.auth.callback" { return "/api/auth/callback" }
        "API.auth.refresh" { return "/api/auth/refresh" }
        "API.auth.logout" { return "/api/auth/logout" }
        default { throw "Unsupported auth path source '$PathSource'" }
    }
}

function New-IdentityAwareApplicationRule {
    param([string] $SourcePolicyId)
    return [ordered]@{
        source_policy_id = $SourcePolicyId
        reason           = "key_strategy_not_representable_in_wafv2"
    }
}

$rustTrafficLines = New-Object System.Collections.Generic.List[string]
$rustTrafficLines.Add("//! Generated traffic/auth serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.")
$rustTrafficLines.Add("//! Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.")
$rustTrafficLines.Add("")
$rustTrafficLines.Add("use crate::backend_authorization::BackendRolePolicy;")
$rustTrafficLines.Add("use crate::backend_rate_limit::{BackendRateKeyStrategy, BackendRatePolicy};")
$rustTrafficLines.Add("use user_domain::entity::UserRole;")
$rustTrafficLines.Add("")
$rustTrafficLines.Add("pub const BACKEND_RATE_POLICIES: &[BackendRatePolicy] = &[")
foreach ($route in $publicRoutes) {
    $pathPattern = Convert-StringToTs -Value (Convert-BackendPolicyPathToRustPattern -Path ([string] $route.backend_route))
    $keyPrefix = Convert-StringToTs -Value ([string] $route.rate_policy.key_prefix)
    $limit = [int64] $route.rate_policy.limit
    $windowSec = [int64] $route.rate_policy.window_seconds
    $problemType = Convert-StringToTs -Value ([string] $route.rate_policy.problem_type)
    foreach ($methodValue in @($route.methods)) {
        $method = Convert-StringToTs -Value ([string] $methodValue)
        $rustTrafficLines.Add("    BackendRatePolicy {")
        $rustTrafficLines.Add("        method: `"$method`",")
        $rustTrafficLines.Add("        path_pattern: `"$pathPattern`",")
        $rustTrafficLines.Add("        key_prefix: `"$keyPrefix`",")
        $rustTrafficLines.Add("        key_strategy: BackendRateKeyStrategy::ClientIp,")
        $rustTrafficLines.Add("        limit: $limit,")
        $rustTrafficLines.Add("        window_seconds: $windowSec,")
        $rustTrafficLines.Add("        problem_type: `"$problemType`",")
        $rustTrafficLines.Add("    },")
    }
}
foreach ($route in $backendRoutes) {
    $rateProfileId = Get-OptionalStringPropertyValue -Object $route -Name "rate_profile"
    if ($null -eq $rateProfileId) {
        continue
    }
    $rateProfile = Get-RouteRateProfile -Profiles $routeRateProfiles -Id $rateProfileId
    $pathPattern = Convert-StringToTs -Value ([string] $route.path)
    $keyPrefix = Convert-StringToTs -Value ([string] $rateProfile.key_prefix)
    $keyStrategy = Convert-KeyStrategyToRust -Strategy ([string] $rateProfile.key_strategy)
    $limit = [int64] $rateProfile.limit
    $windowSec = [int64] $rateProfile.window_seconds
    $problemType = Convert-StringToTs -Value ([string] $rateProfile.problem_type)
    foreach ($methodValue in @($route.methods)) {
        $method = Convert-StringToTs -Value ([string] $methodValue)
        $rustTrafficLines.Add("    BackendRatePolicy {")
        $rustTrafficLines.Add("        method: `"$method`",")
        $rustTrafficLines.Add("        path_pattern: `"$pathPattern`",")
        $rustTrafficLines.Add("        key_prefix: `"$keyPrefix`",")
        $rustTrafficLines.Add("        key_strategy: $keyStrategy,")
        $rustTrafficLines.Add("        limit: $limit,")
        $rustTrafficLines.Add("        window_seconds: $windowSec,")
        $rustTrafficLines.Add("        problem_type: `"$problemType`",")
        $rustTrafficLines.Add("    },")
    }
}
$rustTrafficLines.Add("];")
$rustTrafficLines.Add("")
$rustTrafficLines.Add("pub const BACKEND_ROLE_POLICIES: &[BackendRolePolicy] = &[")
foreach ($route in $backendRoutes) {
    $requiredRoles = @(Get-OptionalPropertyValue -Object $route -Name "required_roles")
    if ($requiredRoles.Count -eq 0) {
        continue
    }
    $pathPattern = Convert-StringToTs -Value ([string] $route.path)
    $roleValues = @($requiredRoles | ForEach-Object { Convert-RoleToRust -Role ([string] $_) })
    $roleLiteral = "&[$($roleValues -join ", ")]"
    foreach ($methodValue in @($route.methods)) {
        $method = Convert-StringToTs -Value ([string] $methodValue)
        $rustTrafficLines.Add("    BackendRolePolicy {")
        $rustTrafficLines.Add("        method: `"$method`",")
        $rustTrafficLines.Add("        path_pattern: `"$pathPattern`",")
        $rustTrafficLines.Add("        required_roles: $roleLiteral,")
        $rustTrafficLines.Add("    },")
    }
}
$rustTrafficLines.Add("];")

$rustTrafficPath = Resolve-RepoPath -RelativePath "services/api/src/traffic_auth_policy.rs"
[System.IO.File]::WriteAllText($rustTrafficPath, (($rustTrafficLines -join "`n") + "`n"), $utf8NoBom)

$tileRoute = Get-RouteById -Routes $publicRoutes -Id "gongzzang.public_map.listing_marker_tile"
$maskRoute = Get-RouteById -Routes $publicRoutes -Id "gongzzang.public_map.listing_marker_mask"

$rustLines = New-Object System.Collections.Generic.List[string]
$rustLines.Add("//! Generated listing marker serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.")
$rustLines.Add("//! Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.")
$rustLines.Add("")
$rustLines.Add("pub const MAX_LISTING_MARKER_TILE_BYTES: usize = $(Format-NumberLiteral -Value ([int64] $tileRoute.response_budget.max_tile_bytes));")
$rustLines.Add("pub const MAX_LISTING_MARKER_TILE_FEATURES: i64 = $(Format-NumberLiteral -Value ([int64] $tileRoute.response_budget.max_features));")
$rustLines.Add("pub const MAX_LISTING_MARKER_MASK_IDS: usize = $(Format-NumberLiteral -Value ([int64] $maskRoute.response_budget.max_mask_ids));")
$rustLines.Add("pub const LISTING_MARKER_CACHE_TTL_SECONDS: u64 = $(Format-NumberLiteral -Value ([int64] $tileRoute.cache_policy.ttl_seconds));")
$rustLines.Add("pub const LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = $(Format-NumberLiteral -Value ([int64] $tileRoute.single_flight_policy.lock_seconds));")
$rustLines.Add("pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = $(Format-NumberLiteral -Value ([int64] $tileRoute.single_flight_policy.wait_attempts));")
$rustLines.Add("pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = $(Format-NumberLiteral -Value ([int64] $tileRoute.single_flight_policy.wait_milliseconds));")

$rustPath = Resolve-RepoPath -RelativePath "services/api/src/listing_marker_policy.rs"
[System.IO.File]::WriteAllText($rustPath, (($rustLines -join "`n") + "`n"), $utf8NoBom)

$publicEdgeRules = @($publicRoutes | ForEach-Object {
        [ordered]@{
            source_policy_id        = [string] $_.id
            proxy_path              = [string] $_.proxy_path
            backend_route           = [string] $_.backend_route
            methods                 = Convert-MethodsToArray -Methods @($_.methods)
            exposure_class          = "public_derived"
            rate                    = New-RateProjection -Rate $_.rate_policy -KeyStrategy "client_ip"
            forbidden_request_shapes = @(Get-OptionalPropertyValue -Object $_ -Name "forbidden_request_shapes" | ForEach-Object { [string] $_ })
        }
    })

$authEdgeRules = @($authRoutes | ForEach-Object {
        [ordered]@{
            source_policy_id = [string] $_.id
            path_source      = [string] $_.path_source
            methods          = Convert-MethodsToArray -Methods @($_.methods)
            rate             = New-RateProjection -Rate $_.rate_policy
        }
    })

$apiProxyEdgeRules = @($apiProxyRoutes | ForEach-Object {
        $rateProfileId = Get-OptionalStringPropertyValue -Object $_ -Name "rate_profile"
        $rule = [ordered]@{
            source_policy_id = [string] $_.id
            edge_path        = "/api/proxy/$([string] $_.target_path)"
            target_path      = [string] $_.target_path
            target_path_kind = [string] $_.target_path_kind
            methods          = Convert-MethodsToArray -Methods @($_.methods)
            exposure_class   = [string] $_.exposure_class
            required_roles   = Convert-RequiredRolesToArray -Route $_
        }
        if ($null -ne $rateProfileId) {
            $rule.rate = New-RateProjection -Rate (Get-RouteRateProfile -Profiles $routeRateProfiles -Id $rateProfileId)
        }
        [pscustomobject] $rule
    })

$serviceEdgeRules = @($registry.service_call_policies | ForEach-Object {
        $targetAuthPolicy = $_.target_auth_policy
        $currentAuthEnv = $null
        $currentAuthProperty = $_.PSObject.Properties["current_auth_policy"]
        if ($null -ne $currentAuthProperty -and $null -ne $currentAuthProperty.Value.PSObject.Properties["env"]) {
            $currentAuthEnv = [string] $currentAuthProperty.Value.env
        }
        [ordered]@{
            source_policy_id   = [string] $_.id
            source_service     = [string] $_.source_service
            target_service     = [string] $_.target_service
            target_auth_method = [string] $targetAuthPolicy.method
            service_identity   = [string] $targetAuthPolicy.service_identity
            current_auth_env   = $currentAuthEnv
        }
    })

$edgeProjection = [ordered]@{
    schema_version         = "gongzzang.traffic_auth_edge_policy_projection.v1"
    source_registry        = "docs/architecture/traffic-auth-policy-registry.v1.json"
    projection_kind        = "provider_neutral_edge_ingress"
    generated_targets      = @("cloudfront", "aws_wafv2", "alb", "service_mesh")
    public_route_rules     = $publicEdgeRules
    auth_route_rules       = $authEdgeRules
    api_proxy_route_rules  = $apiProxyEdgeRules
    service_to_service_rules = $serviceEdgeRules
}
$edgePath = Resolve-RepoPath -RelativePath "infrastructure/security/traffic-auth-edge-policy.generated.json"
New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($edgePath)) | Out-Null
$edgeJson = $edgeProjection | ConvertTo-Json -Depth 16
[System.IO.File]::WriteAllText($edgePath, ($edgeJson + "`n"), $utf8NoBom)

$awsWafRateRules = New-Object System.Collections.Generic.List[object]
$awsWafPriority = 1000
foreach ($route in $publicRoutes) {
    $awsWafRateRules.Add((New-AwsWafRateRule `
                -SourcePolicyId ([string] $route.id) `
                -Priority $awsWafPriority `
                -Rate $route.rate_policy `
                -Path ([string] $route.proxy_path) `
                -PathMatch (Convert-PathKindToAwsWafPathMatch -Kind ([string] $route.proxy_path_kind)) `
                -Methods @($route.methods)))
    $awsWafPriority += 10
}
foreach ($route in $authRoutes) {
    $keyStrategy = [string] $route.rate_policy.key_strategy
    if ($keyStrategy -ne "client_ip") {
        continue
    }
    $awsWafRateRules.Add((New-AwsWafRateRule `
                -SourcePolicyId ([string] $route.id) `
                -Priority $awsWafPriority `
                -Rate $route.rate_policy `
                -Path (Resolve-AuthPathSource -PathSource ([string] $route.path_source)) `
                -PathSource ([string] $route.path_source) `
                -PathMatch "EXACT" `
                -Methods @($route.methods)))
    $awsWafPriority += 10
}

$blockedQueryShapeRules = New-Object System.Collections.Generic.List[object]
$blockedShapePriority = 2000
foreach ($route in $publicRoutes) {
    $forbiddenShapes = @(Get-OptionalPropertyValue -Object $route -Name "forbidden_request_shapes")
    if ($forbiddenShapes.Count -eq 0) {
        continue
    }
    $blockedQueryShapeRules.Add([ordered]@{
            source_policy_id = [string] $route.id
            priority         = $blockedShapePriority
            action           = "BLOCK"
            match            = [ordered]@{
                path             = [string] $route.proxy_path
                path_match       = Convert-PathKindToAwsWafPathMatch -Kind ([string] $route.proxy_path_kind)
                query_parameters = @($forbiddenShapes | ForEach-Object { [string] $_ })
            }
        })
    $blockedShapePriority += 10
}

$identityAwareApplicationRules = New-Object System.Collections.Generic.List[object]
foreach ($route in $authRoutes) {
    if ([string] $route.rate_policy.key_strategy -ne "client_ip") {
        $identityAwareApplicationRules.Add((New-IdentityAwareApplicationRule -SourcePolicyId ([string] $route.id)))
    }
}
foreach ($rule in $apiProxyEdgeRules) {
    $rateProperty = $rule.PSObject.Properties["rate"]
    if ($null -ne $rateProperty -and [string] $rateProperty.Value.key_strategy -ne "client_ip") {
        $identityAwareApplicationRules.Add((New-IdentityAwareApplicationRule -SourcePolicyId ([string] $rule.source_policy_id)))
    }
}

$serviceIdentityRules = @($serviceEdgeRules | ForEach-Object {
        [ordered]@{
            source_policy_id   = [string] $_.source_policy_id
            target_auth_method = [string] $_.target_auth_method
        }
    })

$awsWafManifest = [ordered]@{
    schema_version                   = "gongzzang.aws_wafv2_edge_policy_manifest.v1"
    source_projection                = "infrastructure/security/traffic-auth-edge-policy.generated.json"
    source_registry                  = "docs/architecture/traffic-auth-policy-registry.v1.json"
    managed_by                       = "pulumi"
    scope_options                    = @("CLOUDFRONT", "REGIONAL")
    rate_based_rules                 = @($awsWafRateRules.ToArray())
    blocked_query_shape_rules        = @($blockedQueryShapeRules.ToArray())
    identity_aware_application_rules = @($identityAwareApplicationRules.ToArray())
    service_identity_rules           = @($serviceIdentityRules)
}
$awsWafPath = Resolve-RepoPath -RelativePath "infrastructure/security/aws-wafv2-edge-policy.generated.json"
$awsWafJson = $awsWafManifest | ConvertTo-Json -Depth 16
[System.IO.File]::WriteAllText($awsWafPath, ($awsWafJson + "`n"), $utf8NoBom)

Write-Host "traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts rust=services/api/src/listing_marker_policy.rs,services/api/src/traffic_auth_policy.rs edge=infrastructure/security/traffic-auth-edge-policy.generated.json aws_wafv2=infrastructure/security/aws-wafv2-edge-policy.generated.json"
