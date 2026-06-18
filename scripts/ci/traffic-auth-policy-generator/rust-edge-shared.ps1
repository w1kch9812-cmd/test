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
