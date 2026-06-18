$apiProxyRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "api_proxy_route_policies" `
        -Message "api_proxy_route_policies")
if ($apiProxyRoutePolicies.Count -eq 0) {
    throw "api_proxy_route_policies must not be empty"
}
Assert-Unique -Values ($apiProxyRoutePolicies | ForEach-Object { $_.id }) -Message "API proxy route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_API_PROXY_ROUTE_POLICIES" -Message "generated TS API proxy route policies"
$apiProxyPolicyPathSet = @{}
$apiProxyPolicyMethodPathSet = @{}
foreach ($routePolicy in $apiProxyRoutePolicies) {
    $policyPattern = Convert-ApiProxyTargetToCoveragePattern -Path ([string] $routePolicy.target_path)
    if ($null -eq $policyPattern) {
        throw "API proxy route policy target_path cannot be normalized for $($routePolicy.id): $($routePolicy.target_path)"
    }
    $apiProxyPolicyPathSet[$policyPattern] = $true
    foreach ($method in @($routePolicy.methods)) {
        $apiProxyPolicyMethodPathSet["$([string] $method) $policyPattern"] = $true
    }
}

$webSourceFiles = @(Get-WebSourceFiles)
foreach ($usage in (Get-DirectApiTransportUsages -Files $webSourceFiles)) {
    throw "direct API transport usage is forbidden outside generated API proxy client: $($usage.Method) in $($usage.Source)"
}
foreach ($usage in (Get-ApiClientRouteUsages -Files $webSourceFiles)) {
    $usageKey = "$($usage.Method) $($usage.Pattern)"
    if (!$apiProxyPolicyMethodPathSet.ContainsKey($usageKey)) {
        throw "API proxy client route usage has no matching api_proxy_route_policies: $usageKey in $($usage.Source)"
    }
}
foreach ($usage in (Get-ApiProxyLiteralUsages -Files $webSourceFiles)) {
    if (!(Test-ApiProxyPolicyPathCoverage -PolicyPathSet $apiProxyPolicyPathSet -Pattern ([string] $usage.Pattern))) {
        throw "API proxy literal has no matching api_proxy_route_policies target_path: $($usage.Pattern) in $($usage.Source)"
    }
}
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
