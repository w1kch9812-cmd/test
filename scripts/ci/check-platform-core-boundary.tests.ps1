Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-boundary.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-boundary-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

. (Join-Path $PSScriptRoot "platform-core-boundary.tests.helpers.ps1")

$FixtureRoot = Join-Path $PSScriptRoot "platform-core-boundary.tests"
. (Join-Path $FixtureRoot "fixture-boundary-json.ps1")
. (Join-Path $FixtureRoot "fixture-ready.ps1")

Assert-FileLineCountAtMost -Path $PSCommandPath -MaxLines 600
Assert-FileLineCountAtMost -Path $ScriptPath -MaxLines 600

$CheckerModuleRoot = Join-Path $PSScriptRoot "platform-core-boundary"
Get-ChildItem -LiteralPath $CheckerModuleRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

$testHelperPath = Join-Path $PSScriptRoot "platform-core-boundary.tests.helpers.ps1"
Assert-FileLineCountAtMost -Path $testHelperPath -MaxLines 600

Get-ChildItem -LiteralPath $FixtureRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

try {
    New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

    $readyRoot = Join-Path $TempRoot "ready"
    Write-ReadyFixture -Root $readyRoot
    $ready = Invoke-Checker -Root $readyRoot
    Assert-Equals $ready.ExitCode 0 "Ready fixture exit code mismatch. Output: $($ready.Output)"
    Assert-Contains $ready.Output "platform-core-boundary-ok"

    $staleExtractedPathRoot = Join-Path $TempRoot "stale-extracted-path"
    Write-ReadyFixture -Root $staleExtractedPathRoot -CreateExtractedCatalogPaths $true
    $staleExtractedPath = Invoke-Checker -Root $staleExtractedPathRoot
    Assert-Equals $staleExtractedPath.ExitCode 1 "Stale extracted path fixture exit code mismatch"
    Assert-Contains $staleExtractedPath.Output "extracted Platform Core path"
    Assert-Contains $staleExtractedPath.Output "crates/domain/core/parcel"

    $staleRawCapturePathRoot = Join-Path $TempRoot "stale-raw-capture-path"
    Write-ReadyFixture -Root $staleRawCapturePathRoot -CreateRawCaptureExtractedPath $true
    $staleRawCapturePath = Invoke-Checker -Root $staleRawCapturePathRoot
    Assert-Equals $staleRawCapturePath.ExitCode 1 "Stale raw-capture path fixture exit code mismatch"
    Assert-Contains $staleRawCapturePath.Output "extracted Platform Core path"
    Assert-Contains $staleRawCapturePath.Output "crates/data-clients/raw-capture"

    $staleR2PublicDataPathRoot = Join-Path $TempRoot "stale-r2-public-data-path"
    Write-ReadyFixture -Root $staleR2PublicDataPathRoot -CreateR2PublicDataExtractedPath $true
    $staleR2PublicDataPath = Invoke-Checker -Root $staleR2PublicDataPathRoot
    Assert-Equals $staleR2PublicDataPath.ExitCode 1 "Stale r2-public-data path fixture exit code mismatch"
    Assert-Contains $staleR2PublicDataPath.Output "extracted Platform Core path"
    Assert-Contains $staleR2PublicDataPath.Output "crates/data-clients/r2-public-data"

    $staleExtractedServicePathRoot = Join-Path $TempRoot "stale-extracted-service-path"
    Write-ReadyFixture -Root $staleExtractedServicePathRoot -CreateExtractedServicePaths $true
    $staleExtractedServicePath = Invoke-Checker -Root $staleExtractedServicePathRoot
    Assert-Equals $staleExtractedServicePath.ExitCode 1 "Stale extracted service path fixture exit code mismatch"
    Assert-Contains $staleExtractedServicePath.Output "extracted Platform Core path"
    Assert-Contains $staleExtractedServicePath.Output "services/data-pipeline"

    $staleVectorTilePathRoot = Join-Path $TempRoot "stale-vector-tile-path"
    Write-ReadyFixture -Root $staleVectorTilePathRoot -CreateExtractedVectorTilePaths $true
    $staleVectorTilePath = Invoke-Checker -Root $staleVectorTilePathRoot
    Assert-Equals $staleVectorTilePath.ExitCode 1 "Stale vector tile path fixture exit code mismatch"
    Assert-Contains $staleVectorTilePath.Output "extracted Platform Core path"
    Assert-Contains $staleVectorTilePath.Output "crates/sp9-base-layer-config"

    $staleCatalogObservabilityPathRoot = Join-Path $TempRoot "stale-catalog-observability-path"
    Write-ReadyFixture -Root $staleCatalogObservabilityPathRoot -CreateExtractedCatalogObservabilityPaths $true
    $staleCatalogObservabilityPath = Invoke-Checker -Root $staleCatalogObservabilityPathRoot
    Assert-Equals $staleCatalogObservabilityPath.ExitCode 1 "Stale Catalog observability path fixture exit code mismatch"
    Assert-Contains $staleCatalogObservabilityPath.Output "extracted Platform Core path"
    Assert-Contains $staleCatalogObservabilityPath.Output ".github/workflows/api-drift-smoke-test.yml"

    $staleCatalogObservabilityDocRoot = Join-Path $TempRoot "stale-catalog-observability-doc"
    Write-ReadyFixture $staleCatalogObservabilityDocRoot -CreateExtractedCatalogObservabilityDoc $true
    $staleCatalogObservabilityDoc = Invoke-Checker -Root $staleCatalogObservabilityDocRoot
    Assert-Equals $staleCatalogObservabilityDoc.ExitCode 1 "Stale Catalog observability doc fixture exit code mismatch"
    Assert-Contains $staleCatalogObservabilityDoc.Output "extracted Platform Core path"
    Assert-Contains $staleCatalogObservabilityDoc.Output "docs/observability/api-drift-smoke-test.md"

    $staleSharedCatalogEventRoot = Join-Path $TempRoot "stale-shared-catalog-event"
    Write-ReadyFixture -Root $staleSharedCatalogEventRoot -CreateExtractedSharedCatalogEventPath $true
    $staleSharedCatalogEvent = Invoke-Checker -Root $staleSharedCatalogEventRoot
    Assert-Equals $staleSharedCatalogEvent.ExitCode 1 "Stale shared Catalog event fixture exit code mismatch"
    Assert-Contains $staleSharedCatalogEvent.Output "extracted Platform Core path"
    Assert-Contains $staleSharedCatalogEvent.Output "shared-kernel/src/catalog_event.rs"

    $staleActiveDocsRoot = Join-Path $TempRoot "stale-active-docs"
    Write-ReadyFixture -Root $staleActiveDocsRoot -CreateStaleActiveDocs $true
    $staleActiveDocs = Invoke-Checker -Root $staleActiveDocsRoot
    Assert-Equals $staleActiveDocs.ExitCode 1 "Stale active docs fixture exit code mismatch"
    Assert-Contains $staleActiveDocs.Output "forbidden active documentation"
    Assert-Contains $staleActiveDocs.Output "AGENTS.md"

    $staleNextActionsRoot = Join-Path $TempRoot "stale-next-actions"
    Write-ReadyFixture -Root $staleNextActionsRoot -CreateStaleNextActions $true
    $staleNextActions = Invoke-Checker -Root $staleNextActionsRoot
    Assert-Equals $staleNextActions.ExitCode 1 "Stale next-actions fixture exit code mismatch"
    Assert-Contains $staleNextActions.Output "forbidden active documentation"
    Assert-Contains $staleNextActions.Output "next-actions.md"

    $staleRoadmapRoot = Join-Path $TempRoot "stale-roadmap-active-section"
    Write-ReadyFixture -Root $staleRoadmapRoot -CreateStaleRoadmapActiveSection $true
    $staleRoadmap = Invoke-Checker -Root $staleRoadmapRoot
    Assert-Equals $staleRoadmap.ExitCode 1 "Stale roadmap active section fixture exit code mismatch"
    Assert-Contains $staleRoadmap.Output "forbidden active documentation section token"
    Assert-Contains $staleRoadmap.Output "roadmap.md"

    $wrongOwnerRoot = Join-Path $TempRoot "wrong-owner"
    Write-ReadyFixture -Root $wrongOwnerRoot -OwnerOverride "platform-core"
    $wrongOwner = Invoke-Checker -Root $wrongOwnerRoot
    Assert-Equals $wrongOwner.ExitCode 1 "Wrong owner fixture exit code mismatch"
    Assert-Contains $wrongOwner.Output "owner mismatch"

    $directDbRoot = Join-Path $TempRoot "direct-db"
    Write-ReadyFixture -Root $directDbRoot
    Write-File -Root $directDbRoot -RelativePath "services\api\src\main.rs" -Content 'let url = "postgres://platform_core";'
    $directDb = Invoke-Checker -Root $directDbRoot
    Assert-Equals $directDb.ExitCode 1 "Direct DB fixture exit code mismatch"
    Assert-Contains $directDb.Output "direct Platform Core database"

    $directDbAliasRoot = Join-Path $TempRoot "direct-db-alias"
    Write-ReadyFixture -Root $directDbAliasRoot
    Write-File -Root $directDbAliasRoot -RelativePath "services\api\src\main.rs" -Content 'let env_name = "PLATFORM_CORE_DB_URL";'
    $directDbAlias = Invoke-Checker -Root $directDbAliasRoot
    Assert-Equals $directDbAlias.ExitCode 1 "Direct DB alias fixture exit code mismatch"
    Assert-Contains $directDbAlias.Output "direct Platform Core database"
    Assert-Contains $directDbAlias.Output "PLATFORM_CORE_DB_URL"

    $rootDirectDbAliasRoot = Join-Path $TempRoot "root-direct-db-alias"
    Write-ReadyFixture -Root $rootDirectDbAliasRoot
    Write-File -Root $rootDirectDbAliasRoot -RelativePath ".env.example" -Content 'PLATFORM_CORE_DB_URL=postgresql://localhost/gongzzang'
    $rootDirectDbAlias = Invoke-Checker -Root $rootDirectDbAliasRoot
    Assert-Equals $rootDirectDbAlias.ExitCode 1 "Root direct DB alias fixture exit code mismatch"
    Assert-Contains $rootDirectDbAlias.Output "direct Platform Core database"
    Assert-Contains $rootDirectDbAlias.Output ".env.example"

    $staleRootEnvExampleRoot = Join-Path $TempRoot "stale-root-env-example"
    Write-ReadyFixture -Root $staleRootEnvExampleRoot -CreateStaleRootEnvExample $true
    $staleRootEnvExample = Invoke-Checker -Root $staleRootEnvExampleRoot
    Assert-Equals $staleRootEnvExample.ExitCode 1 "Stale root env example fixture exit code mismatch"
    Assert-Contains $staleRootEnvExample.Output "root env example"
    Assert-Contains $staleRootEnvExample.Output "VWORLD_*"

    $missingServiceAuthContractRoot = Join-Path $TempRoot "missing-service-auth-contract"
    Write-ReadyFixture -Root $missingServiceAuthContractRoot -OmitServiceAuthEnvContract $true
    $missingServiceAuthContract = Invoke-Checker -Root $missingServiceAuthContractRoot
    Assert-Equals $missingServiceAuthContract.ExitCode 1 "Missing service-auth env contract fixture exit code mismatch"
    Assert-Contains $missingServiceAuthContract.Output "required_service_auth_env"

    $missingServiceAuthRootEnvRoot = Join-Path $TempRoot "missing-service-auth-root-env"
    Write-ReadyFixture -Root $missingServiceAuthRootEnvRoot
    Write-File -Root $missingServiceAuthRootEnvRoot -RelativePath ".env.example" -Content @'
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
PLATFORM_CORE_API_BASE_URL=http://localhost:8081
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=http://localhost:8081
DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:15432/gongzzang
'@
    $missingServiceAuthRootEnv = Invoke-Checker -Root $missingServiceAuthRootEnvRoot
    Assert-Equals $missingServiceAuthRootEnv.ExitCode 1 "Missing service-auth root env fixture exit code mismatch"
    Assert-Contains $missingServiceAuthRootEnv.Output "PLATFORM_CORE_SERVICE_TOKEN"

    $staleLocalPostgresPortRoot = Join-Path $TempRoot "stale-local-postgres-port"
    Write-ReadyFixture -Root $staleLocalPostgresPortRoot -CreateStaleLocalPostgresPort $true
    $staleLocalPostgresPort = Invoke-Checker -Root $staleLocalPostgresPortRoot
    Assert-Equals $staleLocalPostgresPort.ExitCode 1 "Stale local Postgres port fixture exit code mismatch"
    Assert-Contains $staleLocalPostgresPort.Output "local Gongzzang Postgres"
    Assert-Contains $staleLocalPostgresPort.Output "15432"

    $catalogRawTableRoot = Join-Path $TempRoot "catalog-raw-table"
    Write-ReadyFixture -Root $catalogRawTableRoot
    Write-File -Root $catalogRawTableRoot -RelativePath "services\api\src\main.rs" -Content 'let table = "parcel_external_data";'
    $catalogRawTable = Invoke-Checker -Root $catalogRawTableRoot
    Assert-Equals $catalogRawTable.ExitCode 1 "Catalog raw table fixture exit code mismatch"
    Assert-Contains $catalogRawTable.Output "forbidden direct Platform Core"
    Assert-Contains $catalogRawTable.Output "parcel_external_data"

    $directVworldApiRoot = Join-Path $TempRoot "direct-vworld-api"
    Write-ReadyFixture -Root $directVworldApiRoot
    Write-File -Root $directVworldApiRoot -RelativePath "apps\worker\README.md" -Content "vworld-cache-refresh calls api.vworld.kr"
    $directVworldApi = Invoke-Checker -Root $directVworldApiRoot
    Assert-Equals $directVworldApi.ExitCode 1 "Direct V-World API fixture exit code mismatch"
    Assert-Contains $directVworldApi.Output "forbidden direct Platform Core"
    Assert-Contains $directVworldApi.Output "api.vworld.kr"

    $directDataGoKrApiRoot = Join-Path $TempRoot "direct-data-go-kr-api"
    Write-ReadyFixture -Root $directDataGoKrApiRoot
    Write-File -Root $directDataGoKrApiRoot -RelativePath "services\worker\job.py" -Content 'url = "https://apis.data.go.kr/building-register-sync"'
    $directDataGoKrApi = Invoke-Checker -Root $directDataGoKrApiRoot
    Assert-Equals $directDataGoKrApi.ExitCode 1 "Direct data.go.kr API fixture exit code mismatch"
    Assert-Contains $directDataGoKrApi.Output "forbidden direct Platform Core"
    Assert-Contains $directDataGoKrApi.Output "apis.data.go.kr"

    $directCatalogWorkflowRoot = Join-Path $TempRoot "direct-catalog-workflow"
    Write-ReadyFixture -Root $directCatalogWorkflowRoot
    Write-File -Root $directCatalogWorkflowRoot -RelativePath ".github\workflows\catalog-refresh.yml" -Content "run: curl https://api.vworld.kr/catalog-refresh"
    $directCatalogWorkflow = Invoke-Checker -Root $directCatalogWorkflowRoot
    Assert-Equals $directCatalogWorkflow.ExitCode 1 "Direct Catalog workflow fixture exit code mismatch"
    Assert-Contains $directCatalogWorkflow.Output "forbidden direct Platform Core"
    Assert-Contains $directCatalogWorkflow.Output "api.vworld.kr"
    Assert-Contains $directCatalogWorkflow.Output ".github/workflows/catalog-refresh.yml"

    $newLegacyMigrationRoot = Join-Path $TempRoot "new-legacy-migration"
    Write-ReadyFixture -Root $newLegacyMigrationRoot
    Write-File -Root $newLegacyMigrationRoot -RelativePath "migrations\99999_new_legacy.sql" -Content "create table legacy_probe(id bigint); select * from parcel_external_data;"
    $newLegacyMigration = Invoke-Checker -Root $newLegacyMigrationRoot
    Assert-Equals $newLegacyMigration.ExitCode 1 "New legacy migration fixture exit code mismatch"
    Assert-Contains $newLegacyMigration.Output "unapproved legacy schema token"
    Assert-Contains $newLegacyMigration.Output "migrations/99999_new_legacy.sql"

    $missingAnchorImporterRoot = Join-Path $TempRoot "missing-anchor-importer"
    Write-ReadyFixture -Root $missingAnchorImporterRoot -OmitAnchorImporter $true
    $missingAnchorImporter = Invoke-Checker -Root $missingAnchorImporterRoot
    Assert-Equals $missingAnchorImporter.ExitCode 1 "Missing anchor importer boundary entry exit code mismatch"
    Assert-Contains $missingAnchorImporter.Output "missing path ownership entry"
    Assert-Contains $missingAnchorImporter.Output "platform_core_anchor_import.rs"

    $missingAnchorInboxMigrationRoot = Join-Path $TempRoot "missing-anchor-inbox-migration-entry"
    Write-ReadyFixture -Root $missingAnchorInboxMigrationRoot -OmitAnchorInboxMigration $true
    $missingAnchorInboxMigration = Invoke-Checker -Root $missingAnchorInboxMigrationRoot
    Assert-Equals $missingAnchorInboxMigration.ExitCode 1 "Missing anchor inbox migration boundary entry exit code mismatch"
    Assert-Contains $missingAnchorInboxMigration.Output "missing path ownership entry"
    Assert-Contains $missingAnchorInboxMigration.Output "30016_platform_core_event_inbox_anchor_import.sql"

    $missingAnchorDbImportRoot = Join-Path $TempRoot "missing-anchor-db-import-entry"
    Write-ReadyFixture -Root $missingAnchorDbImportRoot -OmitAnchorDbImport $true
    $missingAnchorDbImport = Invoke-Checker -Root $missingAnchorDbImportRoot
    Assert-Equals $missingAnchorDbImport.ExitCode 1 "Missing anchor DB import boundary entry exit code mismatch"
    Assert-Contains $missingAnchorDbImport.Output "missing path ownership entry"
    Assert-Contains $missingAnchorDbImport.Output "crates/db/src/platform_core_anchor.rs"

    $missingRustEventReceiverRoot = Join-Path $TempRoot "missing-rust-event-receiver-entry"
    Write-ReadyFixture -Root $missingRustEventReceiverRoot -OmitRustEventReceiver $true
    $missingRustEventReceiver = Invoke-Checker -Root $missingRustEventReceiverRoot
    Assert-Equals $missingRustEventReceiver.ExitCode 1 "Missing Rust event receiver boundary entry exit code mismatch"
    Assert-Contains $missingRustEventReceiver.Output "missing path ownership entry"
    Assert-Contains $missingRustEventReceiver.Output "routes/platform_core_events.rs"

    $missingAnchorImporterBinRoot = Join-Path $TempRoot "missing-anchor-importer-bin-entry"
    Write-ReadyFixture -Root $missingAnchorImporterBinRoot -OmitAnchorImporterBin $true
    $missingAnchorImporterBin = Invoke-Checker -Root $missingAnchorImporterBinRoot
    Assert-Equals $missingAnchorImporterBin.ExitCode 1 "Missing anchor importer bin boundary entry exit code mismatch"
    Assert-Contains $missingAnchorImporterBin.Output "missing path ownership entry"
    Assert-Contains $missingAnchorImporterBin.Output "bin/platform_core_anchor_import.rs"

    $missingWebhookPinRoot = Join-Path $TempRoot "missing-webhook-pin"
    Write-ReadyFixture -Root $missingWebhookPinRoot -OmitWebhookPin $true
    $missingWebhookPin = Invoke-Checker -Root $missingWebhookPinRoot
    Assert-Equals $missingWebhookPin.ExitCode 1 "Missing webhook pin boundary entry exit code mismatch"
    Assert-Contains $missingWebhookPin.Output "missing path ownership entry"
    Assert-Contains $missingWebhookPin.Output "platform-core-webhook-receiver"
    Assert-Contains $missingWebhookPin.Output "pin.json"

    $missingCatalogApiPinRoot = Join-Path $TempRoot "missing-catalog-api-pin"
    Write-ReadyFixture -Root $missingCatalogApiPinRoot -OmitCatalogApiPin $true
    $missingCatalogApiPin = Invoke-Checker -Root $missingCatalogApiPinRoot
    Assert-Equals $missingCatalogApiPin.ExitCode 1 "Missing Catalog API pin boundary entry exit code mismatch"
    Assert-Contains $missingCatalogApiPin.Output "missing path ownership entry"
    Assert-Contains $missingCatalogApiPin.Output "platform-core-catalog-api-contract"
    Assert-Contains $missingCatalogApiPin.Output "pin.json"

    $missingEventGateRoot = Join-Path $TempRoot "missing-event-gate"
    Write-ReadyFixture -Root $missingEventGateRoot -OmitEventReceiverGate $true
    $missingEventGate = Invoke-Checker -Root $missingEventGateRoot
    Assert-Equals $missingEventGate.ExitCode 1 "Missing event receiver gate boundary entry exit code mismatch"
    Assert-Contains $missingEventGate.Output "missing required CI gate"
    Assert-Contains $missingEventGate.Output "check-platform-core-event-receiver-contract.ps1"

    $missingCatalogApiGateRoot = Join-Path $TempRoot "missing-catalog-api-gate"
    Write-ReadyFixture -Root $missingCatalogApiGateRoot -OmitCatalogApiGate $true
    $missingCatalogApiGate = Invoke-Checker -Root $missingCatalogApiGateRoot
    Assert-Equals $missingCatalogApiGate.ExitCode 1 "Missing Catalog API contract gate boundary entry exit code mismatch"
    Assert-Contains $missingCatalogApiGate.Output "missing required CI gate"
    Assert-Contains $missingCatalogApiGate.Output "check-platform-core-catalog-api-contract.ps1"

    $missingDependencyGateRoot = Join-Path $TempRoot "missing-dependency-gate"
    Write-ReadyFixture -Root $missingDependencyGateRoot -OmitDependencyGate $true
    $missingDependencyGate = Invoke-Checker -Root $missingDependencyGateRoot
    Assert-Equals $missingDependencyGate.ExitCode 1 "Missing dependency boundary gate entry exit code mismatch"
    Assert-Contains $missingDependencyGate.Output "missing required CI gate"
    Assert-Contains $missingDependencyGate.Output "check-platform-core-dependency-boundary.ps1"

    $missingMigrationSmokeWorkflowRoot = Join-Path $TempRoot "missing-migration-smoke-workflow"
    Write-ReadyFixture -Root $missingMigrationSmokeWorkflowRoot -OmitMigrationSmokeWorkflow $true
    $missingMigrationSmokeWorkflow = Invoke-Checker -Root $missingMigrationSmokeWorkflowRoot
    Assert-Equals $missingMigrationSmokeWorkflow.ExitCode 1 "Missing migration smoke workflow exit code mismatch"
    Assert-Contains $missingMigrationSmokeWorkflow.Output "db-migrations.yml"

    $staleMigrationSmokeWorkflowRoot = Join-Path $TempRoot "stale-migration-smoke-workflow"
    Write-ReadyFixture -Root $staleMigrationSmokeWorkflowRoot -CreateStaleMigrationSmokeWorkflow $true
    $staleMigrationSmokeWorkflow = Invoke-Checker -Root $staleMigrationSmokeWorkflowRoot
    Assert-Equals $staleMigrationSmokeWorkflow.ExitCode 1 "Stale migration smoke workflow exit code mismatch"
    Assert-Contains $staleMigrationSmokeWorkflow.Output "migration smoke workflow"
    Assert-Contains $staleMigrationSmokeWorkflow.Output "test_v001_full.sh"

    $missingNextActionsRuleRoot = Join-Path $TempRoot "missing-next-actions-rule"
    Write-ReadyFixture -Root $missingNextActionsRuleRoot -OmitNextActionsDocRule $true
    $missingNextActionsRule = Invoke-Checker -Root $missingNextActionsRuleRoot
    Assert-Equals $missingNextActionsRule.ExitCode 1 "Missing next-actions doc rule exit code mismatch"
    Assert-Contains $missingNextActionsRule.Output "missing forbidden_active"
    Assert-Contains $missingNextActionsRule.Output "next-actions.md"

    $missingRoadmapSectionRuleRoot = Join-Path $TempRoot "missing-roadmap-section-rule"
    Write-ReadyFixture -Root $missingRoadmapSectionRuleRoot -OmitRoadmapSectionRule $true
    $missingRoadmapSectionRule = Invoke-Checker -Root $missingRoadmapSectionRuleRoot
    Assert-Equals $missingRoadmapSectionRule.ExitCode 1 "Missing roadmap section rule exit code mismatch"
    Assert-Contains $missingRoadmapSectionRule.Output "missing forbidden_active_documentation_section_tokens entry"
    Assert-Contains $missingRoadmapSectionRule.Output "roadmap.md"

    $missingCleanupMigrationRoot = Join-Path $TempRoot "missing-cleanup-migration"
    Write-ReadyFixture -Root $missingCleanupMigrationRoot -OmitCleanupMigration $true
    $missingCleanupMigration = Invoke-Checker -Root $missingCleanupMigrationRoot
    Assert-Equals $missingCleanupMigration.ExitCode 1 "Missing cleanup migration exit code mismatch"
    Assert-Contains $missingCleanupMigration.Output "30015_drop_platform_core_legacy_schema.sql"

    $nonDropCleanupMigrationRoot = Join-Path $TempRoot "non-drop-cleanup-migration"
    Write-ReadyFixture -Root $nonDropCleanupMigrationRoot -CreateNonDropCleanupMigration $true
    $nonDropCleanupMigration = Invoke-Checker -Root $nonDropCleanupMigrationRoot
    Assert-Equals $nonDropCleanupMigration.ExitCode 1 "Non-drop cleanup migration exit code mismatch"
    Assert-Contains $nonDropCleanupMigration.Output "cleanup migration must drop"
    Assert-Contains $nonDropCleanupMigration.Output "api_health_check"

    $staleMigrationSmokeRoot = Join-Path $TempRoot "stale-migration-smoke"
    Write-ReadyFixture -Root $staleMigrationSmokeRoot -CreateStaleMigrationSmokeExpectations $true
    $staleMigrationSmoke = Invoke-Checker -Root $staleMigrationSmokeRoot
    Assert-Equals $staleMigrationSmoke.ExitCode 1 "Stale migration smoke fixture exit code mismatch"
    Assert-Contains $staleMigrationSmoke.Output "migration smoke"
    Assert-Contains $staleMigrationSmoke.Output "api_health_check"

    $canonicalCatalogMigrationRoot = Join-Path $TempRoot "canonical-catalog-migration"
    Write-ReadyFixture -Root $canonicalCatalogMigrationRoot -CreateCanonicalCatalogMigration $true
    $canonicalCatalogMigration = Invoke-Checker -Root $canonicalCatalogMigrationRoot
    Assert-Equals $canonicalCatalogMigration.ExitCode 1 "Canonical Catalog migration exit code mismatch"
    Assert-Contains $canonicalCatalogMigration.Output "canonical Catalog table"
    Assert-Contains $canonicalCatalogMigration.Output "industrial_complex"

    $canonicalCatalogSqlUsageRoot = Join-Path $TempRoot "canonical-catalog-sql-usage"
    Write-ReadyFixture -Root $canonicalCatalogSqlUsageRoot -CreateCanonicalCatalogSqlUsage $true
    $canonicalCatalogSqlUsage = Invoke-Checker -Root $canonicalCatalogSqlUsageRoot
    Assert-Equals $canonicalCatalogSqlUsage.ExitCode 1 "Canonical Catalog SQL usage exit code mismatch"
    Assert-Contains $canonicalCatalogSqlUsage.Output "canonical Catalog table"
    Assert-Contains $canonicalCatalogSqlUsage.Output "building"

    $schemaQualifiedCanonicalCatalogSqlUsageRoot = Join-Path $TempRoot "schema-qualified-canonical-catalog-sql-usage"
    Write-ReadyFixture -Root $schemaQualifiedCanonicalCatalogSqlUsageRoot -CreateSchemaQualifiedCanonicalCatalogSqlUsage $true
    $schemaQualifiedCanonicalCatalogSqlUsage = Invoke-Checker -Root $schemaQualifiedCanonicalCatalogSqlUsageRoot
    Assert-Equals $schemaQualifiedCanonicalCatalogSqlUsage.ExitCode 1 "Schema-qualified canonical Catalog SQL usage exit code mismatch"
    Assert-Contains $schemaQualifiedCanonicalCatalogSqlUsage.Output "canonical Catalog table"
    Assert-Contains $schemaQualifiedCanonicalCatalogSqlUsage.Output "catalog_schema_probe.rs"
    Assert-Contains $schemaQualifiedCanonicalCatalogSqlUsage.Output "parcel"

    $missingCiRoot = Join-Path $TempRoot "missing-ci"
    Write-ReadyFixture -Root $missingCiRoot
    Write-File -Root $missingCiRoot -RelativePath ".github\workflows\ci.yml" -Content "run: pnpm test"
    $missingCi = Invoke-Checker -Root $missingCiRoot
    Assert-Equals $missingCi.ExitCode 1 "Missing CI fixture exit code mismatch"
    Assert-Contains $missingCi.Output "CI workflow must run"

    Write-Host "check-platform-core-boundary-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
