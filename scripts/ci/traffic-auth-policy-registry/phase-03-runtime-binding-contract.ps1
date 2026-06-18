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
$backendRoutePolicyPathSet = @{}
foreach ($routePolicy in $backendRoutePolicies) {
    $backendRoutePolicyPathSet[[string] $routePolicy.path] = $true
}
foreach ($rustRoutePath in (Get-AxumRoutePaths -Content $apiRouteSources)) {
    if (!$backendRoutePolicyPathSet.ContainsKey($rustRoutePath)) {
        throw "backend_route_policies missing Rust route path: $rustRoutePath"
    }
}

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
