function Resolve-RepoPath {
    param([string] $RelativePath)
    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-TextFile {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)
    $content = Read-TextFile -RelativePath $RelativePath
    return $content | ConvertFrom-Json
}

function Read-ListingMarkerServingSources {
    $flatPath = Resolve-RepoPath -RelativePath "services/api/src/listing_marker_serving.rs"
    if (Test-Path -LiteralPath $flatPath -PathType Leaf) {
        return Get-Content -LiteralPath $flatPath -Raw -Encoding UTF8
    }

    $modulePath = Resolve-RepoPath -RelativePath "services/api/src/listing_marker_serving"
    if (!(Test-Path -LiteralPath $modulePath -PathType Container)) {
        throw "Required listing marker serving source is missing: services/api/src/listing_marker_serving.rs or services/api/src/listing_marker_serving/mod.rs"
    }
    $moduleSources = @(
        Get-ChildItem -LiteralPath $modulePath -File -Filter "*.rs" -Recurse |
            Sort-Object FullName
    )
    if ($moduleSources.Count -eq 0) {
        throw "Required listing marker serving source is missing: services/api/src/listing_marker_serving/*.rs"
    }
    return (($moduleSources | ForEach-Object {
                Get-Content -LiteralPath $_.FullName -Raw -Encoding UTF8
            }) -join "`n")
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Content, [string] $Needle, [string] $Message)
    if (!$Content.Contains($Needle)) {
        throw "$Message missing '$Needle'"
    }
}

function Assert-RegexContains {
    param([string] $Content, [string] $Pattern, [string] $Message)
    if (![regex]::IsMatch($Content, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)) {
        throw "$Message missing pattern '$Pattern'"
    }
}

function Assert-NotContains {
    param([string] $Content, [string] $Needle, [string] $Message)
    if ($Content.Contains($Needle)) {
        throw "$Message must not contain '$Needle'"
    }
}

function Assert-Unique {
    param([object[]] $Values, [string] $Message)
    $seen = @{}
    foreach ($value in $Values) {
        $key = [string] $value
        if ($seen.ContainsKey($key)) {
            throw "$Message duplicate '$key'"
        }
        $seen[$key] = $true
    }
}

function Assert-ArrayContains {
    param([object[]] $Values, [string] $Expected, [string] $Message)
    foreach ($value in $Values) {
        if ([string] $value -eq $Expected) {
            return
        }
    }
    throw "$Message missing '$Expected'"
}

function Assert-ArrayNotContains {
    param([object[]] $Values, [string] $Forbidden, [string] $Message)
    foreach ($value in $Values) {
        if ([string] $value -eq $Forbidden) {
            throw "$Message must not contain '$Forbidden'"
        }
    }
}

function Assert-StringSetEquals {
    param([object[]] $Actual, [object[]] $Expected, [string] $Message)
    $actualValues = @($Actual | ForEach-Object { [string] $_ } | Sort-Object)
    $expectedValues = @($Expected | ForEach-Object { [string] $_ } | Sort-Object)
    $actualJoined = $actualValues -join ","
    $expectedJoined = $expectedValues -join ","
    if ($actualJoined -ne $expectedJoined) {
        throw "$Message expected=[$expectedJoined] actual=[$actualJoined]"
    }
}

function Get-RequiredProperty {
    param([object] $Object, [string] $Name, [string] $Message)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        throw "$Message missing '$Name'"
    }
    if ($null -eq $property.Value) {
        throw "$Message missing '$Name'"
    }
    return $property.Value
}

function Get-ExposureClass {
    param([object[]] $Classes, [string] $Class)
    foreach ($entry in $Classes) {
        if ($entry.class -eq $Class) {
            return $entry
        }
    }
    throw "Missing exposure class '$Class'"
}

function Get-RegexInt {
    param([string] $Content, [string] $Pattern, [string] $Field)
    $match = [regex]::Match($Content, $Pattern)
    if (!$match.Success) {
        throw "Could not find $Field with pattern $Pattern"
    }
    return [int64] (($match.Groups[1].Value) -replace "_", "")
}

function Assert-RegexInt {
    param([string] $Content, [string] $Pattern, [int64] $Expected, [string] $Field)
    $actual = Get-RegexInt -Content $Content -Pattern $Pattern -Field $Field
    Assert-Equals -Actual $actual -Expected $Expected -Message $Field
}

function Get-RouteById {
    param([object[]] $Routes, [string] $Id)
    foreach ($route in $Routes) {
        if ($route.id -eq $Id) {
            return $route
        }
    }
    throw "Missing public route policy id=$Id"
}

function Get-RuleBySourcePolicyId {
    param([object[]] $Rules, [string] $Id, [string] $Message)
    foreach ($rule in $Rules) {
        if ([string] $rule.source_policy_id -eq $Id) {
            return $rule
        }
    }
    throw "$Message missing source_policy_id=$Id"
}

function Assert-EdgeRateProjection {
    param([object] $ActualRate, [object] $ExpectedRate, [string] $ExpectedKeyStrategy, [string] $Message)
    Assert-Equals -Actual ([string] $ActualRate.key_strategy) -Expected $ExpectedKeyStrategy -Message "$Message key_strategy"
    Assert-Equals -Actual ([string] $ActualRate.key_prefix) -Expected ([string] $ExpectedRate.key_prefix) -Message "$Message key_prefix"
    Assert-Equals -Actual ([int64] $ActualRate.limit) -Expected ([int64] $ExpectedRate.limit) -Message "$Message limit"
    Assert-Equals -Actual ([int64] $ActualRate.window_seconds) -Expected ([int64] $ExpectedRate.window_seconds) -Message "$Message window_seconds"
    Assert-Equals -Actual ([string] $ActualRate.problem_type) -Expected ([string] $ExpectedRate.problem_type) -Message "$Message problem_type"
}

function Convert-PathKindToAwsWafPathMatch {
    param([string] $Kind)
    switch ($Kind) {
        "exact" { return "EXACT" }
        "prefix" { return "STARTS_WITH" }
        default { throw "Unsupported AWS WAFv2 path kind '$Kind'" }
    }
}

function Convert-RateToFiveMinuteLimit {
    param([object] $Rate)
    $limit = [int64] $Rate.limit
    $windowSeconds = [int64] $Rate.window_seconds
    if ($windowSeconds -le 0) {
        throw "Rate window_seconds must be positive for $($Rate.key_prefix)"
    }
    return [int64] [Math]::Ceiling(([double] $limit) * 300.0 / ([double] $windowSeconds))
}

function Resolve-AuthPathSource {
    param([string] $PathSource)
    switch ($PathSource) {
        "API.auth.login" { return "/api/auth/login" }
        "API.auth.callback" { return "/api/auth/callback" }
        "API.auth.refresh" { return "/api/auth/refresh" }
        "API.auth.logout" { return "/api/auth/logout" }
        default { throw "Unsupported auth path source '$PathSource'" }
    }
}

function Get-AuthPathSourcesFromRoutesTs {
    param([string] $Content)
    $sources = New-Object System.Collections.Generic.List[string]
    $matches = [regex]::Matches($Content, '(?m)^\s*([A-Za-z][A-Za-z0-9_]*)\s*:\s*"/api/auth/[^"]+"')
    foreach ($match in $matches) {
        $sources.Add("API.auth.$($match.Groups[1].Value)")
    }
    return @($sources.ToArray() | Sort-Object -Unique)
}

function Get-AxumRoutePaths {
    param([string] $Content)
    $paths = New-Object System.Collections.Generic.List[string]
    $matches = [regex]::Matches($Content, '\.route\s*\(\s*"([^"]+)"', [System.Text.RegularExpressions.RegexOptions]::Singleline)
    foreach ($match in $matches) {
        $paths.Add([string] $match.Groups[1].Value)
    }
    return @($paths.ToArray() | Sort-Object -Unique)
}

. (Join-Path $PSScriptRoot "coverage.ps1")

function Format-TsStringArray {
    param([object[]] $Values)
    $quotedValues = @($Values | ForEach-Object {
            $escaped = ([string] $_).Replace("\", "\\").Replace('"', '\"')
            "`"$escaped`""
        })
    return "[$($quotedValues -join ", ")]"
}

function Format-RustUserRoleArray {
    param([object[]] $Values)
    $roleValues = @($Values | ForEach-Object {
            $role = [string] $_
            if (!(@("Admin", "Broker", "Buyer", "Developer", "Enterprise", "Operator", "Seller") -contains $role)) {
                throw "invalid generated backend role: $role"
            }
            "UserRole::$role"
        })
    return "&[$($roleValues -join ", ")]"
}
