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
