function Write-ReadyFixture {
    param(
        [string] $Root,
        [string] $OwnerOverride = "",
        [bool] $OmitAnchorImporter = $false,
        [bool] $OmitAnchorInboxMigration = $false,
        [bool] $OmitAnchorDbImport = $false,
        [bool] $OmitRustEventReceiver = $false,
        [bool] $OmitAnchorImporterBin = $false,
        [bool] $OmitEventReceiverGate = $false,
        [bool] $OmitWebhookPin = $false,
        [bool] $OmitCatalogApiGate = $false,
        [bool] $OmitCatalogApiPin = $false,
        [bool] $OmitDependencyGate = $false,
        [bool] $OmitServiceAuthEnvContract = $false,
        [bool] $OmitMigrationSmokeWorkflow = $false,
        [bool] $OmitNextActionsDocRule = $false,
        [bool] $OmitRoadmapSectionRule = $false,
        [bool] $CreateExtractedCatalogPaths = $false,
        [bool] $CreateExtractedServicePaths = $false,
        [bool] $CreateRawCaptureExtractedPath = $false,
        [bool] $CreateR2PublicDataExtractedPath = $false,
        [bool] $CreateExtractedVectorTilePaths = $false,
        [bool] $CreateExtractedCatalogObservabilityPaths = $false,
        [bool] $CreateExtractedCatalogObservabilityDoc = $false,
        [bool] $CreateExtractedSharedCatalogEventPath = $false,
        [bool] $CreateStaleNextActions = $false,
        [bool] $CreateStaleRoadmapActiveSection = $false,
        [bool] $OmitCleanupMigration = $false,
        [bool] $CreateNonDropCleanupMigration = $false,
        [bool] $CreateCanonicalCatalogMigration = $false,
        [bool] $CreateCanonicalCatalogSqlUsage = $false,
        [bool] $CreateSchemaQualifiedCanonicalCatalogSqlUsage = $false,
        [bool] $CreateStaleMigrationSmokeExpectations = $false,
        [bool] $CreateStaleMigrationSmokeWorkflow = $false,
        [bool] $CreateStaleActiveDocs = $false,
        [bool] $CreateStaleRootEnvExample = $false,
        [bool] $CreateStaleLocalPostgresPort = $false
    )

    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content (New-BoundaryJson -OwnerOverride $OwnerOverride -OmitAnchorImporter $OmitAnchorImporter -OmitAnchorInboxMigration $OmitAnchorInboxMigration -OmitAnchorDbImport $OmitAnchorDbImport -OmitRustEventReceiver $OmitRustEventReceiver -OmitAnchorImporterBin $OmitAnchorImporterBin -OmitEventReceiverGate $OmitEventReceiverGate -OmitWebhookPin $OmitWebhookPin -OmitCatalogApiGate $OmitCatalogApiGate -OmitCatalogApiPin $OmitCatalogApiPin -OmitDependencyGate $OmitDependencyGate -OmitServiceAuthEnvContract $OmitServiceAuthEnvContract -OmitMigrationSmokeWorkflow $OmitMigrationSmokeWorkflow -OmitNextActionsDocRule $OmitNextActionsDocRule -OmitRoadmapSectionRule $OmitRoadmapSectionRule)
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @'
run: ./scripts/ci/check-platform-core-boundary.ps1
run: ./scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
run: ./scripts/ci/check-platform-core-event-receiver-contract.ps1
run: ./scripts/ci/check-platform-core-catalog-api-contract.ps1
run: ./scripts/ci/check-platform-core-dependency-boundary.ps1
run: bash scripts/lefthook/catalog-m1-boundary.sh
'@
    if (!$OmitMigrationSmokeWorkflow) {
        $migrationWorkflow = if ($CreateStaleMigrationSmokeWorkflow) {
            @'
name: db-migrations
env:
  POSTGRES_DB: gongzzang
jobs:
  migrate:
    services:
      postgres:
        image: postgis/postgis:17-3.5
    env:
      DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang
    steps:
      - run: echo "migration smoke intentionally missing"
'@
        } else {
            @'
name: db-migrations
env:
  POSTGRES_DB: gongzzang
jobs:
  migrate:
    services:
      postgres:
        image: postgis/postgis:17-3.5
    env:
      DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang
    steps:
      - run: bazelisk test //tools/bazel:ci_migration_v001_full_transition --config=ci --verbose_failures
'@
        }
        Write-File -Root $Root -RelativePath ".github\workflows\db-migrations.yml" -Content $migrationWorkflow
    }
    Write-File -Root $Root -RelativePath "lefthook.yml" -Content @'
run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-boundary.ps1
run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-event-receiver-contract.ps1
run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-catalog-api-contract.ps1
run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-dependency-boundary.ps1
run: bash scripts/lefthook/catalog-m1-boundary.sh
'@
    $rootEnvExample = if ($CreateStaleRootEnvExample) {
        @'
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
PLATFORM_CORE_API_BASE_URL=http://localhost:8081
PLATFORM_CORE_SERVICE_TOKEN=fixture-platform-core-service-token
PLATFORM_CORE_WEBHOOK_SECRET=fixture-platform-core-webhook-secret
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=http://localhost:8081
DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:15432/gongzzang
VWORLD_API_KEY=legacy
ODP_SERVICE_KEY=legacy
R2_LOCAL_ACCOUNT_ID=legacy
'@
    } elseif ($CreateStaleLocalPostgresPort) {
        @'
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
PLATFORM_CORE_API_BASE_URL=http://localhost:8081
PLATFORM_CORE_SERVICE_TOKEN=fixture-platform-core-service-token
PLATFORM_CORE_WEBHOOK_SECRET=fixture-platform-core-webhook-secret
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=http://localhost:8081
DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:5500/gongzzang
'@
    } else {
        @'
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
PLATFORM_CORE_API_BASE_URL=http://localhost:8081
PLATFORM_CORE_SERVICE_TOKEN=fixture-platform-core-service-token
PLATFORM_CORE_WEBHOOK_SECRET=fixture-platform-core-webhook-secret
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=http://localhost:8081
DATABASE_URL=postgres://gongzzang:changeme_local_only@localhost:15432/gongzzang
'@
    }
    Write-File -Root $Root -RelativePath ".env.example" -Content $rootEnvExample

    $dockerCompose = if ($CreateStaleLocalPostgresPort) {
        @'
services:
  postgres:
    image: postgis/postgis:17-3.5
    ports: ["5500:5432"]
'@
    } else {
        @'
services:
  postgres:
    image: postgis/postgis:17-3.5
    ports: ["${POSTGRES_HOST_PORT:-15432}:5432"]
'@
    }
    Write-File -Root $Root -RelativePath "infrastructure\docker\docker-compose.yml" -Content $dockerCompose

    $dockerEnvExample = if ($CreateStaleLocalPostgresPort) {
        @'
POSTGRES_USER=gongzzang
POSTGRES_PASSWORD=changeme_local_only
POSTGRES_DB=gongzzang
'@
    } else {
        @'
POSTGRES_USER=gongzzang
POSTGRES_PASSWORD=changeme_local_only
POSTGRES_DB=gongzzang
POSTGRES_HOST_PORT=15432
'@
    }
    Write-File -Root $Root -RelativePath "infrastructure\docker\.env.example" -Content $dockerEnvExample
    foreach ($path in @(
            "crates\domain\core\listing",
            "crates\domain\core\listing-photo",
            "crates\domain\core\user",
            "crates\domain\market",
            "crates\domain\insights",
            "services\etl-base-layer"
        )) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root $path) | Out-Null
    }
    if ($CreateExtractedCatalogPaths) {
        foreach ($path in @(
                "crates\domain\core\parcel",
                "crates\data-clients\vworld"
            )) {
            New-Item -ItemType Directory -Force -Path (Join-Path $Root $path) | Out-Null
        }
    }
    if ($CreateRawCaptureExtractedPath) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "crates\data-clients\raw-capture") | Out-Null
    }
    if ($CreateR2PublicDataExtractedPath) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "crates\data-clients\r2-public-data") | Out-Null
    }
    if ($CreateExtractedServicePaths) {
        foreach ($path in @(
                "services\data-pipeline",
                "services\scraper-py"
            )) {
            New-Item -ItemType Directory -Force -Path (Join-Path $Root $path) | Out-Null
        }
    }
    if ($CreateExtractedVectorTilePaths) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "crates\sp9-base-layer-config") | Out-Null
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "services\etl-base-layer\scripts") | Out-Null
        Write-File -Root $Root -RelativePath ".github\workflows\sp9-base-layer-etl.yml" -Content "name: sp9-base-layer-etl"
        Write-File -Root $Root -RelativePath "scripts\setup-dev-tippecanoe.sh" -Content "#!/usr/bin/env bash"
        Write-File -Root $Root -RelativePath "services\etl-base-layer\Dockerfile.etl" -Content "FROM scratch"
    }
    if ($CreateExtractedCatalogObservabilityPaths) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "crates\operations\api-health") | Out-Null
        New-Item -ItemType Directory -Force -Path (Join-Path $Root "crates\api-health-recorder") | Out-Null
        Write-File -Root $Root -RelativePath ".github\workflows\api-drift-smoke-test.yml" -Content "name: api-drift-smoke-test"
        Write-File -Root $Root -RelativePath "crates\db\src\api_health.rs" -Content "pub struct PgHealthCheckRepository;"
        Write-File -Root $Root -RelativePath "crates\db\tests\api_health_integration.rs" -Content "mod api_health_integration;"
    }
    if ($CreateExtractedCatalogObservabilityDoc) {
        Write-File -Root $Root -RelativePath "docs\observability\api-drift-smoke-test.md" -Content "# API drift smoke test"
    }
    if ($CreateExtractedSharedCatalogEventPath) {
        Write-File -Root $Root -RelativePath "crates\domain\core\shared-kernel\src\catalog_event.rs" -Content "pub struct CatalogEventV1;"
    }
    Write-File -Root $Root -RelativePath "docs\conventions\rust.md" -Content "services/* -> crates/*"
    Write-File -Root $Root -RelativePath "docs\conventions\testing.md" -Content "crates/db | 70%"
    Write-File -Root $Root -RelativePath "crates\data-clients\README.md" -Content "# crates/data-clients`nGongzzang-owned non-Catalog adapters only."
    Write-File -Root $Root -RelativePath "AGENTS.md" -Content "# AGENTS`nCatalog raw lineage belongs to Platform Core. Gongzzang-owned external adapters need approved lineage contracts."
    Write-File -Root $Root -RelativePath "docs\superpowers\next-actions.md" -Content "# Next Actions`nCurrent queue only."
    Write-File -Root $Root -RelativePath "docs\superpowers\roadmap.md" -Content @'
# Roadmap
## sentinel
no stale token
## end
## 다음 sub-project
Platform Core boundary verification, DB cleanup approval, and Gongzzang-owned product work.
## 추천 순서
1. Platform Core boundary verification.
## Production
'@
    if ($CreateStaleActiveDocs) {
        Write-File -Root $Root -RelativePath "AGENTS.md" -Content @'
# AGENTS
- raw ?묐떟 蹂댁〈 (`raw_response JSONB` 而щ읆)
- 怨듦났 API raw ??DB `raw_response JSONB`
'@
        Write-File -Root $Root -RelativePath "docs\conventions\rust.md" -Content "crates/data-clients/* ??crates/{circuit-breaker, observability, api-types}"
        Write-File -Root $Root -RelativePath "docs\conventions\testing.md" -Content '| `crates/data-clients/*`, `crates/db` | 70% |'
        Write-File -Root $Root -RelativePath "crates\data-clients\README.md" -Content @'
# crates/data-clients
- `vworld/` ??V-World API
- `data-go-kr/` ??怨듦났?곗씠?고룷??- raw_response *??긽* 蹂댁〈 (DB JSONB 而щ읆)
'@
    }
    if ($CreateStaleNextActions) {
        Write-File -Root $Root -RelativePath "docs\superpowers\next-actions.md" -Content @'
# Next Actions
- ?좉퇋 ?뚯씪: crates/data-clients/data-go-kr/src/real_transaction/client.rs
- ?좉퇋 crate: crates/data-clients/r2-public-data/
- raw_capture(source = "data_go_kr_tx")
- services/scraper-py/dtmk_vworld.py
'@
    }
    if ($CreateStaleRoadmapActiveSection) {
        Write-File -Root $Root -RelativePath "docs\superpowers\roadmap.md" -Content @'
# Roadmap
## sentinel
no stale token
## end
## 다음 sub-project
### A. SP4-iii — data.go.kr + 법제처 + R2 Reader 6 (남은 분해)
| 4-iii-b | data.go.kr 실거래가 + RealTransactionReader | 미착수 |
## 추천 순서
SP4-iii-b 실거래가, SP4-iii-c 법제처, SP4-iii-e R2 PMTiles
## Production
'@
    }
    Write-File -Root $Root -RelativePath "migrations\30012_parcel_marker_anchor_projection.sql" -Content "create table parcel_marker_anchor"
    Write-File -Root $Root -RelativePath "migrations\30013_listing_marker_projection.sql" -Content "create table listing_marker_projection"
    Write-File -Root $Root -RelativePath "migrations\30016_platform_core_event_inbox_anchor_import.sql" -Content "create table platform_core_event_inbox"
    if (!$OmitCleanupMigration) {
        $cleanupMigration = if ($CreateNonDropCleanupMigration) {
            "select * from api_health_check; select * from parcel_external_data; select * from pipeline_run; select * from pipeline_schedule;"
        } else {
            "drop table if exists api_health_check;`ndrop table if exists parcel_external_data;`ndrop table if exists pipeline_run;`ndrop table if exists pipeline_schedule;"
        }
        Write-File -Root $Root -RelativePath "migrations\30015_drop_platform_core_legacy_schema.sql" -Content $cleanupMigration
    }
    Write-File -Root $Root -RelativePath "migrations\10004_pipeline_tables.sql" -Content "create table pipeline_schedule(id char(30)); create table pipeline_run(id char(30));"
    Write-File -Root $Root -RelativePath "migrations\10005_operations_tables.sql" -Content "-- legacy action_kind example: force_pipeline_run"
    Write-File -Root $Root -RelativePath "migrations\30006_parcel_external_data.sql" -Content "create table parcel_external_data(pnu char(19));"
    Write-File -Root $Root -RelativePath "migrations\30007_api_health_check.sql" -Content "create table api_health_check(id char(30));"
    Write-File -Root $Root -RelativePath "migrations\30010_parcel_external_data_r2_pointer.sql" -Content "alter table parcel_external_data add column r2_object_key text;"
    Write-File -Root $Root -RelativePath "migrations\30011_parcel_external_data_r2_key_idx.sql" -Content "create index parcel_external_data_r2_key_idx on parcel_external_data(r2_object_key);"
    $migrationSmoke = if ($CreateStaleMigrationSmokeExpectations) {
        @'
EXPECTED_TABLES=(
  "user"
  parcel_external_data
  pipeline_run
  pipeline_schedule
  api_health_check
)
FORBIDDEN_TABLES=(
  api_health_check
  parcel_external_data
  pipeline_run
  pipeline_schedule
)
'@
    } else {
        @'
EXPECTED_TABLES=(
  "user"
  listing
)
FORBIDDEN_TABLES=(
  api_health_check
  parcel_external_data
  pipeline_run
  pipeline_schedule
)
'@
    }
    Write-File -Root $Root -RelativePath "tests\migrations\test_v001_full.sh" -Content $migrationSmoke
    if ($CreateCanonicalCatalogMigration) {
        Write-File -Root $Root -RelativePath "migrations\99998_create_catalog_master.sql" -Content "create table industrial_complex(id text primary key);"
    }
    if ($CreateCanonicalCatalogSqlUsage) {
        Write-File -Root $Root -RelativePath "services\api\src\catalog_probe.rs" -Content 'let rows = sqlx::query("select * from building where id = $1");'
    }
    if ($CreateSchemaQualifiedCanonicalCatalogSqlUsage) {
        Write-File -Root $Root -RelativePath "services\api\src\catalog_schema_probe.rs" -Content 'let rows = sqlx::query("select * from catalog.parcel where pnu = $1");'
    }
    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-webhook-receiver-contract.v1.pin.json" -Content "{}"
    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-catalog-api-contract.v1.pin.json" -Content "{}"
    Write-File -Root $Root -RelativePath "services\api\src\routes\listing_marker_tiles.rs" -Content "pub fn listing_marker_tiles() {}"
    Write-File -Root $Root -RelativePath "services\api\src\platform_core_anchor_import.rs" -Content "pub fn parse_anchor_manifest() {}"
    Write-File -Root $Root -RelativePath "crates\db\src\platform_core_anchor.rs" -Content "pub fn import_anchor_rows() {}"
    Write-File -Root $Root -RelativePath "services\api\src\routes\platform_core_events.rs" -Content "pub fn post_platform_core_event() {}"
    Write-File -Root $Root -RelativePath "services\api\src\bin\platform_core_anchor_import.rs" -Content "fn main() {}"
    Write-File -Root $Root -RelativePath "apps\web\app\platform-core\events\route.ts" -Content "export const runtime = 'nodejs';"
    Write-File -Root $Root -RelativePath "scripts\ci\check-platform-core-boundary.ps1" -Content "Write-Host ok"
    Write-File -Root $Root -RelativePath "scripts\lefthook\catalog-m1-boundary.sh" -Content "#!/usr/bin/env bash"
    Write-File -Root $Root -RelativePath "scripts\ci\check-pnu-anchor-pbf-marker-contract.ps1" -Content "Write-Host ok"
    Write-File -Root $Root -RelativePath "scripts\ci\check-platform-core-event-receiver-contract.ps1" -Content "Write-Host ok"
    Write-File -Root $Root -RelativePath "scripts\ci\check-platform-core-catalog-api-contract.ps1" -Content "Write-Host ok"
    Write-File -Root $Root -RelativePath "scripts\ci\check-platform-core-dependency-boundary.ps1" -Content "Write-Host ok"
}
