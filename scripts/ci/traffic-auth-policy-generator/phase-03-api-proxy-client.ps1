$apiClientLines = New-Object System.Collections.Generic.List[string]
$apiClientLines.Add("// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.")
$apiClientLines.Add("// Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.")
$apiClientLines.Add("")
$apiClientLines.Add('import type { Options as KyOptions } from "ky";')
$apiClientLines.Add('import { api } from "@/lib/api";')
$apiClientLines.Add("")
$apiClientLines.Add('export type ApiProxyRequestOptions = Omit<KyOptions, "prefixUrl" | "method">;')
$apiClientLines.Add('export type ApiProxyJsonRequestOptions = Omit<KyOptions, "prefixUrl" | "method" | "body" | "json"> & {')
$apiClientLines.Add("  readonly json?: unknown;")
$apiClientLines.Add("};")
$apiClientLines.Add("")
$apiClientLines.Add("function encodePathParam(value: string): string {")
$apiClientLines.Add("  return encodeURIComponent(value);")
$apiClientLines.Add("}")
$apiClientLines.Add("")
$apiClientLines.Add("function toJsonRequestOptions(options?: ApiProxyJsonRequestOptions): KyOptions | undefined {")
$apiClientLines.Add("  if (options === undefined) {")
$apiClientLines.Add("    return undefined;")
$apiClientLines.Add("  }")
$apiClientLines.Add("  const { json, ...rest } = options;")
$apiClientLines.Add("  if (json === undefined) {")
$apiClientLines.Add("    return rest;")
$apiClientLines.Add("  }")
$apiClientLines.Add("  return { ...rest, json };")
$apiClientLines.Add("}")
$apiClientLines.Add("")
$apiClientLines.Add("export const API_PROXY_CLIENT_OPERATIONS = {")
foreach ($route in $apiProxyRoutes) {
    $operationName = Convert-PolicyIdToOperationName -Id ([string] $route.id)
    $targetPath = Convert-StringToTs -Value ([string] $route.target_path)
    $methods = Convert-StringArrayToTs -Values @($route.methods)
    $sourcePolicyId = Convert-StringToTs -Value ([string] $route.id)
    $apiClientLines.Add("  ${operationName}: {")
    $apiClientLines.Add("    sourcePolicyId: `"$sourcePolicyId`",")
    $apiClientLines.Add("    targetPath: `"$targetPath`",")
    $apiClientLines.Add("    methods: $methods,")
    $apiClientLines.Add("  },")
}
$apiClientLines.Add("} as const;")
$apiClientLines.Add("")
$apiClientLines.Add("export const apiProxyClient = {")
foreach ($route in $apiProxyRoutes) {
    $operationName = Convert-PolicyIdToOperationName -Id ([string] $route.id)
    $targetPath = [string] $route.target_path
    $params = @(Get-ApiProxyPathParameterNames -TargetPath $targetPath)
    $paramsType = Format-ApiProxyParamsType -Names $params
    $pathExpression = Convert-ApiProxyTargetPathToTsExpression -TargetPath $targetPath
    $apiClientLines.Add("  ${operationName}: {")
    foreach ($methodValue in @($route.methods)) {
        $method = [string] $methodValue
        $methodName = Get-RequestMethodName -Method $method
        if ($method -eq "GET" -or $method -eq "DELETE") {
            $signature = if ($params.Count -eq 0) {
                "options?: ApiProxyRequestOptions"
            } else {
                "params: $paramsType, options?: ApiProxyRequestOptions"
            }
            $apiClientLines.Add("    ${methodName}: ($signature) => api.$methodName($pathExpression, options),")
            $apiClientLines.Add("    $($methodName)Json: <T>($signature) => api.$methodName($pathExpression, options).json<T>(),")
        } else {
            $signature = if ($params.Count -eq 0) {
                "options?: ApiProxyJsonRequestOptions"
            } else {
                "params: $paramsType, options?: ApiProxyJsonRequestOptions"
            }
            $apiClientLines.Add("    ${methodName}: ($signature) => api.$methodName($pathExpression, toJsonRequestOptions(options)),")
            $apiClientLines.Add("    $($methodName)Json: <T>($signature) => api.$methodName($pathExpression, toJsonRequestOptions(options)).json<T>(),")
        }
    }
    $apiClientLines.Add("  },")
}
$apiClientLines.Add("} as const;")

$apiClientPath = Resolve-RepoPath -RelativePath "apps/web/lib/api/api-proxy-client.generated.ts"
New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetDirectoryName($apiClientPath)) | Out-Null
[System.IO.File]::WriteAllText($apiClientPath, (($apiClientLines -join "`n") + "`n"), $utf8NoBom)
