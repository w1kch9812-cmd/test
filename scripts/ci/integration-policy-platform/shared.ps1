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
    return (Read-TextFile -RelativePath $RelativePath) | ConvertFrom-Json
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

function Assert-FileExists {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
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

function Get-JsonProperty {
    param([object] $Object, [string] $Name)
    if ($null -eq $Object -or $null -eq $Object.PSObject.Properties[$Name]) {
        return $null
    }
    return $Object.PSObject.Properties[$Name].Value
}

function Assert-JsonArrayContains {
    param([object[]] $Values, [string] $Expected, [string] $Message)
    $strings = @($Values | ForEach-Object { [string] $_ })
    if (!($strings -contains $Expected)) {
        throw "$Message missing '$Expected'"
    }
}

function Assert-NotEmptyString {
    param([object] $Value, [string] $Message)
    if ([string]::IsNullOrWhiteSpace([string] $Value)) {
        throw "$Message must be set"
    }
}

function Assert-DateNotExpired {
    param([string] $Value, [string] $Message)
    $expiresAt = [DateTimeOffset]::Parse($Value, [System.Globalization.CultureInfo]::InvariantCulture)
    $todayUtc = [DateTimeOffset]::UtcNow.Date
    if ($expiresAt.UtcDateTime.Date -lt $todayUtc) {
        throw "$Message expired_at '$Value' is in the past"
    }
}
