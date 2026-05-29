[CmdletBinding()]
param(
    [string] $Root = "",
    [string] $PlatformCoreRoot = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $scriptRoot = $PSScriptRoot
    if ([string]::IsNullOrWhiteSpace($scriptRoot)) {
        $scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
    }
    $Root = Join-Path $scriptRoot "..\.."
}

$PinRelativePath = "docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json"
$RouteRelativePath = "apps/web/app/platform-core/events/route.ts"
$TestRelativePath = "apps/web/tests/unit/platform-core-events.test.ts"
$PlatformCoreContractRelativePath = "docs/events/webhook/receiver-contract.v1.example.json"

function Resolve-RepoPath {
    param([string] $RootPath, [string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $RootPath $RelativePath))
}

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
        throw "platform-core-event-receiver-contract: missing array '$Name'"
    }
    return @($value)
}

function Get-RequiredString {
    param([object] $Object, [string] $Name)

    $value = [string] (Get-PropertyValue -Object $Object -Name $Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "platform-core-event-receiver-contract: missing string '$Name'"
    }
    return $value
}

function Read-JsonFile {
    param([string] $Path, [string] $Label)

    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "platform-core-event-receiver-contract: missing $Label"
    }
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Normalize-StringArray {
    param([object[]] $Values)

    return @($Values | ForEach-Object { [string] $_ } | Sort-Object -Unique)
}

function Get-PinnedContract {
    param([object] $Pin)

    $schema = Get-RequiredString -Object $Pin -Name "schema_version"
    if ($schema -ne "gongzzang.platform_core_webhook_receiver_contract_pin.v1") {
        throw "platform-core-event-receiver-contract: pin schema_version mismatch"
    }

    $events = @(Get-RequiredArray -Object $Pin -Name "supported_events" | ForEach-Object {
        [pscustomobject]@{
            event_type = Get-RequiredString -Object $_ -Name "event_type"
            required_effect = Get-RequiredString -Object $_ -Name "required_effect"
            envelope_ref = Get-RequiredString -Object $_ -Name "envelope_ref"
        }
    } | Sort-Object -Property event_type)

    [pscustomobject]@{
        source_schema_version = Get-RequiredString -Object $Pin -Name "source_schema_version"
        consumer_slug = Get-RequiredString -Object $Pin -Name "consumer_slug"
        endpoint_path = Get-RequiredString -Object $Pin -Name "endpoint_path"
        required_headers = Normalize-StringArray -Values (Get-RequiredArray -Object $Pin -Name "required_headers")
        accepted_status_codes = @(
            Get-RequiredArray -Object $Pin -Name "accepted_status_codes" |
                ForEach-Object { [int] $_ } |
                Sort-Object -Unique
        )
        required_ack_status = Get-RequiredString -Object $Pin -Name "required_ack_status"
        supported_events = $events
    }
}

function Get-PlatformCoreGongzzangContract {
    param([object] $Source)

    $sourceSchema = Get-RequiredString -Object $Source -Name "schema_version"
    $consumerSlug = "gongzzang"
    $consumer = @(Get-RequiredArray -Object $Source -Name "consumers" | Where-Object {
            (Get-RequiredString -Object $_ -Name "slug") -eq $consumerSlug
        })
    if ($consumer.Count -ne 1) {
        throw "platform-core-event-receiver-contract: Platform Core source must declare one Gongzzang consumer"
    }

    $ack = Get-PropertyValue -Object $consumer[0] -Name "ack_contract"
    if ($null -eq $ack) {
        throw "platform-core-event-receiver-contract: Platform Core Gongzzang consumer is missing ack_contract"
    }

    $events = @()
    foreach ($event in Get-RequiredArray -Object $Source -Name "supported_events") {
        $gongzzangConsumers = @(Get-RequiredArray -Object $event -Name "consumers" | Where-Object {
                (Get-RequiredString -Object $_ -Name "slug") -eq $consumerSlug
            })
        if ($gongzzangConsumers.Count -eq 0) {
            continue
        }
        if ($gongzzangConsumers.Count -ne 1) {
            throw "platform-core-event-receiver-contract: duplicate Gongzzang consumer for supported event"
        }
        $events += [pscustomobject]@{
            event_type = Get-RequiredString -Object $event -Name "event_type"
            required_effect = Get-RequiredString -Object $gongzzangConsumers[0] -Name "required_effect"
            envelope_ref = Get-RequiredString -Object $event -Name "envelope_ref"
        }
    }

    [pscustomobject]@{
        source_schema_version = $sourceSchema
        consumer_slug = $consumerSlug
        endpoint_path = Get-RequiredString -Object $consumer[0] -Name "endpoint_path"
        required_headers = Normalize-StringArray -Values (Get-RequiredArray -Object $Source -Name "required_headers")
        accepted_status_codes = @(
            Get-RequiredArray -Object $ack -Name "accepted_status_codes" |
                ForEach-Object { [int] $_ } |
                Sort-Object -Unique
        )
        required_ack_status = Get-RequiredString -Object $ack -Name "required_ack_status"
        supported_events = @($events | Sort-Object -Property event_type)
    }
}

function ConvertTo-ContractSignature {
    param([object] $Contract)

    return ([pscustomobject]@{
            source_schema_version = $Contract.source_schema_version
            consumer_slug = $Contract.consumer_slug
            endpoint_path = $Contract.endpoint_path
            required_headers = @($Contract.required_headers)
            accepted_status_codes = @($Contract.accepted_status_codes)
            required_ack_status = $Contract.required_ack_status
            supported_events = @($Contract.supported_events | ForEach-Object {
                    [pscustomobject]@{
                        event_type = $_.event_type
                        required_effect = $_.required_effect
                        envelope_ref = $_.envelope_ref
                    }
                })
        } | ConvertTo-Json -Depth 8 -Compress)
}

function Get-RouteEventTypes {
    param([string] $Content)

    return @(
        [regex]::Matches($Content, '"(catalog\.[a-z0-9_.-]+\.v[0-9]+)"') |
            ForEach-Object { $_.Groups[1].Value } |
            Sort-Object -Unique
    )
}

function Assert-ReceiverMatchesPinnedContract {
    param([object] $Contract, [string] $RouteContent, [string] $TestContent)

    if ($Contract.consumer_slug -ne "gongzzang") {
        throw "platform-core-event-receiver-contract: pin consumer_slug must be gongzzang"
    }
    if ($Contract.endpoint_path -ne "/platform-core/events") {
        throw "platform-core-event-receiver-contract: pin endpoint_path must be /platform-core/events"
    }
    if (!(@($Contract.accepted_status_codes) -contains 202)) {
        throw "platform-core-event-receiver-contract: pin accepted_status_codes must include 202"
    }
    if ($Contract.required_ack_status -ne "accepted") {
        throw "platform-core-event-receiver-contract: pin required_ack_status must be accepted"
    }
    if (!$RouteContent.Contains('status: "accepted"')) {
        throw "platform-core-event-receiver-contract: receiver must return accepted ack status"
    }
    if (![regex]::IsMatch($RouteContent, "status\s*:\s*202")) {
        throw "platform-core-event-receiver-contract: receiver must return accepted status code 202"
    }

    foreach ($header in @($Contract.required_headers)) {
        if (!$RouteContent.Contains($header)) {
            throw "platform-core-event-receiver-contract: missing required header '$header'"
        }
    }

    $expectedEvents = Normalize-StringArray -Values (@($Contract.supported_events | ForEach-Object { $_.event_type }))
    $routeEvents = Get-RouteEventTypes -Content $RouteContent

    foreach ($eventType in $expectedEvents) {
        if (!($routeEvents -contains $eventType)) {
            throw "platform-core-event-receiver-contract: missing Gongzzang receiver event '$eventType'"
        }
    }
    foreach ($eventType in $routeEvents) {
        if (!($expectedEvents -contains $eventType)) {
            throw "platform-core-event-receiver-contract: Gongzzang receiver event '$eventType' is not declared by Platform Core"
        }
    }

    foreach ($event in @($Contract.supported_events)) {
        if (!$RouteContent.Contains($event.required_effect)) {
            throw "platform-core-event-receiver-contract: missing required effect '$($event.required_effect)'"
        }
        if (!$TestContent.Contains($event.event_type) -or !$TestContent.Contains($event.required_effect)) {
            throw "platform-core-event-receiver-contract: missing receiver unit-test coverage for '$($event.event_type)'"
        }
    }
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$pinPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $PinRelativePath
$routePath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $RouteRelativePath
$testPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $TestRelativePath

$pin = Read-JsonFile -Path $pinPath -Label $PinRelativePath
$pinnedContract = Get-PinnedContract -Pin $pin

if (!(Test-Path -LiteralPath $routePath -PathType Leaf)) {
    throw "platform-core-event-receiver-contract: missing receiver route: $RouteRelativePath"
}
if (!(Test-Path -LiteralPath $testPath -PathType Leaf)) {
    throw "platform-core-event-receiver-contract: missing receiver tests: $TestRelativePath"
}

$routeContent = Get-Content -LiteralPath $routePath -Raw
$testContent = Get-Content -LiteralPath $testPath -Raw
Assert-ReceiverMatchesPinnedContract -Contract $pinnedContract -RouteContent $routeContent -TestContent $testContent

if ([string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
    $PlatformCoreRoot = Join-Path $resolvedRoot "..\platform-core"
}
$sourcePath = Resolve-RepoPath -RootPath $PlatformCoreRoot -RelativePath $PlatformCoreContractRelativePath
$sourceChecked = $false
if (Test-Path -LiteralPath $sourcePath -PathType Leaf) {
    $source = Read-JsonFile -Path $sourcePath -Label $PlatformCoreContractRelativePath
    $sourceContract = Get-PlatformCoreGongzzangContract -Source $source
    if ((ConvertTo-ContractSignature -Contract $pinnedContract) -ne (ConvertTo-ContractSignature -Contract $sourceContract)) {
        throw "platform-core-event-receiver-contract: pinned contract differs from Platform Core source"
    }
    $sourceChecked = $true
}

Write-Host "platform-core-event-receiver-contract-ok events=$(@($pinnedContract.supported_events).Count) source_checked=$sourceChecked"
