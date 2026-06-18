function Get-PropertyValue {
    param([object] $Object, [string] $Name)

    if ($null -eq $Object -or $null -eq $Object.PSObject.Properties[$Name]) {
        return $null
    }
    return $Object.PSObject.Properties[$Name].Value
}

function Get-RequiredArray {
    param([object] $Object, [string] $Name)

    $value = Get-PropertyValue -Object $Object -Name $Name
    if ($null -eq $value) {
        throw "platform-core-boundary: missing array '$Name'"
    }
    return @($value)
}

function Get-RequiredString {
    param([object] $Object, [string] $Name)

    $value = [string] (Get-PropertyValue -Object $Object -Name $Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "platform-core-boundary: missing string '$Name'"
    }
    return $value
}

function Normalize-RelativePath {
    param([string] $Path)

    $normalized = $Path -replace "\\", "/"
    if ($normalized.StartsWith("./", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(2)
    }
    while ($normalized.StartsWith("/", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(1)
    }
    return $normalized
}

function Resolve-RepoPath {
    param([string] $RootPath, [string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $RootPath $RelativePath))
}

function Read-Utf8Text {
    param([string] $Path)

    return Get-Content -LiteralPath $Path -Raw -Encoding UTF8
}

function Assert-Equal {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "platform-core-boundary: $Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-PathExists {
    param([string] $RootPath, [string] $RelativePath)

    $path = Resolve-RepoPath -RootPath $RootPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path)) {
        throw "platform-core-boundary: required path is missing: $RelativePath"
    }
}

function Assert-PathAbsent {
    param([string] $RootPath, [string] $RelativePath)

    $path = Resolve-RepoPath -RootPath $RootPath -RelativePath $RelativePath
    if (Test-Path -LiteralPath $path) {
        throw "platform-core-boundary: extracted Platform Core path must be absent: $RelativePath"
    }
}

function Get-RepoRelativePath {
    param([string] $RootPath, [string] $FullPath)

    $rootPrefix = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    $resolvedPath = [System.IO.Path]::GetFullPath($FullPath)
    if ($resolvedPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return Normalize-RelativePath -Path ($resolvedPath.Substring($rootPrefix.Length))
    }
    return Normalize-RelativePath -Path $resolvedPath
}
