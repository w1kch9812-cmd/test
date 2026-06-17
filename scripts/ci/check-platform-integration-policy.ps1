[CmdletBinding()]
param(
    [string] $Root = "",
    [switch] $IncludeProductionPromotion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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

$indexPath = "docs/architecture/platform-integration/index.v1.json"
$routePath = "docs/architecture/platform-integration/route-exposure-policy.v1.json"
$callMatrixPath = "docs/architecture/platform-integration/allowed-call-matrix.v1.json"
$serviceAuthPath = "docs/architecture/platform-integration/service-auth-policy.v1.json"
$webhookPath = "docs/architecture/platform-integration/webhook-policy.v1.json"
$supplyChainPath = "docs/architecture/platform-integration/supply-chain-policy.v1.json"
$operationsPath = "docs/architecture/platform-integration/operations-policy.v1.json"
$exceptionPath = "docs/architecture/platform-integration/exception-policy.v1.json"
$lakehouseRegistryPath = "docs/architecture/platform-integration/lakehouse-registry-policy.v1.json"
$trafficPath = "docs/architecture/traffic-auth-policy-registry.v1.json"
$boundaryPath = "docs/architecture/platform-core-boundary.v1.json"

$index = Read-JsonFile -RelativePath $indexPath
$routePolicy = Read-JsonFile -RelativePath $routePath
$callMatrix = Read-JsonFile -RelativePath $callMatrixPath
$serviceAuthPolicy = Read-JsonFile -RelativePath $serviceAuthPath
$webhookPolicy = Read-JsonFile -RelativePath $webhookPath
$supplyChainPolicy = Read-JsonFile -RelativePath $supplyChainPath
$operationsPolicy = Read-JsonFile -RelativePath $operationsPath
$exceptionPolicy = Read-JsonFile -RelativePath $exceptionPath
$lakehouseRegistryPolicy = Read-JsonFile -RelativePath $lakehouseRegistryPath
$trafficPolicy = Read-JsonFile -RelativePath $trafficPath
$boundary = Read-JsonFile -RelativePath $boundaryPath

Assert-Equals -Actual $index.schema_version -Expected "gongzzang.platform_integration.index.v1" -Message "index schema_version mismatch"
Assert-Equals -Actual $routePolicy.schema_version -Expected "gongzzang.platform_integration.route_exposure_policy.v1" -Message "route policy schema_version mismatch"
Assert-Equals -Actual $callMatrix.schema_version -Expected "gongzzang.platform_integration.allowed_call_matrix.v1" -Message "allowed call matrix schema_version mismatch"
Assert-Equals -Actual $serviceAuthPolicy.schema_version -Expected "gongzzang.platform_integration.service_auth_policy.v1" -Message "service auth policy schema_version mismatch"
Assert-Equals -Actual $webhookPolicy.schema_version -Expected "gongzzang.platform_integration.webhook_policy.v1" -Message "webhook policy schema_version mismatch"
Assert-Equals -Actual $supplyChainPolicy.schema_version -Expected "gongzzang.platform_integration.supply_chain_policy.v1" -Message "supply chain policy schema_version mismatch"
Assert-Equals -Actual $operationsPolicy.schema_version -Expected "gongzzang.platform_integration.operations_policy.v1" -Message "operations policy schema_version mismatch"
Assert-Equals -Actual $exceptionPolicy.schema_version -Expected "gongzzang.platform_integration.exception_policy.v1" -Message "exception policy schema_version mismatch"
Assert-Equals -Actual $lakehouseRegistryPolicy.schema_version -Expected "gongzzang.platform_integration.lakehouse_registry_policy.v1" -Message "lakehouse registry policy schema_version mismatch"
Assert-Equals -Actual $trafficPolicy.schema_version -Expected "gongzzang.traffic_auth_policy_registry.v1" -Message "traffic policy schema_version mismatch"
Assert-Equals -Actual $boundary.schema_version -Expected "gongzzang.platform_core_boundary.v1" -Message "boundary schema_version mismatch"

$components = @($index.components)
Assert-Equals -Actual $components.Count -Expected 10 -Message "platform integration component count mismatch"
Assert-Unique -Values ($components | ForEach-Object { $_.id }) -Message "platform integration component ids must be unique"
foreach ($component in $components) {
    Assert-FileExists -RelativePath ([string] $component.path)
    $componentPolicy = Read-JsonFile -RelativePath ([string] $component.path)
    Assert-Equals -Actual $componentPolicy.schema_version -Expected ([string] $component.schema_version) -Message "component schema mismatch for $($component.id)"
}

foreach ($required in @(
    "platform_integration.route_exposure",
    "platform_integration.traffic_auth",
    "platform_integration.platform_core_boundary",
    "platform_integration.allowed_call_matrix",
    "platform_integration.service_auth",
    "platform_integration.webhook",
    "platform_integration.supply_chain",
    "platform_integration.operations",
    "platform_integration.exception_policy",
    "platform_integration.lakehouse_registry"
)) {
    if (!(@($components | ForEach-Object { [string] $_.id }) -contains $required)) {
        throw "platform integration index missing component '$required'"
    }
}

$requiredIndexGuardrails = @(
    "scripts/ci/check-platform-integration-policy.ps1",
    "scripts/ci/check-lakehouse-registry-integration.ps1",
    "scripts/ci/check-traffic-auth-policy-registry.ps1",
    "scripts/ci/check-platform-core-boundary.ps1",
    "scripts/ci/check-platform-core-event-receiver-contract.ps1",
    "scripts/ci/check-platform-core-catalog-api-contract.ps1",
    "scripts/ci/check-platform-core-dependency-boundary.ps1",
    "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1",
    "scripts/ci/check-migration-version-prefixes.ps1",
    "scripts/ci/check-platform-core-anchor-inbox-db-approval.ps1"
)
$requiredProductionPromotionGuardrails = @(
    "scripts/ci/check-production-edge-admission.ps1",
    "scripts/ci/check-pulumi-local-preview.ps1",
    "scripts/ci/check-load-test-assets.ps1",
    "scripts/ci/verify-load-test-capacity-evidence.ps1"
)
$indexGuardrails = @(Get-JsonProperty -Object $index -Name "required_guardrails" | ForEach-Object { [string] $_ })
$indexProductionPromotionGuardrails = @(
    Get-JsonProperty -Object $index -Name "production_promotion_guardrails" |
        ForEach-Object { [string] $_ }
)
$guardrailsToRequire = @($requiredIndexGuardrails)
if ($IncludeProductionPromotion) {
    foreach ($guardrail in $requiredProductionPromotionGuardrails) {
        Assert-JsonArrayContains `
            -Values $indexProductionPromotionGuardrails `
            -Expected $guardrail `
            -Message "index production promotion guardrails"
    }
}
foreach ($guardrail in $guardrailsToRequire) {
    Assert-JsonArrayContains `
        -Values $indexGuardrails `
        -Expected $guardrail `
        -Message "index required guardrails"
    Assert-FileExists -RelativePath $guardrail
}

$publicRouteIds = @($trafficPolicy.public_route_policies | ForEach-Object { [string] $_.id })
foreach ($surface in @($routePolicy.surfaces | Where-Object { [string] (Get-JsonProperty -Object $_ -Name "policy_source") -eq $trafficPath })) {
    if (!($publicRouteIds -contains ([string] $surface.id))) {
        throw "route exposure surface has no matching traffic policy: $($surface.id)"
    }
}
foreach ($shape in @("bbox", "bounds", "south", "west", "north", "east")) {
    Assert-JsonArrayContains -Values @($routePolicy.forbidden_public_shapes) -Expected $shape -Message "route exposure forbidden public shapes"
}

$allowedCalls = @($callMatrix.allowed_calls)
Assert-Equals -Actual $callMatrix.default_decision -Expected "deny" -Message "allowed call matrix default decision mismatch"
Assert-Unique -Values ($allowedCalls | ForEach-Object { $_.id }) -Message "allowed call ids must be unique"

$activeCalls = @($allowedCalls | Where-Object { [string] $_.status -eq "active" })
foreach ($call in $activeCalls) {
    Assert-NotEmptyString -Value $call.id -Message "active allowed call id"
    Assert-NotEmptyString -Value $call.source_repo -Message "active allowed call source_repo for $($call.id)"
    Assert-NotEmptyString -Value $call.source_service -Message "active allowed call source_service for $($call.id)"
    Assert-NotEmptyString -Value $call.target_repo -Message "active allowed call target_repo for $($call.id)"
    Assert-NotEmptyString -Value $call.target_service -Message "active allowed call target_service for $($call.id)"
    Assert-NotEmptyString -Value $call.decision_reference -Message "active allowed call decision_reference for $($call.id)"
    if (@($call.allowed_surfaces).Count -eq 0) {
        throw "active allowed call '$($call.id)' must declare allowed_surfaces"
    }
    if (@($call.current_required_controls).Count -eq 0) {
        throw "active allowed call '$($call.id)' must declare current_required_controls"
    }
    if (@($call.target_required_controls).Count -eq 0) {
        throw "active allowed call '$($call.id)' must declare target_required_controls"
    }
}

$catalogAllowedCall = @($activeCalls | Where-Object { [string] $_.id -eq "gongzzang_api_to_platform_core_catalog_read" })
Assert-Equals -Actual $catalogAllowedCall.Count -Expected 1 -Message "allowed call matrix must define gongzzang catalog read"
Assert-Equals -Actual $catalogAllowedCall[0].traffic_policy_id -Expected "gongzzang_to_platform_core_catalog" -Message "catalog allowed call traffic policy mismatch"
Assert-Equals -Actual $catalogAllowedCall[0].service_auth_policy_id -Expected "gongzzang_api_to_platform_core_api" -Message "catalog allowed call service auth policy mismatch"
Assert-JsonArrayContains -Values @($catalogAllowedCall[0].current_required_controls) -Expected "no_direct_database" -Message "catalog allowed call current controls"

$platformCoreParcelLookup = Read-TextFile -RelativePath "services/api/src/platform_core_parcel_lookup.rs"
$platformCoreBuildingReader = Read-TextFile -RelativePath "services/api/src/building_reader.rs"
Assert-Contains `
    -Content $platformCoreParcelLookup `
    -Needle "catalog/v1/parcels/by-pnu" `
    -Message "catalog runtime surface parcel lookup source"
Assert-Contains `
    -Content $platformCoreBuildingReader `
    -Needle "catalog/v1/parcels/by-pnu" `
    -Message "catalog runtime surface building reader source"
foreach ($runtimeSurface in @(
    "/catalog/v1/parcels/by-pnu/:pnu",
    "/catalog/v1/parcels/by-pnu/:pnu/buildings"
)) {
    Assert-JsonArrayContains `
        -Values @($catalogAllowedCall[0].allowed_surfaces) `
        -Expected $runtimeSurface `
        -Message "catalog runtime surface allowed-call matrix"
}

$webhookAllowedCall = @($activeCalls | Where-Object { [string] $_.id -eq "platform_core_outbox_to_gongzzang_webhook" })
Assert-Equals -Actual $webhookAllowedCall.Count -Expected 1 -Message "allowed call matrix must define Platform Core webhook"
Assert-Equals -Actual $webhookAllowedCall[0].traffic_policy_id -Expected "platform_core_to_gongzzang_events" -Message "webhook allowed call traffic policy mismatch"
Assert-Equals -Actual $webhookAllowedCall[0].webhook_policy_id -Expected "platform_core_event_receiver" -Message "webhook allowed call webhook policy mismatch"
Assert-JsonArrayContains -Values @($webhookAllowedCall[0].target_required_controls) -Expected "event_replay_ledger" -Message "webhook target controls"

$dawneerPlannedCall = @($allowedCalls | Where-Object {
        [string] $_.id -eq "dawneer_to_platform_core_catalog_read" -and
        [string] $_.status -eq "planned" -and
        [string] $_.source_repo -eq "dawneer" -and
        [string] $_.target_repo -eq "platform-core"
    })
Assert-Equals -Actual $dawneerPlannedCall.Count -Expected 1 -Message "allowed call matrix must reserve Dawneer Platform Core read path"

foreach ($prohibited in @($callMatrix.prohibited_calls)) {
    Assert-Equals -Actual $prohibited.decision -Expected "deny" -Message "prohibited call decision mismatch for $($prohibited.id)"
}
foreach ($requiredProhibition in @(
    "gongzzang_to_platform_core_database",
    "dawneer_to_platform_core_database",
    "platform_core_to_gongzzang_listing_write"
)) {
    if (!(@($callMatrix.prohibited_calls | ForEach-Object { [string] $_.id }) -contains $requiredProhibition)) {
        throw "allowed call matrix missing prohibited call '$requiredProhibition'"
    }
}

$servicePolicies = @($trafficPolicy.service_call_policies)
$catalogCall = @($servicePolicies | Where-Object { [string] $_.id -eq "gongzzang_to_platform_core_catalog" })
Assert-Equals -Actual $catalogCall.Count -Expected 1 -Message "traffic policy must define gongzzang_to_platform_core_catalog"
Assert-Equals -Actual $catalogCall[0].current_auth_policy.env -Expected "PLATFORM_CORE_SERVICE_TOKEN" -Message "catalog call service auth env mismatch"
$eventCall = @($servicePolicies | Where-Object { [string] $_.id -eq "platform_core_to_gongzzang_events" })
Assert-Equals -Actual $eventCall.Count -Expected 1 -Message "traffic policy must define platform_core_to_gongzzang_events"
Assert-Equals -Actual $eventCall[0].current_auth_policy.env -Expected "PLATFORM_CORE_WEBHOOK_SECRET" -Message "event webhook auth env mismatch"

$outboundIdentityIds = @($serviceAuthPolicy.outbound_identities | ForEach-Object { [string] $_.id })
$inboundIdentityIds = @($serviceAuthPolicy.inbound_identities | ForEach-Object { [string] $_.id })
Assert-JsonArrayContains -Values $outboundIdentityIds -Expected "gongzzang_api_to_platform_core_api" -Message "service auth outbound identities"
Assert-JsonArrayContains -Values $inboundIdentityIds -Expected "platform_core_outbox_to_gongzzang_webhook" -Message "service auth inbound identities"

$catalogIdentity = @($serviceAuthPolicy.outbound_identities | Where-Object { [string] $_.id -eq "gongzzang_api_to_platform_core_api" })
Assert-Equals -Actual $catalogIdentity.Count -Expected 1 -Message "service auth policy must define Gongzzang Platform Core identity"
$workloadIdentityTokenFile = Get-JsonProperty -Object $catalogIdentity[0] -Name "workload_identity_token_file"
$workloadIdentityPreferred = Get-JsonProperty -Object $workloadIdentityTokenFile -Name "preferred_in_production"
$workloadIdentityRequiredEnv = Get-JsonProperty -Object $workloadIdentityTokenFile -Name "required_env"
$workloadIdentityRefreshBehavior = Get-JsonProperty -Object $workloadIdentityTokenFile -Name "refresh_behavior"
$workloadIdentityFallbackPolicy = Get-JsonProperty -Object $workloadIdentityTokenFile -Name "fallback_static_token_policy"
Assert-Equals -Actual $workloadIdentityPreferred -Expected $true -Message "workload identity token file production preference mismatch"
Assert-Equals -Actual $workloadIdentityRequiredEnv -Expected "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE" -Message "workload identity token file env mismatch"
Assert-Equals -Actual $workloadIdentityRefreshBehavior -Expected "read_before_each_request" -Message "workload identity token file refresh behavior mismatch"
Assert-Equals `
    -Actual $workloadIdentityFallbackPolicy `
    -Expected "allowed_only_with_metadata_until_cutover" `
    -Message "workload identity token file fallback policy mismatch"
$tokenMetadata = $catalogIdentity[0].token_metadata
Assert-Equals -Actual $tokenMetadata.required_in_production -Expected $true -Message "service auth token metadata production requirement mismatch"
Assert-Equals -Actual $tokenMetadata.required_scope -Expected "catalog:read" -Message "service auth token metadata scope mismatch"
if ([int] $tokenMetadata.max_ttl_days -gt 90) {
    throw "service auth token metadata max_ttl_days must be 90 or lower"
}
foreach ($requiredEnv in @(
    "PLATFORM_CORE_SERVICE_TOKEN_SCOPE",
    "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT",
    "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT",
    "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER"
)) {
    Assert-JsonArrayContains -Values @($tokenMetadata.required_env) -Expected $requiredEnv -Message "service auth token metadata required env"
}
foreach ($requiredHeader in @(
    "x-gongzzang-service-auth-policy-id",
    "x-gongzzang-service-auth-source",
    "x-gongzzang-service-auth-target",
    "x-gongzzang-allowed-call-id",
    "x-gongzzang-service-auth-scope",
    "x-gongzzang-service-auth-issued-at",
    "x-gongzzang-service-auth-expires-at",
    "x-gongzzang-service-auth-rotation-owner"
)) {
    Assert-JsonArrayContains -Values @($tokenMetadata.runtime_headers) -Expected $requiredHeader -Message "service auth token metadata runtime headers"
}
Assert-FileExists -RelativePath ([string] $tokenMetadata.rotation_runbook)
Assert-Equals -Actual $tokenMetadata.production_dev_token_policy -Expected "forbidden" -Message "service auth production dev token policy mismatch"
Assert-Equals -Actual $catalogIdentity[0].authorization_policy.default_decision -Expected "deny" -Message "service auth authorization default decision mismatch"
Assert-Equals -Actual $catalogIdentity[0].authorization_policy.allow_source -Expected $callMatrixPath -Message "service auth authorization allow source mismatch"
Assert-Equals -Actual $catalogIdentity[0].workload_identity_cutover.target -Expected "spiffe_spire_or_cloud_workload_identity" -Message "service auth workload identity target mismatch"

$rootEnv = Get-JsonProperty -Object $boundary -Name "root_env_example_contract"
$serviceAuthEnv = @((Get-JsonProperty -Object $rootEnv -Name "required_service_auth_env") | ForEach-Object { [string] $_ })
Assert-JsonArrayContains -Values $serviceAuthEnv -Expected "PLATFORM_CORE_SERVICE_TOKEN" -Message "boundary service auth env"
Assert-JsonArrayContains -Values $serviceAuthEnv -Expected "PLATFORM_CORE_WEBHOOK_SECRET" -Message "boundary webhook secret env"

$envExample = Read-TextFile -RelativePath ".env.example"
Assert-Contains -Content $envExample -Needle "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE=" -Message ".env.example workload identity token file placeholder"
Assert-Contains -Content $envExample -Needle "PLATFORM_CORE_SERVICE_TOKEN=" -Message ".env.example service token placeholder"
foreach ($needle in @(
    "PLATFORM_CORE_SERVICE_TOKEN_SCOPE=",
    "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT=",
    "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT=",
    "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER="
)) {
    Assert-Contains -Content $envExample -Needle $needle -Message ".env.example service token metadata placeholder"
}
Assert-Contains -Content $envExample -Needle "PLATFORM_CORE_WEBHOOK_SECRET=" -Message ".env.example webhook secret placeholder"

$apiAuth = Read-TextFile -RelativePath "crates/auth/src/platform_core_service.rs"
Assert-Contains -Content $apiAuth -Needle "bearer_auth" -Message "Platform Core outbound bearer auth"
Assert-Contains -Content $apiAuth -Needle "<redacted>" -Message "Platform Core auth debug redaction"
foreach ($needle in @(
    "new_for_environment",
    "new_from_workload_identity_token_file",
    "WorkloadIdentityTokenFile",
    "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE",
    "read_before_each_request",
    "MAX_TOKEN_TTL_DAYS",
    "MetadataIssuedInFuture",
    "MetadataTtlTooLong",
    "x-gongzzang-service-auth-policy-id",
    "x-gongzzang-service-auth-source",
    "x-gongzzang-service-auth-target",
    "x-gongzzang-allowed-call-id",
    "gongzzang-api",
    "platform-core-api",
    "gongzzang_api_to_platform_core_catalog_read",
    "x-gongzzang-service-auth-scope",
    "x-gongzzang-service-auth-issued-at",
    "x-gongzzang-service-auth-expires-at",
    "x-gongzzang-service-auth-rotation-owner"
)) {
    Assert-Contains -Content $apiAuth -Needle $needle -Message "Platform Core default-deny identity runtime"
}

$startup = Read-TextFile -RelativePath "services/api/src/startup.rs"
Assert-Contains -Content $startup -Needle "PLATFORM_CORE_SERVICE_TOKEN must be set" -Message "production Platform Core token fail-fast"
Assert-Contains -Content $startup -Needle "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE" -Message "startup workload identity token file env"
foreach ($needle in @(
    "PlatformCoreServiceAuthMetadataConfig",
    "PLATFORM_CORE_SERVICE_TOKEN_SCOPE",
    "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT",
    "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT",
    "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER"
)) {
    Assert-Contains -Content $startup -Needle $needle -Message "startup Platform Core service auth metadata"
}

$rotationRunbook = Read-TextFile -RelativePath ([string] $tokenMetadata.rotation_runbook)
foreach ($needle in @(
    "catalog:read",
    "90 days",
    "SPIFFE/SPIRE",
    "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE",
    "read before each Platform Core request",
    "default-deny",
    "x-gongzzang-service-auth-policy-id",
    "gongzzang_api_to_platform_core_catalog_read"
)) {
    Assert-Contains -Content $rotationRunbook -Needle $needle -Message "Platform Core service token rotation runbook"
}

foreach ($relativeProductionPath in @(
    ".env.production",
    ".env.production.example",
    "infrastructure/docker/.env.production",
    "infrastructure/docker/.env.production.example"
)) {
    $path = Resolve-RepoPath -RelativePath $relativeProductionPath
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $content = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        if ($content.Contains("dev-platform-core-service-token")) {
            throw "production config '$relativeProductionPath' must not contain dev Platform Core service token"
        }
    }
}

$webEnv = Read-TextFile -RelativePath "apps/web/lib/env.ts"
Assert-Contains -Content $webEnv -Needle "PLATFORM_CORE_WEBHOOK_SECRET" -Message "web env webhook secret schema"

$webhookRoute = Read-TextFile -RelativePath "apps/web/app/platform-core/events/route.ts"
foreach ($needle in @(
    "createHmac",
    "timingSafeEqual",
    "x-platform-core-signature",
    "x-platform-core-timestamp",
    "WEBHOOK_MAX_SKEW_SECONDS",
    "x-platform-core-event-id",
    "x-platform-core-event-type",
    "x-platform-core-outbox-scope",
    "reservePlatformCoreEvent",
    "recordPlatformCoreEventAccepted",
    "recordPlatformCoreEventDeadLetter"
)) {
    Assert-Contains -Content $webhookRoute -Needle $needle -Message "webhook runtime control"
}

$acceptedWebhookEvents = @($webhookPolicy.receiver.accepted_events | ForEach-Object { [string] $_ })
foreach ($eventType in @(
    "catalog.industrial_complex.gold_pointer.published.v1",
    "catalog.parcel_marker_anchor.snapshot.published.v1"
)) {
    Assert-JsonArrayContains -Values $acceptedWebhookEvents -Expected $eventType -Message "webhook accepted events"
    Assert-Contains -Content $webhookRoute -Needle $eventType -Message "webhook accepted event runtime schema"
}

$webhookRuntimeSafety = $webhookPolicy.receiver.runtime_safety
Assert-Equals -Actual $webhookRuntimeSafety.idempotency_key -Expected "event_id" -Message "webhook runtime idempotency key mismatch"
Assert-Equals -Actual $webhookRuntimeSafety.replay_ledger -Expected "apps/web/lib/platform-core/event-inbox.ts" -Message "webhook runtime replay ledger mismatch"
Assert-Equals -Actual $webhookRuntimeSafety.duplicate_policy -Expected "ack_without_side_effects" -Message "webhook duplicate policy mismatch"
Assert-Equals -Actual $webhookRuntimeSafety.dead_letter_queue -Expected "redis_event_inbox_dead_letter_status" -Message "webhook dead letter queue mismatch"
Assert-Equals -Actual $webhookRuntimeSafety.schema_compatibility -Expected "accepted_events_must_have_zod_handler" -Message "webhook schema compatibility policy mismatch"

$webhookInbox = Read-TextFile -RelativePath "apps/web/lib/platform-core/event-inbox.ts"
foreach ($needle in @(
    "PlatformCoreEventInboxRecord",
    "reservePlatformCoreEvent",
    "recordPlatformCoreEventAccepted",
    "recordPlatformCoreEventDeadLetter",
    "dead_letter",
    '"NX"'
)) {
    Assert-Contains -Content $webhookInbox -Needle $needle -Message "webhook event inbox control"
}

$exceptionContract = $exceptionPolicy.exception_contract
Assert-Equals -Actual $exceptionPolicy.default_exception_decision -Expected "deny" -Message "exception policy default decision mismatch"
foreach ($requiredField in @(
    "id",
    "owner",
    "reason",
    "risk_level",
    "expires_at",
    "approval_reference",
    "compensating_controls"
)) {
    Assert-JsonArrayContains -Values @($exceptionContract.required_fields) -Expected $requiredField -Message "exception policy required fields"
}
if ([int] $exceptionContract.max_ttl_days -gt 90) {
    throw "exception policy max_ttl_days must be 90 or lower"
}
Assert-JsonArrayContains -Values @($exceptionContract.forbidden_risk_levels) -Expected "critical" -Message "exception policy forbidden risk levels"

foreach ($exception in @($exceptionPolicy.active_exceptions)) {
    Assert-NotEmptyString -Value $exception.id -Message "active exception id"
    Assert-NotEmptyString -Value $exception.owner -Message "active exception owner for $($exception.id)"
    Assert-NotEmptyString -Value $exception.reason -Message "active exception reason for $($exception.id)"
    Assert-NotEmptyString -Value $exception.risk_level -Message "active exception risk_level for $($exception.id)"
    Assert-NotEmptyString -Value $exception.expires_at -Message "active exception expires_at for $($exception.id)"
    Assert-NotEmptyString -Value $exception.approval_reference -Message "active exception approval_reference for $($exception.id)"
    if (@($exception.compensating_controls).Count -eq 0) {
        throw "active exception '$($exception.id)' must declare compensating_controls"
    }
    if (@($exceptionContract.forbidden_risk_levels | ForEach-Object { [string] $_ }) -contains ([string] $exception.risk_level)) {
        throw "active exception '$($exception.id)' uses forbidden risk_level '$($exception.risk_level)'"
    }
    Assert-DateNotExpired -Value ([string] $exception.expires_at) -Message "active exception '$($exception.id)'"
}

foreach ($forbiddenType in @(
    "direct_platform_core_database_access",
    "anonymous_platform_core_service_call_in_production",
    "unsigned_platform_core_webhook_in_production",
    "unattested_production_artifact_deploy"
)) {
    Assert-JsonArrayContains -Values @($exceptionPolicy.forbidden_exception_types) -Expected $forbiddenType -Message "exception policy forbidden exception types"
}

$npmSupplyChainPolicy = $supplyChainPolicy.npm
Assert-Equals `
    -Actual $npmSupplyChainPolicy.audit_bazel_target `
    -Expected "//tools/bazel:ci_node_audit_transition" `
    -Message "supply chain npm audit Bazel target mismatch"

$rustSupplyChainPolicy = $supplyChainPolicy.rust
Assert-Equals -Actual $rustSupplyChainPolicy.sca -Expected "cargo-deny" -Message "supply chain Rust SCA tool mismatch"
Assert-Equals -Actual $rustSupplyChainPolicy.config -Expected "deny.toml" -Message "supply chain Rust SCA config mismatch"
Assert-Equals `
    -Actual $rustSupplyChainPolicy.bazel_target `
    -Expected "//tools/bazel:ci_cargo_deny_transition" `
    -Message "supply chain Rust SCA Bazel target mismatch"

$releaseArtifacts = @($supplyChainPolicy.release_artifacts)
Assert-Equals -Actual $releaseArtifacts.Count -Expected 2 -Message "supply chain release artifact count mismatch"
Assert-Unique -Values ($releaseArtifacts | ForEach-Object { $_.id }) -Message "supply chain release artifact ids must be unique"
foreach ($artifact in $releaseArtifacts) {
    Assert-NotEmptyString -Value $artifact.ecosystem -Message "supply chain release artifact ecosystem for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.bazel_target -Message "supply chain release artifact Bazel target for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.subject_path -Message "supply chain release artifact subject_path for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.sbom_source_path -Message "supply chain release artifact sbom_source_path for $($artifact.id)"
}
foreach ($requiredEcosystem in @("node", "rust")) {
    Assert-JsonArrayContains -Values @($releaseArtifacts | ForEach-Object { [string] $_.ecosystem }) -Expected $requiredEcosystem -Message "supply chain release artifact ecosystems"
}

$sbomPolicy = $supplyChainPolicy.sbom
Assert-Equals -Actual $sbomPolicy.required -Expected $true -Message "supply chain SBOM requirement mismatch"
Assert-Equals -Actual $sbomPolicy.format -Expected "cyclonedx-json" -Message "supply chain SBOM format mismatch"
Assert-Equals -Actual $sbomPolicy.generator.tool -Expected "syft" -Message "supply chain SBOM generator mismatch"
Assert-Equals -Actual $sbomPolicy.generator.ci_action -Expected "anchore/sbom-action" -Message "supply chain SBOM action mismatch"
Assert-Equals -Actual $sbomPolicy.generator.pinned_ref -Expected "e22c389904149dbc22b58101806040fa8d37a610" -Message "supply chain SBOM action pin mismatch"
$sbomArtifacts = @($sbomPolicy.artifacts)
Assert-Equals -Actual $sbomArtifacts.Count -Expected 2 -Message "supply chain SBOM artifact count mismatch"
Assert-Unique -Values ($sbomArtifacts | ForEach-Object { $_.id }) -Message "supply chain SBOM artifact ids must be unique"
foreach ($artifact in $sbomArtifacts) {
    Assert-NotEmptyString -Value $artifact.ecosystem -Message "supply chain SBOM ecosystem for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.source_path -Message "supply chain SBOM source_path for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.output_file -Message "supply chain SBOM output_file for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.subject_path -Message "supply chain SBOM subject_path for $($artifact.id)"
}
foreach ($requiredEcosystem in @("node", "rust")) {
    Assert-JsonArrayContains -Values @($sbomArtifacts | ForEach-Object { [string] $_.ecosystem }) -Expected $requiredEcosystem -Message "supply chain SBOM ecosystems"
}

$provenancePolicy = $supplyChainPolicy.provenance
Assert-Equals -Actual $provenancePolicy.required -Expected $true -Message "supply chain provenance requirement mismatch"
Assert-Equals -Actual $provenancePolicy.provider -Expected "github_artifact_attestations" -Message "supply chain provenance provider mismatch"
Assert-Equals -Actual $provenancePolicy.predicate -Expected "slsa_build_provenance" -Message "supply chain provenance predicate mismatch"
Assert-Equals -Actual $provenancePolicy.ci_action -Expected "actions/attest" -Message "supply chain provenance action mismatch"
Assert-Equals -Actual $provenancePolicy.pinned_ref -Expected "281a49d4cbb0a72c9575a50d18f6deb515a11deb" -Message "supply chain provenance action pin mismatch"
foreach ($permission in @("contents: read", "id-token: write", "attestations: write", "artifact-metadata: write")) {
    Assert-JsonArrayContains -Values @($provenancePolicy.required_permissions) -Expected $permission -Message "supply chain provenance permissions"
}
foreach ($subjectPath in @($releaseArtifacts | ForEach-Object { [string] $_.subject_path })) {
    Assert-JsonArrayContains -Values @($provenancePolicy.production_subjects) -Expected $subjectPath -Message "supply chain provenance production subjects"
}

$deployGate = $supplyChainPolicy.deploy_gate

if ($IncludeProductionPromotion) {
    Assert-Equals -Actual $deployGate.required -Expected $true -Message "supply chain deploy gate requirement mismatch"
    Assert-Equals -Actual $deployGate.approved_workflow -Expected ".github/workflows/ci.yml" -Message "supply chain deploy gate workflow mismatch"
    Assert-Equals -Actual $deployGate.approved_job -Expected "supply-chain-provenance" -Message "supply chain deploy gate job mismatch"
    Assert-Equals -Actual $deployGate.candidate_policy -Expected "production_candidates_must_be_built_on_main_by_approved_workflow" -Message "supply chain deploy gate candidate policy mismatch"
    Assert-Contains -Content ([string] $deployGate.verification_command) -Needle "gh attestation verify" -Message "supply chain deploy gate verification command"
    Assert-FileExists -RelativePath ([string] $deployGate.verification_script)
    Assert-FileExists -RelativePath ([string] $deployGate.runbook)
    foreach ($forbiddenDeploy in @(
        "deploy_without_attestation",
        "deploy_from_unapproved_workflow",
        "deploy_from_unverified_subject_digest",
        "mutable_image_tag_without_digest"
    )) {
        Assert-JsonArrayContains -Values @($deployGate.forbidden) -Expected $forbiddenDeploy -Message "supply chain deploy gate forbidden list"
    }

    $deployGateScript = Read-TextFile -RelativePath ([string] $deployGate.verification_script)
    foreach ($needle in @(
        "gh",
        "attestation",
        "verify",
        "RequiredWorkflow",
        "RequiredRef",
        "--predicate-type",
        "production-deploy-candidate-ok"
    )) {
        Assert-Contains -Content $deployGateScript -Needle $needle -Message "supply chain deploy gate verification script"
    }

    Assert-Equals -Actual $deployGate.admission_workflow -Expected ".github/workflows/production-deploy-admission.yml" -Message "supply chain deploy gate admission workflow mismatch"
    Assert-Equals -Actual $deployGate.admission_job -Expected "verify-production-deploy-candidates" -Message "supply chain deploy gate admission job mismatch"
    Assert-Equals -Actual $deployGate.admission_environment -Expected "production" -Message "supply chain deploy gate admission environment mismatch"
    Assert-Equals -Actual $deployGate.download_artifact_action.ci_action -Expected "actions/download-artifact" -Message "supply chain deploy gate download action mismatch"
    Assert-Equals -Actual $deployGate.download_artifact_action.pinned_ref -Expected "d3f86a106a0bac45b974a628896c90dbdf5c8093" -Message "supply chain deploy gate download action pin mismatch"
    Assert-FileExists -RelativePath ([string] $deployGate.admission_workflow)

    $loadTestCapacityAdmission = Get-JsonProperty -Object $deployGate -Name "load_test_capacity_admission"
    if ($null -eq $loadTestCapacityAdmission) {
        throw "load-test capacity admission missing"
    }
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.required `
        -Expected $true `
        -Message "load-test capacity admission requirement mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.verification_script `
        -Expected "scripts/ci/verify-load-test-capacity-evidence.ps1" `
        -Message "load-test capacity admission verification script mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.evidence_artifact_name `
        -Expected "load-test-capacity-evidence" `
        -Message "load-test capacity admission artifact name mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.workflow_input_run_id `
        -Expected "load-evidence-run-id" `
        -Message "load-test capacity admission run-id input mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.workflow_input_artifact_name `
        -Expected "load-evidence-artifact-name" `
        -Message "load-test capacity admission artifact input mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.required_classification `
        -Expected "healthy" `
        -Message "load-test capacity admission classification mismatch"
    Assert-FileExists -RelativePath ([string] $loadTestCapacityAdmission.verification_script)
    foreach ($requiredScenario in @("api-read-mix", "map-marker-mix", "platform-core-events")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.required_scenarios) `
            -Expected $requiredScenario `
            -Message "load-test capacity admission required scenarios"
    }
    foreach ($requiredEnvironment in @("perf", "staging")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.required_environments) `
            -Expected $requiredEnvironment `
            -Message "load-test capacity admission required environments"
    }
    $targetHostByEnvironment = Get-JsonProperty -Object $loadTestCapacityAdmission -Name "target_host_by_environment"
    Assert-Equals `
        -Actual ([string] (Get-JsonProperty -Object $targetHostByEnvironment -Name "perf")) `
        -Expected "perf.gongzzang.internal" `
        -Message "load-test capacity admission perf target host mismatch"
    Assert-Equals `
        -Actual ([string] (Get-JsonProperty -Object $targetHostByEnvironment -Name "staging")) `
        -Expected "staging.gongzzang.internal" `
        -Message "load-test capacity admission staging target host mismatch"
    foreach ($forbiddenEnvironment in @("local", "ci")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.forbidden_environments) `
            -Expected $forbiddenEnvironment `
            -Message "load-test capacity admission forbidden environments"
    }
    foreach ($forbiddenLoadEvidenceDeploy in @(
        "production_deploy_without_perf_or_staging_load_evidence",
        "local_or_ci_smoke_used_as_launch_capacity_evidence",
        "capacity_evidence_from_production_target"
    )) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.forbidden) `
            -Expected $forbiddenLoadEvidenceDeploy `
            -Message "load-test capacity admission forbidden list"
    }

    $edgeAdmission = Get-JsonProperty -Object $deployGate -Name "edge_admission"
    if ($null -eq $edgeAdmission) {
        throw "production edge admission missing"
    }
    Assert-Equals -Actual $edgeAdmission.required -Expected $true -Message "production edge admission requirement mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.policy_source `
        -Expected "docs/architecture/traffic-auth-policy-registry.v1.json" `
        -Message "production edge admission policy source mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.generated_waf_manifest `
        -Expected "infrastructure/security/aws-wafv2-edge-policy.generated.json" `
        -Message "production edge admission WAF manifest mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.pulumi_project `
        -Expected "infrastructure/Pulumi.yaml" `
        -Message "production edge admission Pulumi project mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.pulumi_program `
        -Expected "infrastructure/index.ts" `
        -Message "production edge admission Pulumi program mismatch"
    Assert-FileExists -RelativePath ([string] $edgeAdmission.generated_waf_manifest)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.pulumi_project)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.pulumi_program)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.verification_script)

    $regionalAttachment = Get-JsonProperty -Object $edgeAdmission -Name "regional_attachment"
    Assert-Equals -Actual $regionalAttachment.supported -Expected $true -Message "production edge regional attachment support mismatch"
    Assert-Equals `
        -Actual $regionalAttachment.config_key `
        -Expected "wafRegionalResourceArn" `
        -Message "production edge regional attachment config mismatch"
    Assert-Equals `
        -Actual $regionalAttachment.required_env `
        -Expected "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN" `
        -Message "production edge regional attachment env mismatch"
    $pulumiAssociationPreview = Get-JsonProperty -Object $regionalAttachment -Name "pulumi_association_preview"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.required `
        -Expected $true `
        -Message "production edge Pulumi association preview requirement mismatch"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.preview_script `
        -Expected "scripts/ci/check-pulumi-local-preview.ps1" `
        -Message "production edge Pulumi association preview script mismatch"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.evidence `
        -Expected "regional_association=planned" `
        -Message "production edge Pulumi association preview evidence mismatch"
    Assert-FileExists -RelativePath ([string] $pulumiAssociationPreview.preview_script)
    $cloudfrontAttachment = Get-JsonProperty -Object $edgeAdmission -Name "cloudfront_attachment"
    Assert-Equals `
        -Actual $cloudfrontAttachment.supported `
        -Expected $false `
        -Message "production edge CloudFront attachment support mismatch"
    Assert-Equals `
        -Actual $cloudfrontAttachment.required_before_production `
        -Expected $true `
        -Message "production edge CloudFront pre-production requirement mismatch"
    foreach ($forbiddenEdgeDeploy in @(
        "production_deploy_without_waf_attachment",
        "edge_policy_not_from_traffic_auth_registry",
        "manual_waf_console_change"
    )) {
        Assert-JsonArrayContains `
            -Values @($edgeAdmission.forbidden) `
            -Expected $forbiddenEdgeDeploy `
            -Message "production edge admission forbidden list"
    }

    $edgeAdmissionScript = Read-TextFile -RelativePath ([string] $edgeAdmission.verification_script)
    foreach ($needle in @(
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "wafRegionalResourceArn",
        "aws-wafv2-edge-policy.generated.json",
        "RequirePulumiAssociationPreview",
        "check-pulumi-local-preview.ps1",
        "regional_association=planned",
        "production-edge-admission-ok",
        "must be an AWS ARN"
    )) {
        Assert-Contains -Content $edgeAdmissionScript -Needle $needle -Message "production edge admission verification script"
    }

    $loadTestCapacityAdmissionScript = Read-TextFile -RelativePath ([string] $loadTestCapacityAdmission.verification_script)
    foreach ($needle in @(
        "EvidenceRoot",
        "run.json",
        "spec.json",
        "k6-summary.json",
        "Classification: healthy",
        "capacity evidence environment must be perf or staging",
        "profile must be baseline, stress, spike, or soak",
        "production targets are not valid load-test capacity evidence",
        "target host must match capacity evidence environment",
        "missing required load-test capacity scenario",
        "RequiredScenarios",
        "load-test-capacity-evidence-ok"
    )) {
        Assert-Contains -Content $loadTestCapacityAdmissionScript -Needle $needle -Message "load-test capacity admission verification script"
    }

    $deployGateWorkflow = Read-TextFile -RelativePath ([string] $deployGate.admission_workflow)
    foreach ($needle in @(
        "workflow_call:",
        "workflow_dispatch:",
        "verify-production-deploy-candidates:",
        "environment: production",
        "actions: read",
        "attestations: read",
        "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093",
        "pnpm install --frozen-lockfile",
        "verify-production-deploy-candidate.ps1",
        "check-production-edge-admission.ps1",
        "verify-load-test-capacity-evidence.ps1",
        "load-evidence-run-id",
        "load-evidence-artifact-name",
        "Download load-test capacity evidence",
        "Verify load-test capacity evidence",
        "target/admission/load-test-capacity-evidence",
        "-RequirePulumiAssociationPreview",
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "-PredicateType https://cyclonedx.org/bom",
        "run-id"
    )) {
        Assert-Contains -Content $deployGateWorkflow -Needle $needle -Message "supply chain deploy admission workflow"
    }
    foreach ($releaseArtifact in $releaseArtifacts) {
        Assert-Contains -Content $deployGateWorkflow -Needle ([string] $releaseArtifact.subject_path) -Message "supply chain deploy admission subject path"
    }
}

$telemetryPolicy = $operationsPolicy.telemetry
foreach ($attribute in @(
    "service.name",
    "peer.service",
    "http.request.method",
    "url.path",
    "platform_integration.call_id",
    "platform_integration.policy_id",
    "platform_integration.direction",
    "platform_integration.decision",
    "platform_core.event_id",
    "platform_core.event_type",
    "correlation_id"
)) {
    Assert-JsonArrayContains -Values @($telemetryPolicy.required_span_attributes) -Expected $attribute -Message "operations telemetry required span attributes"
}
foreach ($forbiddenAttribute in @(
    "authorization",
    "cookie",
    "set-cookie",
    "platform_core_service_token",
    "platform_core_webhook_secret"
)) {
    Assert-JsonArrayContains -Values @($telemetryPolicy.forbidden_attributes) -Expected $forbiddenAttribute -Message "operations telemetry forbidden attributes"
}

$sloPolicies = @($operationsPolicy.slos)
Assert-Equals -Actual $sloPolicies.Count -Expected 2 -Message "operations SLO count mismatch"
Assert-Unique -Values ($sloPolicies | ForEach-Object { $_.id }) -Message "operations SLO ids must be unique"
foreach ($slo in $sloPolicies) {
    Assert-NotEmptyString -Value $slo.alert_policy -Message "operations SLO alert_policy for $($slo.id)"
    Assert-FileExists -RelativePath ([string] $slo.runbook)
    if ([double] $slo.availability_percent -lt 99.9) {
        throw "operations SLO '$($slo.id)' availability_percent must be at least 99.9"
    }
    if ([int] $slo.p95_latency_ms -gt 300) {
        throw "operations SLO '$($slo.id)' p95_latency_ms must be 300ms or lower"
    }
    if ([int] $slo.p99_latency_ms -gt 1000) {
        throw "operations SLO '$($slo.id)' p99_latency_ms must be 1000ms or lower"
    }
}
foreach ($requiredSloId in @(
    "gongzzang_api_to_platform_core_catalog_read",
    "platform_core_outbox_to_gongzzang_webhook"
)) {
    Assert-JsonArrayContains -Values @($sloPolicies | ForEach-Object { [string] $_.id }) -Expected $requiredSloId -Message "operations SLO ids"
}

$alerts = @($operationsPolicy.alerts)
Assert-Unique -Values ($alerts | ForEach-Object { $_.id }) -Message "operations alert ids must be unique"
foreach ($alert in $alerts) {
    Assert-NotEmptyString -Value $alert.signal -Message "operations alert signal for $($alert.id)"
    Assert-NotEmptyString -Value $alert.severity -Message "operations alert severity for $($alert.id)"
    Assert-FileExists -RelativePath ([string] $alert.runbook)
}
foreach ($requiredAlertId in @(
    "platform_core_catalog_read_slo_burn",
    "platform_core_catalog_circuit_open",
    "platform_core_webhook_dead_letter_or_latency",
    "platform_core_webhook_replay_surge"
)) {
    Assert-JsonArrayContains -Values @($alerts | ForEach-Object { [string] $_.id }) -Expected $requiredAlertId -Message "operations alert ids"
}

$loadFaultTests = @($operationsPolicy.load_fault_tests)
Assert-Equals -Actual $loadFaultTests.Count -Expected 5 -Message "operations load/fault test count mismatch"
Assert-Unique -Values ($loadFaultTests | ForEach-Object { $_.id }) -Message "operations load/fault test ids must be unique"
foreach ($test in $loadFaultTests) {
    Assert-Equals -Actual $test.required -Expected $true -Message "operations load/fault test must be required for $($test.id)"
    Assert-FileExists -RelativePath ([string] $test.test_file)
    $testContent = Read-TextFile -RelativePath ([string] $test.test_file)
    Assert-Contains -Content $testContent -Needle ([string] $test.evidence) -Message "operations load/fault test evidence for $($test.id)"
}

$package = Read-JsonFile -RelativePath "package.json"
$overrides = $package.pnpm.overrides
foreach ($name in @("brace-expansion", "postcss", "vite")) {
    $expected = [string] (Get-JsonProperty -Object $supplyChainPolicy.npm.required_overrides -Name $name)
    $actual = [string] (Get-JsonProperty -Object $overrides -Name $name)
    Assert-Equals -Actual $actual -Expected $expected -Message "pnpm override mismatch for $name"
}

Assert-FileExists -RelativePath "deny.toml"
Assert-FileExists -RelativePath ".gitleaks.toml"

$ci = Read-TextFile -RelativePath ".github/workflows/ci.yml"
$requiredCiJobsOrSteps = @(Get-JsonProperty -Object $index -Name "required_ci_jobs_or_steps" | ForEach-Object { [string] $_ })
$productionPromotionCiJobsOrSteps = @(
    "check-production-edge-admission.ps1",
    "check-load-test-assets.ps1",
    "verify-load-test-capacity-evidence.tests.ps1"
)
$requiredCiJobsOrSteps = @($requiredCiJobsOrSteps | Where-Object {
        $productionPromotionCiJobsOrSteps -notcontains ([string] $_)
    })
foreach ($requiredCiJobOrStep in $requiredCiJobsOrSteps) {
    Assert-Contains `
        -Content $ci `
        -Needle $requiredCiJobOrStep `
        -Message "CI required jobs or steps"
}
foreach ($needle in @(
    "//tools/bazel:ci_node_audit_transition",
    "//tools/bazel:ci_cargo_deny_transition",
    "gitleaks-action",
    "check-platform-integration-policy.ps1",
    "check-lakehouse-registry-integration.ps1",
    "check-lakehouse-registry-integration.tests.ps1",
    "supply-chain-provenance:",
    "id-token: write",
    "attestations: write",
    "artifact-metadata: write",
    "anchore/sbom-action@e22c389904149dbc22b58101806040fa8d37a610",
    "actions/attest@281a49d4cbb0a72c9575a50d18f6deb515a11deb",
    "format: cyclonedx-json",
    "output-file: target/supply-chain/gongzzang-node-workspace-sbom.cdx.json",
    "output-file: target/supply-chain/gongzzang-rust-workspace-sbom.cdx.json",
    "sbom-path: target/supply-chain/gongzzang-node-workspace-sbom.cdx.json",
    "sbom-path: target/supply-chain/gongzzang-rust-workspace-sbom.cdx.json"
)) {
    Assert-Contains -Content $ci -Needle $needle -Message "CI platform integration gate"
}
foreach ($releaseArtifact in $releaseArtifacts) {
    Assert-Contains -Content $ci -Needle ([string] $releaseArtifact.bazel_target) -Message "CI release artifact Bazel target"
    Assert-Contains -Content $ci -Needle ([string] $releaseArtifact.subject_path) -Message "CI release artifact subject path"
    Assert-Contains -Content $ci -Needle ([string] $releaseArtifact.sbom_source_path) -Message "CI release artifact SBOM source path"
}

if ($IncludeProductionPromotion) {
    $deployGateRunbook = Read-TextFile -RelativePath ([string] $deployGate.runbook)
    foreach ($needle in @(
        "SBOM",
        "SLSA",
        "GitHub Artifact Attestations",
        "gh attestation verify",
        "approved workflow",
        "subject digest"
    )) {
        Assert-Contains -Content $deployGateRunbook -Needle $needle -Message "supply chain deploy gate runbook"
    }
    foreach ($needle in @(
        "load-test capacity evidence",
        "load-evidence-run-id",
        "verify-load-test-capacity-evidence.ps1",
        "production edge admission",
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "wafRegionalResourceArn",
        "regional_association=planned",
        "WebAclAssociation"
    )) {
        Assert-Contains -Content $deployGateRunbook -Needle $needle -Message "supply chain production promotion runbook"
    }
}

$lefthook = Read-TextFile -RelativePath "lefthook.yml"
Assert-Contains -Content $lefthook -Needle "check-platform-integration-policy.ps1" -Message "lefthook platform integration gate"
Assert-Contains -Content $lefthook -Needle "check-lakehouse-registry-integration.ps1" -Message "lefthook lakehouse registry integration gate"
Assert-Contains -Content $lefthook -Needle "check-migration-version-prefixes.ps1" -Message "lefthook migration prefix gate"
Assert-Contains -Content $lefthook -Needle "check-platform-core-anchor-inbox-db-approval.ps1" -Message "lefthook anchor inbox DB approval gate"
Assert-Contains -Content $lefthook -Needle "gitleaks protect --staged --redact -v" -Message "lefthook gitleaks gate"

$ssotMatrix = Read-TextFile -RelativePath "docs/ssot-matrix.md"
Assert-Contains -Content $ssotMatrix -Needle "Platform Integration Policy" -Message "SSOT matrix platform integration row"

Write-Host "platform-integration-policy-ok components=$($components.Count) route_surfaces=$(@($routePolicy.surfaces).Count)"
