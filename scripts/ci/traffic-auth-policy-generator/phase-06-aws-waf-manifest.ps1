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
