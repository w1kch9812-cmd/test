$pageRoutePolicies = @(Get-RequiredProperty `
        -Object $registry `
        -Name "page_route_policies" `
        -Message "page_route_policies")
if ($pageRoutePolicies.Count -eq 0) {
    throw "page_route_policies must not be empty"
}
Assert-Unique -Values ($pageRoutePolicies | ForEach-Object { $_.id }) -Message "page route policy ids must be unique"
Assert-Contains -Content $tsGenerated -Needle "GENERATED_PAGE_ROUTE_POLICIES" -Message "generated TS page route policies"
$apiProxyRoutePoliciesForPageAlignment = @(Get-RequiredProperty `
        -Object $registry `
        -Name "api_proxy_route_policies" `
        -Message "api_proxy_route_policies")
$listingPageToApiPolicyIds = @{
    "gongzzang.page.listing_create" = "gongzzang.api_proxy.listings_collection_create"
    "gongzzang.page.listing_edit"   = "gongzzang.api_proxy.listing_detail_update"
}

foreach ($routePolicy in $pageRoutePolicies) {
    $kind = [string] $routePolicy.path_kind
    if (!(@("exact", "prefix", "prefix_suffix") -contains $kind)) {
        throw "page route policy path_kind invalid for $($routePolicy.id): $kind"
    }

    $pathProperty = $routePolicy.PSObject.Properties["path"]
    $pathSourceProperty = $routePolicy.PSObject.Properties["path_source"]
    $prefixProperty = $routePolicy.PSObject.Properties["prefix"]
    $prefixSourceProperty = $routePolicy.PSObject.Properties["prefix_source"]
    $suffixProperty = $routePolicy.PSObject.Properties["suffix"]

    if ($kind -eq "prefix_suffix") {
        if ($null -eq $suffixProperty -or [string]::IsNullOrWhiteSpace([string] $suffixProperty.Value)) {
            throw "page route policy suffix missing for $($routePolicy.id)"
        }
        if ($null -eq $prefixProperty -and $null -eq $prefixSourceProperty) {
            throw "page route policy prefix or prefix_source missing for $($routePolicy.id)"
        }
    } else {
        if ($null -eq $pathProperty -and $null -eq $pathSourceProperty) {
            throw "page route policy path or path_source missing for $($routePolicy.id)"
        }
    }

    $requiredRoles = @($routePolicy.required_roles)
    if ($requiredRoles.Count -eq 0) {
        throw "page route policy required_roles missing for $($routePolicy.id)"
    }
    foreach ($role in $requiredRoles) {
        $roleString = [string] $role
        if (!(@("Admin", "Broker", "Operator", "Buyer") -contains $roleString)) {
            throw "page route policy required role invalid for $($routePolicy.id): $roleString"
        }
    }

    Assert-Contains -Content $tsGenerated -Needle "kind: `"$kind`"" -Message "generated TS page route kind for $($routePolicy.id)"
    Assert-Contains `
        -Content $tsGenerated `
        -Needle "requiredRoles: $(Format-TsStringArray -Values $requiredRoles)" `
        -Message "generated TS page route required roles for $($routePolicy.id)"
    if ($null -ne $pathProperty) {
        Assert-Contains -Content $tsGenerated -Needle "path: `"$([string] $pathProperty.Value)`"" -Message "generated TS page route path for $($routePolicy.id)"
    }
    if ($null -ne $pathSourceProperty) {
        Assert-Contains -Content $tsGenerated -Needle "pathSource: `"$([string] $pathSourceProperty.Value)`"" -Message "generated TS page route path source for $($routePolicy.id)"
    }
    if ($null -ne $prefixProperty) {
        Assert-Contains -Content $tsGenerated -Needle "prefix: `"$([string] $prefixProperty.Value)`"" -Message "generated TS page route prefix for $($routePolicy.id)"
    }
    if ($null -ne $prefixSourceProperty) {
        Assert-Contains -Content $tsGenerated -Needle "prefixSource: `"$([string] $prefixSourceProperty.Value)`"" -Message "generated TS page route prefix source for $($routePolicy.id)"
    }
    if ($null -ne $suffixProperty) {
        Assert-Contains -Content $tsGenerated -Needle "suffix: `"$([string] $suffixProperty.Value)`"" -Message "generated TS page route suffix for $($routePolicy.id)"
    }

    if ($listingPageToApiPolicyIds.ContainsKey([string] $routePolicy.id)) {
        $apiPolicyId = [string] $listingPageToApiPolicyIds[[string] $routePolicy.id]
        $matchingApiPolicies = @($apiProxyRoutePoliciesForPageAlignment | Where-Object { [string] $_.id -eq $apiPolicyId })
        if ($matchingApiPolicies.Count -ne 1) {
            throw "listing page route roles cannot find matching API proxy route policy for $($routePolicy.id): $apiPolicyId"
        }
        $apiPolicy = $matchingApiPolicies[0]
        if ([string] $apiPolicy.exposure_class -ne "privileged") {
            throw "listing page route roles must map to privileged API route for $($routePolicy.id): $apiPolicyId"
        }
        $apiRequiredRolesProperty = $apiPolicy.PSObject.Properties["required_roles"]
        if ($null -eq $apiRequiredRolesProperty) {
            throw "listing page route roles required_roles missing on API proxy route for $($routePolicy.id): $apiPolicyId"
        }
        $apiRequiredRoles = @($apiRequiredRolesProperty.Value)
        $pageRoleSet = @($requiredRoles | Sort-Object) -join ","
        $apiRoleSet = @($apiRequiredRoles | Sort-Object) -join ","
        if ($pageRoleSet -ne $apiRoleSet) {
            throw "listing page route roles must match API proxy route roles for $($routePolicy.id): page=[$pageRoleSet] api=[$apiRoleSet]"
        }
    }
}
