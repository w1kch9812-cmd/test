Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-boundary.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-boundary-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
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

function New-BoundaryJson {
    param(
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
        [bool] $CreateExtractedServicePaths = $false,
        [bool] $CreateRawCaptureExtractedPath = $false,
        [bool] $CreateExtractedVectorTilePaths = $false,
        [bool] $CreateExtractedCatalogObservabilityPaths = $false,
        [bool] $CreateExtractedCatalogObservabilityDoc = $false
    )

    $listingOwner = if ([string]::IsNullOrWhiteSpace($OwnerOverride)) { "gongzzang" } else { $OwnerOverride }
    $anchorImporterEntry = if ($OmitAnchorImporter) {
        ""
    } else {
        ',{"path":"services/api/src/platform_core_anchor_import.rs","owner":"gongzzang","classification":"platform_core_read_model_import_contract"}'
    }
    $anchorInboxMigrationEntry = if ($OmitAnchorInboxMigration) {
        ""
    } else {
        ',{"path":"migrations/30016_platform_core_event_inbox_anchor_import.sql","owner":"gongzzang","classification":"platform_core_event_inbox"}'
    }
    $anchorDbImportEntry = if ($OmitAnchorDbImport) {
        ""
    } else {
        ',{"path":"crates/db/src/platform_core_anchor.rs","owner":"gongzzang","classification":"platform_core_read_model_import"}'
    }
    $rustEventReceiverEntry = if ($OmitRustEventReceiver) {
        ""
    } else {
        ',{"path":"services/api/src/routes/platform_core_events.rs","owner":"gongzzang","classification":"platform_core_event_receiver"}'
    }
    $anchorImporterBinEntry = if ($OmitAnchorImporterBin) {
        ""
    } else {
        ',{"path":"services/api/src/bin/platform_core_anchor_import.rs","owner":"gongzzang","classification":"platform_core_read_model_importer"}'
    }
    $webhookPinEntry = if ($OmitWebhookPin) {
        ""
    } else {
        ',{"path":"docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json","owner":"platform-core","classification":"contract_pin_copy"}'
    }
    $catalogApiPinEntry = if ($OmitCatalogApiPin) {
        ""
    } else {
        ',{"path":"docs/architecture/platform-core-catalog-api-contract.v1.pin.json","owner":"platform-core","classification":"contract_pin_copy"}'
    }
    $eventReceiverGateEntry = if ($OmitEventReceiverGate) {
        ""
    } else {
        ',"scripts/ci/check-platform-core-event-receiver-contract.ps1"'
    }
    $catalogApiGateEntry = if ($OmitCatalogApiGate) {
        ""
    } else {
        ',"scripts/ci/check-platform-core-catalog-api-contract.ps1"'
    }
    $dependencyGateEntry = if ($OmitDependencyGate) {
        ""
    } else {
        ',"scripts/ci/check-platform-core-dependency-boundary.ps1"'
    }
    $serviceAuthEnvContract = if ($OmitServiceAuthEnvContract) {
        ""
    } else {
        @'
,
    "required_service_auth_env": [
      "PLATFORM_CORE_SERVICE_TOKEN",
      "PLATFORM_CORE_WEBHOOK_SECRET"
    ]
'@
    }
    $migrationSmokeWorkflowEntry = if ($OmitMigrationSmokeWorkflow) {
        ""
    } else {
        ',{"path":".github/workflows/db-migrations.yml","owner":"gongzzang","classification":"schema_migration_smoke"}'
    }
    $nextActionsDocRules = if ($OmitNextActionsDocRule) {
        ""
    } else {
        @'
,
    {"path":"docs/superpowers/next-actions.md","token":"crates/data-clients/data-go-kr","reason":"active next-actions must not recreate Platform Core Catalog clients","exit_criteria":"next-actions points to Platform Core contract consumption only"},
    {"path":"docs/superpowers/next-actions.md","token":"crates/data-clients/r2-public-data","reason":"active next-actions must not recreate Platform Core public data readers","exit_criteria":"next-actions points to Platform Core vector tile lifecycle ownership"},
    {"path":"docs/superpowers/next-actions.md","token":"raw_capture(source","reason":"active next-actions must not recreate Platform Core raw capture flows","exit_criteria":"next-actions scopes raw lineage to Platform Core or approved Gongzzang-owned adapters"},
    {"path":"docs/superpowers/next-actions.md","token":"services/scraper-py/dtmk_vworld.py","reason":"active next-actions must not recreate extracted scraper service paths","exit_criteria":"next-actions points to Platform Core extraction handoff instead"}
'@
    }
    $roadmapSectionRules = if ($OmitRoadmapSectionRule) {
        ""
    } else {
        @'
,
    {"path":"docs/superpowers/roadmap.md","section_start":"## 다음 sub-project","section_end":"## 추천 순서","token":"SP4-iii — data.go.kr","reason":"active roadmap must not recommend Gongzzang-owned Catalog source client work","exit_criteria":"roadmap active section points to Platform Core contract consumption and DB cleanup only"},
    {"path":"docs/superpowers/roadmap.md","section_start":"## 다음 sub-project","section_end":"## 추천 순서","token":"R2 Reader 6","reason":"active roadmap must not recommend Gongzzang-owned public data readers","exit_criteria":"roadmap active section points to Platform Core vector tile ownership"},
    {"path":"docs/superpowers/roadmap.md","section_start":"## 추천 순서","section_end":"## Production","token":"SP4-iii-b 실거래가","reason":"recommended order must not route Catalog source work back into Gongzzang","exit_criteria":"recommended order starts with boundary verification or Gongzzang-owned product work"}
'@
    }
    return @"
{
  "schema_version": "gongzzang.platform_core_boundary.v1",
  "repo_slug": "gongzzang",
  "phase": "m3_2_physical_extraction_enforced",
  "allowed_integration_contracts": [
    {"kind":"http_api","direction":"gongzzang_to_platform_core"},
    {"kind":"runtime_vector_tile_manifest","direction":"gongzzang_to_platform_core"},
    {"kind":"outbox_webhook_event","direction":"platform_core_to_gongzzang"},
    {"kind":"immutable_anchor_artifact","direction":"gongzzang_to_platform_core"},
    {"kind":"lakehouse_registry_registration","direction":"gongzzang_to_platform_core"}
  ],
  "forbidden_integration_contracts": [
    {"kind":"direct_platform_core_database"},
    {"kind":"platform_core_listing_semantics"},
    {"kind":"gongzzang_canonical_catalog_write"},
    {"kind":"listing_owned_marker_coordinates"}
  ],
  "forbidden_canonical_catalog_tables": [
    {"table":"industrial_complex","owner":"platform-core","reason":"Canonical industrial complex facts belong to Platform Core Catalog"},
    {"table":"parcel","owner":"platform-core","reason":"Canonical parcel facts belong to Platform Core Catalog"},
    {"table":"building","owner":"platform-core","reason":"Canonical building register facts belong to Platform Core Catalog"},
    {"table":"manufacturer","owner":"platform-core","reason":"Canonical manufacturer master data belongs to Platform Core Catalog"}
  ],
  "root_env_example_contract": {
    "required_http_env": [
      "PLATFORM_CORE_API_BASE_URL",
      "NEXT_PUBLIC_PLATFORM_CORE_BASE_URL"
    ]$serviceAuthEnvContract,
    "forbidden_env": [
      "VWORLD_*",
      "ODP_SERVICE_KEY",
      "DATA_GO_KR_*",
      "ETL_*",
      "R2_<ENV>_*",
      "R2_*",
      "GEMINI_API_KEY"
    ],
    "reason": "Gongzzang root env examples must expose Platform Core through HTTP contracts only"
  },
  "path_ownership": [
    {"path":"crates/domain/core/listing","owner":"$listingOwner","classification":"product_domain"},
    {"path":"crates/domain/core/listing-photo","owner":"gongzzang","classification":"product_domain"},
    {"path":"crates/domain/core/user","owner":"gongzzang","classification":"product_domain"},
    {"path":"crates/domain/market","owner":"gongzzang","classification":"product_domain"},
    {"path":"crates/domain/insights","owner":"gongzzang","classification":"product_domain"},
    {"path":"crates/domain/core/industrial-complex","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/parcel","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/building","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/manufacturer","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/data-clients/vworld","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"crates/data-clients/data-go-kr","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"crates/data-clients/raw-capture","owner":"platform-core","classification":"extracted_catalog_raw_asset"},
    {"path":"crates/data-clients/r2-public-data","owner":"platform-core","classification":"extracted_vector_tile_data_asset"},
    {"path":"crates/data-pipeline-control","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"services/data-pipeline","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"services/scraper-py","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"services/etl-base-layer","owner":"gongzzang","classification":"platform_core_handover_stub"},
    {"path":"crates/sp9-base-layer-config","owner":"platform-core","classification":"extracted_vector_tile_config_asset"},
    {"path":".github/workflows/sp9-base-layer-etl.yml","owner":"platform-core","classification":"extracted_vector_tile_workflow_asset"},
    {"path":".github/workflows/sp9-base-layer-validation.yml","owner":"platform-core","classification":"extracted_vector_tile_workflow_asset"},
    {"path":".github/workflows/sp9-base-layer-cleanup.yml","owner":"platform-core","classification":"extracted_vector_tile_workflow_asset"},
    {"path":".github/workflows/sp9-base-layer-rollback.yml","owner":"platform-core","classification":"extracted_vector_tile_workflow_asset"},
    {"path":".github/workflows/sp9-manifest-backup-cleanup.yml","owner":"platform-core","classification":"extracted_vector_tile_workflow_asset"},
    {"path":"scripts/setup-dev-tippecanoe.sh","owner":"platform-core","classification":"extracted_vector_tile_tooling_asset"},
    {"path":"services/etl-base-layer/Dockerfile.etl","owner":"platform-core","classification":"extracted_vector_tile_tooling_asset"},
    {"path":"services/etl-base-layer/scripts","owner":"platform-core","classification":"extracted_vector_tile_tooling_asset"},
    {"path":".github/workflows/api-drift-smoke-test.yml","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"docs/observability/api-drift-smoke-test.md","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"crates/operations/api-health","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"crates/api-health-recorder","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"crates/db/src/api_health.rs","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"crates/db/tests/api_health_integration.rs","owner":"platform-core","classification":"extracted_catalog_observability_asset"},
    {"path":"crates/domain/core/shared-kernel/src/catalog_event.rs","owner":"platform-core","classification":"extracted_catalog_event_schema_asset"},
    {"path":"migrations/30012_parcel_marker_anchor_projection.sql","owner":"platform-core","classification":"gongzzang_read_model_copy"},
    {"path":"migrations/30013_listing_marker_projection.sql","owner":"gongzzang","classification":"serving_projection"},
    {"path":"migrations/30015_drop_platform_core_legacy_schema.sql","owner":"gongzzang","classification":"platform_core_legacy_schema_cleanup"},
    {"path":"services/api/src/routes/listing_marker_tiles.rs","owner":"gongzzang","classification":"product_marker_serving"},
    {"path":"apps/web/app/platform-core/events/route.ts","owner":"gongzzang","classification":"platform_core_event_receiver"}$migrationSmokeWorkflowEntry$anchorImporterEntry$anchorInboxMigrationEntry$anchorDbImportEntry$rustEventReceiverEntry$anchorImporterBinEntry$webhookPinEntry$catalogApiPinEntry
  ],
  "required_ci_gates": [
    "scripts/ci/check-platform-core-boundary.ps1",
    "scripts/lefthook/catalog-m1-boundary.sh",
    "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1"$eventReceiverGateEntry$catalogApiGateEntry$dependencyGateEntry
  ],
  "allowed_legacy_schema_tokens": [
    {"token":"pipeline_schedule","path":"migrations/10004_pipeline_tables.sql","owner":"platform-core","reason":"historical legacy ETL schema creation remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"pipeline_run","path":"migrations/10004_pipeline_tables.sql","owner":"platform-core","reason":"historical legacy ETL schema creation remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"force_pipeline_run","path":"migrations/10005_operations_tables.sql","owner":"platform-core","reason":"legacy ETL admin action example remains in historical migration comment","exit_criteria":"historical comment only; new runtime/code usage remains forbidden"},
    {"token":"parcel_external_data","path":"migrations/30006_parcel_external_data.sql","owner":"platform-core","reason":"historical legacy Catalog raw table creation remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"parcel_external_data","path":"migrations/30010_parcel_external_data_r2_pointer.sql","owner":"platform-core","reason":"historical legacy Catalog raw table alteration remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"parcel_external_data","path":"migrations/30011_parcel_external_data_r2_key_idx.sql","owner":"platform-core","reason":"historical legacy Catalog raw table index remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"api_health_check","path":"migrations/30007_api_health_check.sql","owner":"platform-core","reason":"historical legacy Catalog API drift health schema remains in immutable migration history","exit_criteria":"migrations/30015_drop_platform_core_legacy_schema.sql drops the Gongzzang DB runtime table"},
    {"token":"pipeline_schedule","path":"migrations/30015_drop_platform_core_legacy_schema.sql","owner":"platform-core","reason":"approved Gongzzang DB cleanup migration drops legacy Platform Core ETL table","exit_criteria":"immutable migration history proves legacy ETL table cleanup"},
    {"token":"pipeline_run","path":"migrations/30015_drop_platform_core_legacy_schema.sql","owner":"platform-core","reason":"approved Gongzzang DB cleanup migration drops legacy Platform Core ETL run table","exit_criteria":"immutable migration history proves legacy ETL run cleanup"},
    {"token":"parcel_external_data","path":"migrations/30015_drop_platform_core_legacy_schema.sql","owner":"platform-core","reason":"approved Gongzzang DB cleanup migration drops legacy Platform Core Catalog raw table","exit_criteria":"immutable migration history proves legacy Catalog raw cleanup"},
    {"token":"api_health_check","path":"migrations/30015_drop_platform_core_legacy_schema.sql","owner":"platform-core","reason":"approved Gongzzang DB cleanup migration drops legacy Platform Core API drift table","exit_criteria":"immutable migration history proves legacy API drift cleanup"}
  ],
  "forbidden_code_tokens": [
    "PLATFORM_CORE_DATABASE_URL",
    "postgres://platform_core",
    "platform-core-db",
    "platform_core_dev_2026",
    "parcel_external_data",
    "pipeline_schedule",
    "pipeline_run",
    "force_pipeline_run",
    "api_health_check",
    "api-health-recorder",
    "api-drift-smoke-test",
    "PgRawCapture",
    "R2RawCapture",
    "raw_capture_sync",
    "api.vworld.kr",
    "apis.data.go.kr",
    "dtmk_vworld.py",
    "vworld-cache-refresh",
    "building-register-sync"
  ],
  "forbidden_active_documentation_tokens": [
    {"path":"AGENTS.md","token":"raw_response JSONB","reason":"root rules must not imply Gongzzang stores Catalog raw responses locally","exit_criteria":"AGENTS.md scopes raw lineage to Gongzzang-owned external adapters and Platform Core-owned Catalog"},
    {"path":"docs/conventions/rust.md","token":"crates/data-clients/*","reason":"blanket data-client layer rule reintroduces Catalog client ownership","exit_criteria":"active Rust conventions distinguish Platform Core Catalog adapters from Gongzzang-owned clients"},
    {"path":"docs/conventions/testing.md","token":"crates/data-clients/*","reason":"blanket data-client coverage row reintroduces Catalog client ownership","exit_criteria":"active testing conventions distinguish db coverage from Gongzzang-owned external adapters"},
    {"path":"crates/data-clients/README.md","token":"vworld/","reason":"V-World client crate is Platform Core-owned after extraction","exit_criteria":"README says Catalog clients must not be recreated in Gongzzang"},
    {"path":"crates/data-clients/README.md","token":"data-go-kr/","reason":"Catalog data.go.kr client crate is Platform Core-owned after extraction","exit_criteria":"README says Catalog clients must not be recreated in Gongzzang"},
    {"path":"crates/data-clients/README.md","token":"raw_response *","reason":"Catalog raw response persistence is Platform Core-owned after extraction","exit_criteria":"README scopes raw lineage to the owning service or an approved Gongzzang adapter ADR"}$nextActionsDocRules
  ],
  "forbidden_active_documentation_section_tokens": [
    {"path":"docs/superpowers/roadmap.md","section_start":"## sentinel","section_end":"## end","token":"sentinel-token","reason":"placeholder keeps array shape stable for fixture generation","exit_criteria":"real required rules are appended by fixture options"}$roadmapSectionRules
  ]
}
"@
}

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
- raw 응답 보존 (`raw_response JSONB` 컬럼)
- 공공 API raw → DB `raw_response JSONB`
'@
        Write-File -Root $Root -RelativePath "docs\conventions\rust.md" -Content "crates/data-clients/* → crates/{circuit-breaker, observability, api-types}"
        Write-File -Root $Root -RelativePath "docs\conventions\testing.md" -Content '| `crates/data-clients/*`, `crates/db` | 70% |'
        Write-File -Root $Root -RelativePath "crates\data-clients\README.md" -Content @'
# crates/data-clients
- `vworld/` — V-World API
- `data-go-kr/` — 공공데이터포털
- raw_response *항상* 보존 (DB JSONB 컬럼)
'@
    }
    if ($CreateStaleNextActions) {
        Write-File -Root $Root -RelativePath "docs\superpowers\next-actions.md" -Content @'
# Next Actions
- 신규 파일: crates/data-clients/data-go-kr/src/real_transaction/client.rs
- 신규 crate: crates/data-clients/r2-public-data/
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
