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
