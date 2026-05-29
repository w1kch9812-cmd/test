Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-event-receiver-contract.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-event-receiver-contract-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Invoke-Checker {
    param([string] $Root, [string] $PlatformCoreRoot = "")

    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    if (![string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
        $args += @("-PlatformCoreRoot", $PlatformCoreRoot)
    }

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe @args 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
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

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function New-PinnedContractJson {
    param([string] $AnchorEffect = "enqueue_anchor_projection_import")

    return @"
{
  "schema_version": "gongzzang.platform_core_webhook_receiver_contract_pin.v1",
  "source_repo": "platform-core",
  "source_path": "docs/events/webhook/receiver-contract.v1.example.json",
  "source_schema_version": "platform-core.webhook_receiver_contract.v1",
  "consumer_slug": "gongzzang",
  "endpoint_path": "/platform-core/events",
  "required_headers": [
    "x-platform-core-event-id",
    "x-platform-core-event-type",
    "x-platform-core-outbox-scope",
    "x-platform-core-signature",
    "x-platform-core-timestamp"
  ],
  "accepted_status_codes": [200, 202, 204],
  "required_ack_status": "accepted",
  "supported_events": [
    {
      "event_type": "catalog.industrial_complex.gold_pointer.published.v1",
      "required_effect": "invalidate_catalog_cache",
      "envelope_ref": "docs/events/webhook/outbox-webhook-envelope.v1.example.json"
    },
    {
      "event_type": "catalog.parcel_marker_anchor.snapshot.published.v1",
      "required_effect": "$AnchorEffect",
      "envelope_ref": "docs/events/webhook/parcel-marker-anchor-snapshot-envelope.v1.example.json"
    }
  ]
}
"@
}

function New-PlatformCoreContractJson {
    param([string] $AnchorEffect = "enqueue_anchor_projection_import")

    return @"
{
  "schema_version": "platform-core.webhook_receiver_contract.v1",
  "required_headers": [
    "x-platform-core-event-id",
    "x-platform-core-event-type",
    "x-platform-core-outbox-scope",
    "x-platform-core-signature",
    "x-platform-core-timestamp"
  ],
  "supported_events": [
    {
      "event_type": "catalog.industrial_complex.gold_pointer.published.v1",
      "envelope_ref": "docs/events/webhook/outbox-webhook-envelope.v1.example.json",
      "consumers": [
        {"slug":"gongzzang","required_effect":"invalidate_catalog_cache"}
      ]
    },
    {
      "event_type": "catalog.parcel_marker_anchor.snapshot.published.v1",
      "envelope_ref": "docs/events/webhook/parcel-marker-anchor-snapshot-envelope.v1.example.json",
      "consumers": [
        {"slug":"gongzzang","required_effect":"$AnchorEffect"}
      ]
    }
  ],
  "consumers": [
    {
      "slug": "gongzzang",
      "endpoint_path": "/platform-core/events",
      "ack_contract": {
        "accepted_status_codes": [200, 202, 204],
        "required_ack_status": "accepted"
      }
    }
  ]
}
"@
}

function Write-Route {
    param(
        [string] $Root,
        [bool] $OmitAnchorEvent = $false,
        [bool] $ExtraLocalEvent = $false,
        [bool] $MissingHeader = $false
    )

    $anchor = if ($OmitAnchorEvent) { "" } else { '"catalog.parcel_marker_anchor.snapshot.published.v1"' }
    $extra = if ($ExtraLocalEvent) { '"catalog.local_only.v1"' } else { "" }
    $headers = @(
        '"x-platform-core-event-id"',
        '"x-platform-core-event-type"',
        '"x-platform-core-outbox-scope"',
        '"x-platform-core-signature"',
        '"x-platform-core-timestamp"'
    )
    if ($MissingHeader) {
        $headers = @($headers | Where-Object { $_ -ne '"x-platform-core-outbox-scope"' })
    }

    Write-File -Root $Root -RelativePath "apps\web\app\platform-core\events\route.ts" -Content @"
$($headers -join [Environment]::NewLine)
"catalog.industrial_complex.gold_pointer.published.v1"
$anchor
$extra
"invalidate_catalog_cache"
"enqueue_anchor_projection_import"
status: "accepted"
status: 202
"@
}

function Write-ReceiverTest {
    param([string] $Root, [bool] $OmitAnchorCoverage = $false)

    $anchorCoverage = if ($OmitAnchorCoverage) { "" } else { @'
catalog.parcel_marker_anchor.snapshot.published.v1
enqueue_anchor_projection_import
'@ }

    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-events.test.ts" -Content @"
catalog.industrial_complex.gold_pointer.published.v1
invalidate_catalog_cache
$anchorCoverage
"@
}

function Write-Fixture {
    param(
        [string] $Root,
        [string] $PlatformCoreRoot = "",
        [string] $PinAnchorEffect = "enqueue_anchor_projection_import",
        [string] $CoreAnchorEffect = "enqueue_anchor_projection_import",
        [bool] $OmitAnchorEvent = $false,
        [bool] $ExtraLocalEvent = $false,
        [bool] $MissingHeader = $false,
        [bool] $OmitAnchorCoverage = $false
    )

    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-webhook-receiver-contract.v1.pin.json" -Content (New-PinnedContractJson -AnchorEffect $PinAnchorEffect)
    Write-Route -Root $Root -OmitAnchorEvent $OmitAnchorEvent -ExtraLocalEvent $ExtraLocalEvent -MissingHeader $MissingHeader
    Write-ReceiverTest -Root $Root -OmitAnchorCoverage $OmitAnchorCoverage

    if (![string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
        Write-File -Root $PlatformCoreRoot -RelativePath "docs\events\webhook\receiver-contract.v1.example.json" -Content (New-PlatformCoreContractJson -AnchorEffect $CoreAnchorEffect)
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-Fixture -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean event receiver contract check exit code mismatch"
Assert-Contains $clean.Output "platform-core-event-receiver-contract-ok"

$crossRepoRoot = Join-Path $TempRoot "cross-repo"
$crossCoreRoot = Join-Path $TempRoot "platform-core"
Write-Fixture -Root $crossRepoRoot -PlatformCoreRoot $crossCoreRoot
$crossRepo = Invoke-Checker -Root $crossRepoRoot -PlatformCoreRoot $crossCoreRoot
Assert-Equals $crossRepo.ExitCode 0 "Cross-repo event receiver contract check exit code mismatch"

$missingRouteEventRoot = Join-Path $TempRoot "missing-route-event"
Write-Fixture -Root $missingRouteEventRoot -OmitAnchorEvent $true
$missingRouteEvent = Invoke-Checker -Root $missingRouteEventRoot
Assert-Equals $missingRouteEvent.ExitCode 1 "Missing receiver event check exit code mismatch"
Assert-Contains $missingRouteEvent.Output "missing Gongzzang receiver event"

$extraLocalEventRoot = Join-Path $TempRoot "extra-local-event"
Write-Fixture -Root $extraLocalEventRoot -ExtraLocalEvent $true
$extraLocalEvent = Invoke-Checker -Root $extraLocalEventRoot
Assert-Equals $extraLocalEvent.ExitCode 1 "Extra local receiver event check exit code mismatch"
Assert-Contains $extraLocalEvent.Output "not declared by Platform Core"

$missingHeaderRoot = Join-Path $TempRoot "missing-header"
Write-Fixture -Root $missingHeaderRoot -MissingHeader $true
$missingHeader = Invoke-Checker -Root $missingHeaderRoot
Assert-Equals $missingHeader.ExitCode 1 "Missing receiver header check exit code mismatch"
Assert-Contains $missingHeader.Output "missing required header"

$missingCoverageRoot = Join-Path $TempRoot "missing-coverage"
Write-Fixture -Root $missingCoverageRoot -OmitAnchorCoverage $true
$missingCoverage = Invoke-Checker -Root $missingCoverageRoot
Assert-Equals $missingCoverage.ExitCode 1 "Missing receiver test coverage check exit code mismatch"
Assert-Contains $missingCoverage.Output "missing receiver unit-test coverage"

$driftRoot = Join-Path $TempRoot "contract-drift"
$driftCoreRoot = Join-Path $TempRoot "drift-platform-core"
Write-Fixture -Root $driftRoot -PlatformCoreRoot $driftCoreRoot -CoreAnchorEffect "changed_effect"
$drift = Invoke-Checker -Root $driftRoot -PlatformCoreRoot $driftCoreRoot
Assert-Equals $drift.ExitCode 1 "Platform Core contract drift check exit code mismatch"
Assert-Contains $drift.Output "pinned contract"
Assert-Contains $drift.Output "Platform Core source"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-platform-core-event-receiver-contract-tests-ok"
