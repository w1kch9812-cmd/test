$tsLines = New-Object System.Collections.Generic.List[string]
$tsLines.Add("// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.")
$tsLines.Add("// Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.")
$tsLines.Add("")
$tsLines.Add("export type GeneratedAuthRateRoutePolicy = {")
$tsLines.Add("  readonly pathSource: string;")
$tsLines.Add('  readonly methods: readonly ("GET" | "POST")[];')
$tsLines.Add("  readonly rate: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add('    readonly keyStrategy: "client_ip" | "session_or_anon";')
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("    readonly problemType: string;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_AUTH_RATE_ROUTE_POLICIES: readonly GeneratedAuthRateRoutePolicy[] = [")
foreach ($route in $authRoutes) {
    $pathSource = Convert-PathSourceToTs -Source ([string] $route.path_source)
    $methods = Convert-StringArrayToTs -Values @($route.methods)
    $keyPrefix = Convert-StringToTs -Value ([string] $route.rate_policy.key_prefix)
    $keyStrategy = Convert-StringToTs -Value ([string] $route.rate_policy.key_strategy)
    $limit = [int64] $route.rate_policy.limit
    $windowSec = [int64] $route.rate_policy.window_seconds
    $problemType = Convert-StringToTs -Value ([string] $route.rate_policy.problem_type)
    $tsLines.Add("  {")
    $tsLines.Add("    pathSource: `"$pathSource`",")
    $tsLines.Add("    methods: $methods,")
    $tsLines.Add("    rate: {")
    $tsLines.Add("      keyPrefix: `"$keyPrefix`",")
    $tsLines.Add("      keyStrategy: `"$keyStrategy`",")
    $tsLines.Add("      limit: $limit,")
    $tsLines.Add("      windowSec: $windowSec,")
    $tsLines.Add("      problemType: `"$problemType`",")
    $tsLines.Add("    },")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedPageRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix" | "prefix_suffix";')
$tsLines.Add("  readonly path?: string;")
$tsLines.Add("  readonly pathSource?: string;")
$tsLines.Add("  readonly prefix?: string;")
$tsLines.Add("  readonly prefixSource?: string;")
$tsLines.Add("  readonly suffix?: string;")
$tsLines.Add("  readonly requiredRoles: readonly string[];")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_PAGE_ROUTE_POLICIES: readonly GeneratedPageRoutePolicy[] = [")
foreach ($route in $pageRoutes) {
    $kind = Convert-StringToTs -Value ([string] $route.path_kind)
    $requiredRoles = Convert-StringArrayToTs -Values @($route.required_roles)
    $path = Get-OptionalStringPropertyValue -Object $route -Name "path"
    $pathSource = Get-OptionalStringPropertyValue -Object $route -Name "path_source"
    $prefix = Get-OptionalStringPropertyValue -Object $route -Name "prefix"
    $prefixSource = Get-OptionalStringPropertyValue -Object $route -Name "prefix_source"
    $suffix = Get-OptionalStringPropertyValue -Object $route -Name "suffix"

    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    if ($null -ne $path) {
        $tsLines.Add("    path: `"$(Convert-StringToTs -Value $path)`",")
    }
    if ($null -ne $pathSource) {
        $tsLines.Add("    pathSource: `"$(Convert-StringToTs -Value $pathSource)`",")
    }
    if ($null -ne $prefix) {
        $tsLines.Add("    prefix: `"$(Convert-StringToTs -Value $prefix)`",")
    }
    if ($null -ne $prefixSource) {
        $tsLines.Add("    prefixSource: `"$(Convert-StringToTs -Value $prefixSource)`",")
    }
    if ($null -ne $suffix) {
        $tsLines.Add("    suffix: `"$(Convert-StringToTs -Value $suffix)`",")
    }
    $tsLines.Add("    requiredRoles: $requiredRoles,")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedPublicMapRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix";')
$tsLines.Add("  readonly pathSource: string;")
$tsLines.Add("  readonly exposure: {")
$tsLines.Add('    readonly class: "public_derived";')
$tsLines.Add("    readonly allowedDataClasses: readonly string[];")
$tsLines.Add('    readonly rawRecordAccess: "forbidden";')
$tsLines.Add('    readonly bulkExport: "forbidden";')
$tsLines.Add("  };")
$tsLines.Add("  readonly rate: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = [")
foreach ($route in $publicRoutes) {
    $kind = [string] $route.proxy_path_kind
    $pathSource = Convert-PathSourceToTs -Source ([string] $route.proxy_path_source)
    $keyPrefix = Convert-StringToTs -Value ([string] $route.rate_policy.key_prefix)
    $limit = [int64] $route.rate_policy.limit
    $windowSec = [int64] $route.rate_policy.window_seconds
    $allowedDataClasses = Convert-StringArrayToTs -Values @($route.data_exposure_policy.allowed_data_classes)
    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    $tsLines.Add("    pathSource: `"$pathSource`",")
    $tsLines.Add("    exposure: {")
    $tsLines.Add("      class: `"public_derived`",")
    $tsLines.Add("      allowedDataClasses: $allowedDataClasses,")
    $tsLines.Add("      rawRecordAccess: `"forbidden`",")
    $tsLines.Add("      bulkExport: `"forbidden`",")
    $tsLines.Add("    },")
    $tsLines.Add("    rate: { keyPrefix: `"$keyPrefix`", limit: $limit, windowSec: $windowSec },")
    $tsLines.Add("  },")
}
$tsLines.Add("];")
$tsLines.Add("")
$tsLines.Add("export type GeneratedApiProxyRoutePolicy = {")
$tsLines.Add('  readonly kind: "exact" | "prefix" | "template";')
$tsLines.Add("  readonly targetPath: string;")
$tsLines.Add('  readonly methods: readonly ("GET" | "POST" | "PUT" | "PATCH" | "DELETE")[];')
$tsLines.Add('  readonly exposureClass: "public_derived" | "authenticated_user" | "privileged";')
$tsLines.Add("  readonly requiredRoles: readonly string[];")
$tsLines.Add("  readonly rate?: {")
$tsLines.Add("    readonly keyPrefix: string;")
$tsLines.Add('    readonly keyStrategy: "session_sub";')
$tsLines.Add("    readonly limit: number;")
$tsLines.Add("    readonly windowSec: number;")
$tsLines.Add("    readonly problemType: string;")
$tsLines.Add("  };")
$tsLines.Add("};")
$tsLines.Add("")
$tsLines.Add("export const GENERATED_API_PROXY_ROUTE_POLICIES: readonly GeneratedApiProxyRoutePolicy[] = [")
foreach ($route in $apiProxyRoutes) {
    $kind = Convert-StringToTs -Value ([string] $route.target_path_kind)
    $targetPath = Convert-StringToTs -Value ([string] $route.target_path)
    $methods = Convert-StringArrayToTs -Values @($route.methods)
    $exposureClass = Convert-StringToTs -Value ([string] $route.exposure_class)
    $requiredRoles = Convert-StringArrayToTs -Values @(Get-OptionalPropertyValue -Object $route -Name "required_roles")
    $rateProfileId = Get-OptionalStringPropertyValue -Object $route -Name "rate_profile"
    $tsLines.Add("  {")
    $tsLines.Add("    kind: `"$kind`",")
    $tsLines.Add("    targetPath: `"$targetPath`",")
    $tsLines.Add("    methods: $methods,")
    $tsLines.Add("    exposureClass: `"$exposureClass`",")
    $tsLines.Add("    requiredRoles: $requiredRoles,")
    if ($null -ne $rateProfileId) {
        $rateProfile = Get-RouteRateProfile -Profiles $routeRateProfiles -Id $rateProfileId
        $keyPrefix = Convert-StringToTs -Value ([string] $rateProfile.key_prefix)
        $keyStrategy = Convert-StringToTs -Value ([string] $rateProfile.key_strategy)
        $limit = [int64] $rateProfile.limit
        $windowSec = [int64] $rateProfile.window_seconds
        $problemType = Convert-StringToTs -Value ([string] $rateProfile.problem_type)
        $tsLines.Add("    rate: {")
        $tsLines.Add("      keyPrefix: `"$keyPrefix`",")
        $tsLines.Add("      keyStrategy: `"$keyStrategy`",")
        $tsLines.Add("      limit: $limit,")
        $tsLines.Add("      windowSec: $windowSec,")
        $tsLines.Add("      problemType: `"$problemType`",")
        $tsLines.Add("    },")
    }
    $tsLines.Add("  },")
}
$tsLines.Add("];")

$tsPath = Resolve-RepoPath -RelativePath "apps/web/lib/policies/traffic-auth-policy.generated.ts"
New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($tsPath)) | Out-Null
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($tsPath, (($tsLines -join "`n") + "`n"), $utf8NoBom)
