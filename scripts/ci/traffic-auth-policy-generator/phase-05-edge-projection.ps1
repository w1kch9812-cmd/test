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
