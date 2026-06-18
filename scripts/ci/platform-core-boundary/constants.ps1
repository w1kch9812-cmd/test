$ExpectedSchemaVersion = "gongzzang.platform_core_boundary.v1"
$BoundaryRelativePath = "docs/architecture/platform-core-boundary.v1.json"
$RequiredContracts = @(
    "http_api:gongzzang_to_platform_core",
    "runtime_vector_tile_manifest:gongzzang_to_platform_core",
    "outbox_webhook_event:platform_core_to_gongzzang",
    "immutable_anchor_artifact:gongzzang_to_platform_core",
    "lakehouse_registry_registration:gongzzang_to_platform_core"
)
$RequiredCiGates = @(
    "scripts/ci/check-platform-core-boundary.ps1",
    "scripts/lefthook/catalog-m1-boundary.sh",
    "scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1",
    "scripts/ci/check-platform-core-event-receiver-contract.ps1",
    "scripts/ci/check-platform-core-catalog-api-contract.ps1",
    "scripts/ci/check-platform-core-dependency-boundary.ps1"
)
$RequiredPathOwnership = @(
    @{ Path = "crates/domain/core/listing"; Owner = "gongzzang"; Classification = "product_domain" },
    @{ Path = "crates/domain/core/listing-photo"; Owner = "gongzzang"; Classification = "product_domain" },
    @{ Path = "crates/domain/core/user"; Owner = "gongzzang"; Classification = "product_domain" },
    @{ Path = "crates/domain/market"; Owner = "gongzzang"; Classification = "product_domain" },
    @{ Path = "crates/domain/insights"; Owner = "gongzzang"; Classification = "product_domain" },
    @{ Path = "crates/domain/core/industrial-complex"; Owner = "platform-core"; Classification = "extracted_catalog_asset" },
    @{ Path = "crates/domain/core/parcel"; Owner = "platform-core"; Classification = "extracted_catalog_asset" },
    @{ Path = "crates/domain/core/building"; Owner = "platform-core"; Classification = "extracted_catalog_asset" },
    @{ Path = "crates/domain/core/manufacturer"; Owner = "platform-core"; Classification = "extracted_catalog_asset" },
    @{ Path = "crates/data-clients/vworld"; Owner = "platform-core"; Classification = "extracted_catalog_etl_asset" },
    @{ Path = "crates/data-clients/data-go-kr"; Owner = "platform-core"; Classification = "extracted_catalog_etl_asset" },
    @{ Path = "crates/data-clients/raw-capture"; Owner = "platform-core"; Classification = "extracted_catalog_raw_asset" },
    @{ Path = "crates/data-clients/r2-public-data"; Owner = "platform-core"; Classification = "extracted_vector_tile_data_asset" },
    @{ Path = "crates/data-pipeline-control"; Owner = "platform-core"; Classification = "extracted_catalog_etl_asset" },
    @{ Path = "services/data-pipeline"; Owner = "platform-core"; Classification = "extracted_catalog_etl_asset" },
    @{ Path = "services/scraper-py"; Owner = "platform-core"; Classification = "extracted_catalog_etl_asset" },
    @{ Path = "services/etl-base-layer"; Owner = "gongzzang"; Classification = "platform_core_handover_stub" },
    @{ Path = "crates/sp9-base-layer-config"; Owner = "platform-core"; Classification = "extracted_vector_tile_config_asset" },
    @{ Path = ".github/workflows/sp9-base-layer-etl.yml"; Owner = "platform-core"; Classification = "extracted_vector_tile_workflow_asset" },
    @{ Path = ".github/workflows/sp9-base-layer-validation.yml"; Owner = "platform-core"; Classification = "extracted_vector_tile_workflow_asset" },
    @{ Path = ".github/workflows/sp9-base-layer-cleanup.yml"; Owner = "platform-core"; Classification = "extracted_vector_tile_workflow_asset" },
    @{ Path = ".github/workflows/sp9-base-layer-rollback.yml"; Owner = "platform-core"; Classification = "extracted_vector_tile_workflow_asset" },
    @{ Path = ".github/workflows/sp9-manifest-backup-cleanup.yml"; Owner = "platform-core"; Classification = "extracted_vector_tile_workflow_asset" },
    @{ Path = "scripts/setup-dev-tippecanoe.sh"; Owner = "platform-core"; Classification = "extracted_vector_tile_tooling_asset" },
    @{ Path = "services/etl-base-layer/Dockerfile.etl"; Owner = "platform-core"; Classification = "extracted_vector_tile_tooling_asset" },
    @{ Path = "services/etl-base-layer/scripts"; Owner = "platform-core"; Classification = "extracted_vector_tile_tooling_asset" },
    @{ Path = ".github/workflows/api-drift-smoke-test.yml"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "docs/observability/api-drift-smoke-test.md"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "crates/operations/api-health"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "crates/api-health-recorder"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "crates/db/src/api_health.rs"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "crates/db/tests/api_health_integration.rs"; Owner = "platform-core"; Classification = "extracted_catalog_observability_asset" },
    @{ Path = "crates/domain/core/shared-kernel/src/catalog_event.rs"; Owner = "platform-core"; Classification = "extracted_catalog_event_schema_asset" },
    @{ Path = "migrations/30012_parcel_marker_anchor_projection.sql"; Owner = "platform-core"; Classification = "gongzzang_read_model_copy" },
    @{ Path = "migrations/30013_listing_marker_projection.sql"; Owner = "gongzzang"; Classification = "serving_projection" },
    @{ Path = "migrations/30015_drop_platform_core_legacy_schema.sql"; Owner = "gongzzang"; Classification = "platform_core_legacy_schema_cleanup" },
    @{ Path = "migrations/30016_platform_core_event_inbox_anchor_import.sql"; Owner = "gongzzang"; Classification = "platform_core_event_inbox" },
    @{ Path = "crates/db/src/platform_core_anchor.rs"; Owner = "gongzzang"; Classification = "platform_core_read_model_import" },
    @{ Path = "services/api/src/routes/listing_marker_tiles.rs"; Owner = "gongzzang"; Classification = "product_marker_serving" },
    @{ Path = "services/api/src/platform_core_anchor_import.rs"; Owner = "gongzzang"; Classification = "platform_core_read_model_import_contract" },
    @{ Path = "services/api/src/routes/platform_core_events.rs"; Owner = "gongzzang"; Classification = "platform_core_event_receiver" },
    @{ Path = "services/api/src/bin/platform_core_anchor_import.rs"; Owner = "gongzzang"; Classification = "platform_core_read_model_importer" },
    @{ Path = "apps/web/app/platform-core/events/route.ts"; Owner = "gongzzang"; Classification = "platform_core_event_receiver" },
    @{ Path = ".github/workflows/db-migrations.yml"; Owner = "gongzzang"; Classification = "schema_migration_smoke" },
    @{ Path = "docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json"; Owner = "platform-core"; Classification = "contract_pin_copy" },
    @{ Path = "docs/architecture/platform-core-catalog-api-contract.v1.pin.json"; Owner = "platform-core"; Classification = "contract_pin_copy" }
)
$RequiredForbiddenContracts = @(
    "direct_platform_core_database",
    "platform_core_listing_semantics",
    "gongzzang_canonical_catalog_write",
    "listing_owned_marker_coordinates"
)
$RequiredForbiddenCanonicalCatalogTables = @(
    "industrial_complex",
    "parcel",
    "building",
    "manufacturer"
)
$RequiredCleanupMigrationPath = "migrations/30015_drop_platform_core_legacy_schema.sql"
$RequiredMigrationSmokePath = "tests/migrations/test_v001_full.sh"
$RequiredMigrationSmokeWorkflowPath = ".github/workflows/db-migrations.yml"
$RequiredMigrationSmokeBazelTarget = "//tools/bazel:ci_migration_v001_full_transition"
$RequiredCleanupMigrationDrops = @(
    "api_health_check",
    "parcel_external_data",
    "pipeline_run",
    "pipeline_schedule"
)
$RootEnvExamplePaths = @(
    ".env.example",
    ".env.local.example",
    "apps/web/.env.local.example",
    "services/api/.env.example"
)
$ForbiddenRootEnvExamplePatterns = @(
    @{ Pattern = "(?m)^\s*VWORLD_"; Token = "VWORLD_*" },
    @{ Pattern = "(?m)^\s*ODP_SERVICE_KEY\s*="; Token = "ODP_SERVICE_KEY" },
    @{ Pattern = "(?m)^\s*DATA_GO_KR_"; Token = "DATA_GO_KR_*" },
    @{ Pattern = "(?m)^\s*ETL_"; Token = "ETL_*" },
    @{ Pattern = "(?m)^\s*R2_(?:LOCAL|STAGING|PRODUCTION)_"; Token = "R2_<ENV>_*" },
    @{ Pattern = "(?m)^\s*R2_(?:ACCOUNT_ID|ACCESS_KEY|SECRET_KEY|BUCKET)\s*="; Token = "R2_*" },
    @{ Pattern = "(?m)^\s*GEMINI_API_KEY\s*="; Token = "GEMINI_API_KEY" }
)
$RequiredForbiddenActiveDocumentationTokens = @(
    @{ Path = "AGENTS.md"; Token = "raw_response JSONB" },
    @{ Path = "docs/conventions/rust.md"; Token = "crates/data-clients/*" },
    @{ Path = "docs/conventions/testing.md"; Token = "crates/data-clients/*" },
    @{ Path = "crates/data-clients/README.md"; Token = "vworld/" },
    @{ Path = "crates/data-clients/README.md"; Token = "data-go-kr/" },
    @{ Path = "crates/data-clients/README.md"; Token = "raw_response *" },
    @{ Path = "docs/superpowers/next-actions.md"; Token = "crates/data-clients/data-go-kr" },
    @{ Path = "docs/superpowers/next-actions.md"; Token = "crates/data-clients/r2-public-data" },
    @{ Path = "docs/superpowers/next-actions.md"; Token = "raw_capture(source" },
    @{ Path = "docs/superpowers/next-actions.md"; Token = "services/scraper-py/dtmk_vworld.py" }
)
$RequiredForbiddenActiveDocumentationSectionTokens = @(
    @{ Path = "docs/superpowers/roadmap.md"; SectionStart = "## 다음 sub-project"; SectionEnd = "## 추천 순서"; Token = "SP4-iii — data.go.kr" },
    @{ Path = "docs/superpowers/roadmap.md"; SectionStart = "## 다음 sub-project"; SectionEnd = "## 추천 순서"; Token = "R2 Reader 6" },
    @{ Path = "docs/superpowers/roadmap.md"; SectionStart = "## 추천 순서"; SectionEnd = "## Production"; Token = "SP4-iii-b 실거래가" }
)
