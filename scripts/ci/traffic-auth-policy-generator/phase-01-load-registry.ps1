$registry = Read-JsonFile -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
if ($registry.schema_version -ne $ExpectedSchemaVersion) {
    throw "Unsupported schema_version '$($registry.schema_version)'"
}

$publicRoutes = @($registry.public_route_policies)
$authRoutes = @($registry.auth_route_policies)
$pageRoutes = @($registry.page_route_policies)
$routeRateProfiles = @($registry.route_rate_profiles)
$apiProxyRoutes = @($registry.api_proxy_route_policies)
$backendRoutes = @($registry.backend_route_policies)
