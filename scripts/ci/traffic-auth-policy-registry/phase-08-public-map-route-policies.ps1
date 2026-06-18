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
