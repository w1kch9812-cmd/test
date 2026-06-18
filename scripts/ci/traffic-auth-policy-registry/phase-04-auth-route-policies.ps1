$authRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "auth_route_policies" `
        -Message "auth_route_policies")
if ($authRoutePolicies.Count -eq 0) {
    throw "auth_route_policies must not be empty"
}
Assert-Unique -Values ($authRoutePolicies | ForEach-Object { $_.id }) -Message "auth route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_AUTH_RATE_ROUTE_POLICIES" -Message "generated TS auth rate policies"
$authPathSourcesFromRoutes = @(Get-AuthPathSourcesFromRoutesTs -Content $routesTs)
if ($authPathSourcesFromRoutes.Count -eq 0) {
    throw "apps/web/lib/routes.ts auth route sources missing"
}
foreach ($pathSourceFromRoutes in $authPathSourcesFromRoutes) {
    $matchingAuthRoutePolicy = @($authRoutePolicies | Where-Object { [string] $_.path_source -eq $pathSourceFromRoutes })
    if ($matchingAuthRoutePolicy.Count -eq 0) {
        throw "auth_route_policies missing routes.ts auth path source: $pathSourceFromRoutes"
    }
    Assert-Contains -Content $proxy -Needle $pathSourceFromRoutes -Message "proxy auth path source for $pathSourceFromRoutes"
}
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
    if (!($authPathSourcesFromRoutes -contains $pathSource)) {
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
