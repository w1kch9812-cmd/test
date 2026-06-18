    $migrationPrefixLefthook = if ($OmitMigrationPrefixLefthook) { "" } else { "check-migration-version-prefixes.ps1" }
    Write-File -Root $Root -RelativePath "lefthook.yml" -Content @"
check-platform-integration-policy.ps1
check-lakehouse-registry-integration.ps1
$migrationPrefixLefthook
check-platform-core-anchor-inbox-db-approval.ps1
gitleaks protect --staged --redact -v
"@
    $defaultDenyIdentityRuntime = if ($OmitDefaultDenyIdentityRuntime) {
        ""
    } else {
        @'
x-gongzzang-service-auth-source
x-gongzzang-service-auth-target
x-gongzzang-allowed-call-id
gongzzang-api
platform-core-api
gongzzang_api_to_platform_core_catalog_read
'@
    }
    $workloadIdentityTokenFileRuntime = if ($OmitWorkloadIdentityTokenFileSupport) {
        ""
    } else {
        @'
new_from_workload_identity_token_file
WorkloadIdentityTokenFile
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
read_before_each_request
'@
    }
    Write-File -Root $Root -RelativePath "crates\auth\src\platform_core_service.rs" -Content @"
pub fn apply(request: reqwest::RequestBuilder) { request.bearer_auth("token"); }
pub fn fmt() { "<redacted>"; }
new_for_environment
$workloadIdentityTokenFileRuntime
MAX_TOKEN_TTL_DAYS
MetadataIssuedInFuture
MetadataTtlTooLong
x-gongzzang-service-auth-policy-id
$defaultDenyIdentityRuntime
x-gongzzang-service-auth-scope
x-gongzzang-service-auth-issued-at
x-gongzzang-service-auth-expires-at
x-gongzzang-service-auth-rotation-owner
"@
    Write-File -Root $Root -RelativePath "services\api\src\startup.rs" -Content @'
PLATFORM_CORE_SERVICE_TOKEN must be set
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
PlatformCoreServiceAuthMetadataConfig
PLATFORM_CORE_SERVICE_TOKEN_SCOPE
PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT
PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT
PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER
'@
    Write-File -Root $Root -RelativePath "services\api\src\platform_core_parcel_lookup.rs" -Content @'
catalog/v1/parcels/by-pnu/
'@
    Write-File -Root $Root -RelativePath "services\api\src\building_reader.rs" -Content @'
catalog/v1/parcels/by-pnu/{}/buildings
'@
    Write-File -Root $Root -RelativePath "docs\runbooks\platform-core-service-token-rotation.md" -Content @'
catalog:read
90 days
SPIFFE/SPIRE
PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE
read before each Platform Core request
default-deny
x-gongzzang-service-auth-policy-id
gongzzang_api_to_platform_core_catalog_read
'@
    if (!$OmitDeployGateRunbook) {
        Write-File -Root $Root -RelativePath "docs\runbooks\supply-chain-provenance-and-deploy-gate.md" -Content @'
SBOM
SLSA
GitHub Artifact Attestations
gh attestation verify
approved workflow
subject digest
load-test capacity evidence
load-evidence-run-id
verify-load-test-capacity-evidence.ps1
production edge admission
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
wafRegionalResourceArn
regional_association=planned
WebAclAssociation
'@
    }
    Write-File -Root $Root -RelativePath "docs\runbooks\platform-core-integration-operations.md" -Content @'
platform_core_catalog_read_slo_burn
platform_core_catalog_circuit_open
platform_core_webhook_dead_letter_or_latency
platform_core_webhook_replay_surge
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\env.ts" -Content @'
PLATFORM_CORE_WEBHOOK_SECRET
'@
    Write-File -Root $Root -RelativePath "apps\web\app\platform-core\events\route.ts" -Content @'
createHmac
timingSafeEqual
x-platform-core-signature
x-platform-core-timestamp
WEBHOOK_MAX_SKEW_SECONDS
x-platform-core-event-id
x-platform-core-event-type
x-platform-core-outbox-scope
reservePlatformCoreEvent
recordPlatformCoreEventAccepted
recordPlatformCoreEventDeadLetter
catalog.industrial_complex.gold_pointer.published.v1
catalog.parcel_marker_anchor.snapshot.published.v1
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\platform-core\event-inbox.ts" -Content @'
PlatformCoreEventInboxRecord
reservePlatformCoreEvent
recordPlatformCoreEventAccepted
recordPlatformCoreEventDeadLetter
dead_letter
"NX"
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-events.test.ts" -Content @'
acknowledges duplicate platform-core event bursts without repeated side effects
dead_letter
'@
    Write-File -Root $Root -RelativePath "crates\db\tests\platform_core_anchor_import_integration.rs" -Content @'
processing_inbox_event_can_be_reclaimed_for_retry_after_worker_exit
'@
    Write-File -Root $Root -RelativePath "crates\circuit-breaker\src\execute.rs" -Content @'
execute_timeout_records_failure
execute_returns_open_when_breaker_open
'@
    Write-File -Root $Root -RelativePath "docs\ssot-matrix.md" -Content "Platform Integration Policy"
