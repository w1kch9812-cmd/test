$ExpectedSchemaVersion = "gongzzang.traffic_auth_policy_registry.v1"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)
    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-JsonFile {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return (Get-Content -LiteralPath $path -Raw -Encoding UTF8) | ConvertFrom-Json
}

function Format-NumberLiteral {
    param([int64] $Value)
    $digits = [string] $Value
    if ($digits.Length -le 3) {
        return $digits
    }
    $groups = New-Object System.Collections.Generic.List[string]
    while ($digits.Length -gt 3) {
        $groups.Insert(0, $digits.Substring($digits.Length - 3))
        $digits = $digits.Substring(0, $digits.Length - 3)
    }
    $groups.Insert(0, $digits)
    return ($groups -join "_")
}

function Convert-PathSourceToTs {
    param([string] $Source)
    return $Source.Replace("\", "\\").Replace('"', '\"')
}

function Convert-StringToTs {
    param([string] $Value)
    return $Value.Replace("\", "\\").Replace('"', '\"')
}

function Convert-StringArrayToTs {
    param([object[]] $Values)
    $quotedValues = @($Values | ForEach-Object { "`"$(Convert-StringToTs -Value ([string] $_))`"" })
    return "[$($quotedValues -join ", ")]"
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

function Get-OptionalPropertyValue {
    param([object] $Object, [string] $Name)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return @()
    }
    return @($property.Value)
}

function Get-OptionalStringPropertyValue {
    param([object] $Object, [string] $Name)
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return [string] $property.Value
}

function Get-RouteRateProfile {
    param([object[]] $Profiles, [string] $Id)
    foreach ($profile in $Profiles) {
        if ([string] $profile.id -eq $Id) {
            return $profile
        }
    }
    throw "Missing API proxy rate profile id=$Id"
}

function Convert-PolicyIdToOperationName {
    param([string] $Id)
    $leaf = $Id.Substring($Id.LastIndexOf(".") + 1)
    $parts = @($leaf.Split("_") | Where-Object { ![string]::IsNullOrWhiteSpace([string] $_) })
    if ($parts.Count -eq 0) {
        throw "Cannot derive operation name from policy id '$Id'"
    }
    $name = [string] $parts[0]
    foreach ($part in @($parts | Select-Object -Skip 1)) {
        $value = [string] $part
        $name += $value.Substring(0, 1).ToUpperInvariant() + $value.Substring(1)
    }
    return $name
}

function Get-ApiProxyPathParameterNames {
    param([string] $TargetPath)
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($segment in @($TargetPath.Split("/"))) {
        if ($segment.StartsWith(":")) {
            $name = $segment.Substring(1)
            if ($name -notmatch '^[A-Za-z_][A-Za-z0-9_]*$') {
                throw "Unsupported API proxy template parameter '$name' in '$TargetPath'"
            }
            $names.Add($name)
        }
    }
    return @($names.ToArray())
}

function Convert-StringToTsTemplateSegment {
    param([string] $Value)
    $backslash = [string] [char] 92
    $backtick = [string] [char] 96
    $dollar = '$'
    return $Value.
        Replace($backslash, "$backslash$backslash").
        Replace($backtick, "$backslash$backtick").
        Replace($dollar, "$backslash$dollar")
}

function Convert-ApiProxyTargetPathToTsExpression {
    param([string] $TargetPath)
    $parts = New-Object System.Collections.Generic.List[string]
    $hasParameter = $false
    foreach ($segment in @($TargetPath.Split("/"))) {
        if ([string]::IsNullOrWhiteSpace($segment)) {
            continue
        }
        if ($segment.StartsWith(":")) {
            $hasParameter = $true
            $parts.Add('${encodePathParam(params.' + $segment.Substring(1) + ')}')
        } else {
            $parts.Add((Convert-StringToTsTemplateSegment -Value $segment))
        }
    }
    if ($parts.Count -eq 0) {
        throw "API proxy target_path cannot be empty"
    }
    $path = $parts.ToArray() -join "/"
    if ($hasParameter) {
        $tick = [char] 96
        return "$tick$path$tick"
    }
    return "`"$(Convert-StringToTs -Value $path)`""
}

function Format-ApiProxyParamsType {
    param([string[]] $Names)
    if ($Names.Count -eq 0) {
        return ""
    }
    $fields = @($Names | ForEach-Object { "readonly $($_): string" })
    return "{ $($fields -join "; ") }"
}

function Get-RequestMethodName {
    param([string] $Method)
    switch ($Method) {
        "GET" { return "get" }
        "POST" { return "post" }
        "PUT" { return "put" }
        "PATCH" { return "patch" }
        "DELETE" { return "delete" }
        default { throw "Unsupported API proxy client method '$Method'" }
    }
}
