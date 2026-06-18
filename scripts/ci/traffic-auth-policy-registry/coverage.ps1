Set-StrictMode -Version Latest

function Get-WebSourceFiles {
    $webRoot = Resolve-RepoPath -RelativePath "apps/web"
    if (!(Test-Path -LiteralPath $webRoot -PathType Container)) {
        return @()
    }

    return @(
        Get-ChildItem -LiteralPath $webRoot -File -Recurse -Include "*.ts", "*.tsx" |
            Where-Object {
                $relative = $_.FullName.Substring($resolvedRoot.Length).TrimStart("\", "/") -replace "\\", "/"
                $extension = [System.IO.Path]::GetExtension($_.FullName)
                ($extension -eq ".ts" -or $extension -eq ".tsx") -and
                $relative -notmatch '(^|/)(\.next|node_modules|dist|build|coverage|out|target)/' -and
                $relative -notmatch '(^|/)tests?/' -and
                $relative -notmatch '\.(test|spec)\.tsx?$' -and
                $relative -ne "apps/web/lib/policies/traffic-auth-policy.generated.ts" -and
                $relative -ne "apps/web/lib/api/api-proxy-client.generated.ts" -and
                $relative -ne "apps/web/app/api/proxy/[...path]/route.ts"
            } |
            Sort-Object FullName
    )
}

function Get-DirectApiTransportUsages {
    param([System.IO.FileInfo[]] $Files)
    $allowedSources = @{
        "apps/web/lib/api.ts"                                  = $true
        "apps/web/lib/api/api-proxy-client.generated.ts"       = $true
        "apps/web/app/api/proxy/[...path]/route.ts"            = $true
        "apps/web/lib/policies/traffic-auth-policy.generated.ts" = $true
    }
    $usages = New-Object System.Collections.Generic.List[object]
    foreach ($file in $Files) {
        $content = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        $relative = $file.FullName.Substring($resolvedRoot.Length).TrimStart("\", "/") -replace "\\", "/"
        if ($allowedSources.ContainsKey($relative)) {
            continue
        }
        $matches = [regex]::Matches($content, '\bapi\s*\.\s*(get|post|put|patch|delete)\s*\(', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        foreach ($match in $matches) {
            $usages.Add([pscustomobject]@{
                    Method = ([string] $match.Groups[1].Value).ToUpperInvariant()
                    Source = $relative
                })
        }
    }
    return @($usages.ToArray())
}

function Get-StringLiterals {
    param([string] $Content)
    $pattern = @'
(?s)"((?:\\.|[^"\\])*)"|'((?:\\.|[^'\\])*)'|`([^`]*)`
'@
    $values = New-Object System.Collections.Generic.List[string]
    foreach ($match in [regex]::Matches($Content, $pattern)) {
        for ($groupIndex = 1; $groupIndex -le 3; $groupIndex += 1) {
            if ($match.Groups[$groupIndex].Success) {
                $values.Add([string] $match.Groups[$groupIndex].Value)
                break
            }
        }
    }
    return @($values.ToArray())
}

function Convert-ApiProxyTargetToCoveragePattern {
    param([string] $Path)
    $normalized = $Path.Trim()
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return $null
    }
    $normalized = $normalized -replace "\s+", ""
    $normalized = $normalized -replace '^\$\{API_PROXY_BASE\}', "/api/proxy"
    $normalized = $normalized -replace '^\$\{API\.proxy\.base\}', "/api/proxy"
    $proxyIndex = $normalized.IndexOf("/api/proxy/")
    if ($proxyIndex -ge 0) {
        $normalized = $normalized.Substring($proxyIndex + "/api/proxy/".Length)
    } elseif ($normalized -eq "/api/proxy") {
        return $null
    } else {
        $normalized = $normalized.TrimStart("/")
    }
    $queryIndex = $normalized.IndexOf("?")
    if ($queryIndex -ge 0) {
        $normalized = $normalized.Substring(0, $queryIndex)
    }
    $normalized = $normalized.Trim("/")
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return $null
    }

    $segments = @($normalized.Split("/") | Where-Object { ![string]::IsNullOrWhiteSpace([string] $_) })
    $shape = @($segments | ForEach-Object {
            $segment = [string] $_
            if ($segment.StartsWith(":") -or $segment.Contains('${') -or $segment.Contains("{")) {
                return ":*"
            }
            return $segment
        })
    return $shape -join "/"
}

function Get-ApiClientRouteUsages {
    param([System.IO.FileInfo[]] $Files)
    $usages = New-Object System.Collections.Generic.List[object]
    foreach ($file in $Files) {
        $content = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        $relative = $file.FullName.Substring($resolvedRoot.Length).TrimStart("\", "/") -replace "\\", "/"
        $matches = [regex]::Matches($content, 'api\s*\.\s*(get|post|put|patch|delete)\s*\(', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        foreach ($match in $matches) {
            $depth = 1
            $index = $match.Index + $match.Length
            $quote = [char] 0
            while ($index -lt $content.Length -and $depth -gt 0) {
                $char = $content[$index]
                if ($quote -ne [char] 0) {
                    if ($char -eq "\" -and $quote -ne '`') {
                        $index += 2
                        continue
                    }
                    if ($char -eq $quote) {
                        $quote = [char] 0
                    }
                } else {
                    if ($char -eq '"' -or $char -eq "'" -or $char -eq '`') {
                        $quote = $char
                    } elseif ($char -eq '(') {
                        $depth += 1
                    } elseif ($char -eq ')') {
                        $depth -= 1
                    }
                }
                $index += 1
            }
            if ($depth -ne 0) {
                throw "Could not parse API proxy client call in $relative"
            }
            $bodyStart = $match.Index + $match.Length
            $bodyLength = ($index - 1) - $bodyStart
            $callBody = $content.Substring($bodyStart, $bodyLength)
            foreach ($literal in (Get-StringLiterals -Content $callBody)) {
                $pattern = Convert-ApiProxyTargetToCoveragePattern -Path $literal
                if ($null -eq $pattern) {
                    continue
                }
                $usages.Add([pscustomobject]@{
                        Method  = ([string] $match.Groups[1].Value).ToUpperInvariant()
                        Pattern = $pattern
                        Source  = $relative
                    })
            }
        }
    }
    return @($usages.ToArray())
}

function Get-ApiProxyLiteralUsages {
    param([System.IO.FileInfo[]] $Files)
    $usages = New-Object System.Collections.Generic.List[object]
    foreach ($file in $Files) {
        $content = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        $relative = $file.FullName.Substring($resolvedRoot.Length).TrimStart("\", "/") -replace "\\", "/"
        foreach ($literal in (Get-StringLiterals -Content $content)) {
            if (!$literal.Contains("/api/proxy") -and !$literal.Contains('${API_PROXY_BASE}')) {
                continue
            }
            $pattern = Convert-ApiProxyTargetToCoveragePattern -Path $literal
            if ($null -eq $pattern) {
                continue
            }
            $usages.Add([pscustomobject]@{
                    Pattern = $pattern
                    Source  = $relative
                })
        }
    }
    return @($usages.ToArray())
}

function Test-ApiProxyPolicyPathCoverage {
    param([hashtable] $PolicyPathSet, [string] $Pattern)
    if ($PolicyPathSet.ContainsKey($Pattern)) {
        return $true
    }
    foreach ($policyPattern in $PolicyPathSet.Keys) {
        if ([string] $policyPattern -like "$Pattern/*") {
            return $true
        }
    }
    return $false
}
