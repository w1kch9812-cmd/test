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
