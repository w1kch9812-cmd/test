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
