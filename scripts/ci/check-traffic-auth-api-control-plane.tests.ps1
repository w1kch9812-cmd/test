Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-traffic-auth-policy-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }
$TempParent = Join-Path $RepoRoot "target\check-traffic-auth-api-control-plane-tests"
$FixtureRoot = Join-Path $TempParent ([guid]::NewGuid().ToString("N"))
$FixturePath = Join-Path $FixtureRoot "apps\web\lib\api\raw-api-transport-fixture.ts"

function Copy-RepoFile {
    param([string] $RelativePath)
    $source = Join-Path $RepoRoot ($RelativePath -replace "/", "\")
    if (!(Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required fixture source is missing: $RelativePath"
    }
    $destination = Join-Path $FixtureRoot ($RelativePath -replace "/", "\")
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Force
}

function Copy-RepoDirectoryFiles {
    param([string] $RelativePath, [string] $Filter)
    $sourceRoot = Join-Path $RepoRoot ($RelativePath -replace "/", "\")
    if (!(Test-Path -LiteralPath $sourceRoot -PathType Container)) {
        throw "Required fixture source directory is missing: $RelativePath"
    }
    $sourceFiles = @(
        Get-ChildItem -LiteralPath $sourceRoot -File -Filter $Filter -Recurse |
            Sort-Object FullName
    )
    if ($sourceFiles.Count -eq 0) {
        throw "Required fixture source directory has no files: $RelativePath"
    }
    foreach ($sourceFile in $sourceFiles) {
        $relativeFromSourceRoot = $sourceFile.FullName.Substring($sourceRoot.Length).TrimStart("\", "/")
        $destination = Join-Path (Join-Path $FixtureRoot ($RelativePath -replace "/", "\")) $relativeFromSourceRoot
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath $sourceFile.FullName -Destination $destination -Force
    }
}

function Write-FixtureRepo {
    New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null
    foreach ($relativePath in @(
            "docs/architecture/traffic-auth-policy-registry.v1.json",
            "docs/architecture/platform-core-boundary.v1.json",
            ".github/workflows/ci.yml",
            "apps/web/proxy.ts",
            "apps/web/lib/routes.ts",
            "apps/web/app/api/proxy/[...path]/route.ts",
            "apps/web/lib/policies/traffic-auth-policy.generated.ts",
            "services/api/src/listing_marker_policy.rs",
            "services/api/src/traffic_auth_policy.rs",
            "services/api/src/main.rs",
            "services/api/src/app.rs",
            "services/api/src/routes/health.rs"
        )) {
        Copy-RepoFile -RelativePath $relativePath
    }

    $flatListingMarkerServing = Join-Path $RepoRoot "services\api\src\listing_marker_serving.rs"
    if (Test-Path -LiteralPath $flatListingMarkerServing -PathType Leaf) {
        Copy-RepoFile -RelativePath "services/api/src/listing_marker_serving.rs"
    } else {
        Copy-RepoDirectoryFiles -RelativePath "services/api/src/listing_marker_serving" -Filter "*.rs"
    }
}

function Invoke-Checker {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe `
        -NoProfile `
        -ExecutionPolicy Bypass `
        -File $ScriptPath `
        -Root $FixtureRoot `
        2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)
    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

try {
    Write-FixtureRepo
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $FixturePath) | Out-Null
    Set-Content -LiteralPath $FixturePath -Encoding UTF8 -Value @'
import { api } from "@/lib/api";

export async function rawApiTransportFixture(): Promise<unknown> {
  return api.get("listings").json<unknown>();
}
'@

    $result = Invoke-Checker
    Assert-Equals $result.ExitCode 1 "direct API transport fixture exit code mismatch"
    Assert-Contains $result.Output "direct API transport usage"
    Assert-Contains $result.Output "apps/web/lib/api/raw-api-transport-fixture.ts"

    Write-Host "traffic-auth-api-control-plane-tests-ok"
    exit 0
} finally {
    if (Test-Path -LiteralPath $FixtureRoot -PathType Container) {
        Remove-Item -LiteralPath $FixtureRoot -Recurse -Force
    }
}
