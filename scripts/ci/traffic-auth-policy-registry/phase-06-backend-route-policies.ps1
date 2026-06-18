foreach ($routePolicy in $backendRoutePolicies) {
    $path = [string] $routePolicy.path
    if ([string]::IsNullOrWhiteSpace($path) -or !$path.StartsWith("/") -or $path.Contains("..")) {
        throw "backend route policy path must be absolute and normalized for $($routePolicy.id): $path"
    }
    Assert-Contains -Content $apiRouteSources -Needle $path -Message "backend route policy Rust path for $($routePolicy.id)"

    $routerGroup = [string] $routePolicy.router_group
    if (!(@("public_health", "public_marker", "protected", "internal") -contains $routerGroup)) {
        throw "backend route policy router_group invalid for $($routePolicy.id): $routerGroup"
    }

    $exposureClass = [string] $routePolicy.exposure_class
    if (!(@("public_health", "public_derived", "authenticated_user", "privileged", "service_to_service") -contains $exposureClass)) {
        throw "backend route policy exposure_class invalid for $($routePolicy.id): $exposureClass"
    }

    $authPolicy = [string] $routePolicy.auth_policy
    if ($routerGroup -eq "protected") {
        Assert-Equals -Actual $authPolicy -Expected "bearer_jwt" -Message "backend protected auth policy for $($routePolicy.id)"
        Assert-Contains -Content $apiMain -Needle "auth_layer" -Message "backend protected route auth_layer"
    } elseif ($routerGroup -eq "internal") {
        Assert-Equals -Actual $authPolicy -Expected "internal_shared_secret" -Message "backend internal auth policy for $($routePolicy.id)"
        Assert-Contains -Content $apiMain -Needle "build_internal_auth_secret" -Message "backend internal shared secret builder"
        Assert-Contains -Content $apiMain -Needle "internal_auth_secret" -Message "backend internal shared secret state"
    } else {
        Assert-Equals -Actual $authPolicy -Expected "anonymous_public" -Message "backend public auth policy for $($routePolicy.id)"
        if ($exposureClass -ne "public_health" -and $exposureClass -ne "public_derived") {
            throw "backend public route exposure_class invalid for $($routePolicy.id): $exposureClass"
        }
    }

    $methods = @($routePolicy.methods)
    if ($methods.Count -eq 0) {
        throw "backend route policy methods missing for $($routePolicy.id)"
    }
    foreach ($method in $methods) {
        $methodString = [string] $method
        if (!(@("GET", "POST", "PUT", "PATCH", "DELETE") -contains $methodString)) {
            throw "backend route policy method invalid for $($routePolicy.id): $methodString"
        }
    }

    $requiredRoles = @()
    $requiredRolesProperty = $routePolicy.PSObject.Properties["required_roles"]
    if ($null -ne $requiredRolesProperty) {
        $requiredRoles = @($requiredRolesProperty.Value)
    }
    if ($exposureClass -eq "privileged") {
        if ($requiredRoles.Count -eq 0) {
            throw "backend route policy required_roles missing for privileged route $($routePolicy.id)"
        }
        $requiredRoleLiteral = Format-RustUserRoleArray -Values $requiredRoles
        Assert-Contains -Content $rustTrafficGenerated -Needle $requiredRoleLiteral -Message "generated Rust backend required roles for $($routePolicy.id)"
        foreach ($method in $methods) {
            $rolePolicyPattern = "BackendRolePolicy\s*\{\s*method:\s*`"$([regex]::Escape([string] $method))`",\s*path_pattern:\s*`"$([regex]::Escape($path))`",\s*required_roles:\s*$([regex]::Escape($requiredRoleLiteral)),\s*\}"
            Assert-RegexContains -Content $rustTrafficGenerated -Pattern $rolePolicyPattern -Message "generated Rust backend role policy for $($routePolicy.id)"
        }
    } elseif ($requiredRoles.Count -ne 0) {
        throw "backend route policy required_roles only valid for privileged routes: $($routePolicy.id)"
    }

    $backendRateProfileProperty = $routePolicy.PSObject.Properties["rate_profile"]
    if ($routerGroup -eq "protected") {
        if ($null -eq $backendRateProfileProperty -or [string]::IsNullOrWhiteSpace([string] $backendRateProfileProperty.Value)) {
            throw "backend route policy rate_profile missing for protected route $($routePolicy.id)"
        }
        $backendRateProfileId = [string] $backendRateProfileProperty.Value
        if (!$routeRateProfileById.ContainsKey($backendRateProfileId)) {
            throw "backend route policy rate_profile unknown for $($routePolicy.id): $backendRateProfileId"
        }
        $backendRateProfile = $routeRateProfileById[$backendRateProfileId]
        foreach ($method in $methods) {
            Assert-Contains -Content $rustTrafficGenerated -Needle "method: `"$method`"" -Message "generated Rust backend rate method for $($routePolicy.id)"
        }
        Assert-Contains -Content $rustTrafficGenerated -Needle "path_pattern: `"$path`"" -Message "generated Rust backend rate path for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "key_prefix: `"$($backendRateProfile.key_prefix)`"" -Message "generated Rust backend rate key prefix for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "limit: $($backendRateProfile.limit)" -Message "generated Rust backend rate limit for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "window_seconds: $($backendRateProfile.window_seconds)" -Message "generated Rust backend rate window for $($routePolicy.id)"
        Assert-Contains -Content $rustTrafficGenerated -Needle "problem_type: `"$($backendRateProfile.problem_type)`"" -Message "generated Rust backend rate problem type for $($routePolicy.id)"
    } elseif ($null -ne $backendRateProfileProperty) {
        throw "backend route policy rate_profile only valid for protected routes: $($routePolicy.id)"
    }
}
