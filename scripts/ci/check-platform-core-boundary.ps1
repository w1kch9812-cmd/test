param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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
        throw "platform-core-boundary: missing array '$Name'"
    }
    return @($value)
}

function Get-RequiredString {
    param([object] $Object, [string] $Name)

    $value = [string] (Get-PropertyValue -Object $Object -Name $Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "platform-core-boundary: missing string '$Name'"
    }
    return $value
}

function Normalize-RelativePath {
    param([string] $Path)

    $normalized = $Path -replace "\\", "/"
    if ($normalized.StartsWith("./", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(2)
    }
    while ($normalized.StartsWith("/", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(1)
    }
    return $normalized
}

function Resolve-RepoPath {
    param([string] $RootPath, [string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $RootPath $RelativePath))
}

function Assert-Equal {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "platform-core-boundary: $Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-PathExists {
    param([string] $RootPath, [string] $RelativePath)

    $path = Resolve-RepoPath -RootPath $RootPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path)) {
        throw "platform-core-boundary: required path is missing: $RelativePath"
    }
}

function Assert-PathAbsent {
    param([string] $RootPath, [string] $RelativePath)

    $path = Resolve-RepoPath -RootPath $RootPath -RelativePath $RelativePath
    if (Test-Path -LiteralPath $path) {
        throw "platform-core-boundary: extracted Platform Core path must be absent: $RelativePath"
    }
}

function Assert-RequiredPathOwnership {
    param([object[]] $Entries)

    $paths = @($Entries | ForEach-Object { [string] (Get-PropertyValue -Object $_ -Name "path") })
    $duplicates = @($paths | Group-Object | Where-Object { $_.Count -gt 1 })
    if ($duplicates.Count -gt 0) {
        throw "platform-core-boundary: duplicate path ownership entries: $((@($duplicates | ForEach-Object { $_.Name })) -join ', ')"
    }

    foreach ($required in $RequiredPathOwnership) {
        $entry = @($Entries | Where-Object { [string] (Get-PropertyValue -Object $_ -Name "path") -eq $required.Path })
        if ($entry.Count -ne 1) {
            throw "platform-core-boundary: missing path ownership entry: $($required.Path)"
        }
        Assert-Equal `
            -Actual ([string] (Get-PropertyValue -Object $entry[0] -Name "owner")) `
            -Expected $required.Owner `
            -Message "owner mismatch for $($required.Path)"
        Assert-Equal `
            -Actual ([string] (Get-PropertyValue -Object $entry[0] -Name "classification")) `
            -Expected $required.Classification `
            -Message "classification mismatch for $($required.Path)"
    }
}

function Assert-RequiredContracts {
    param([object[]] $Contracts)

    $actual = @($Contracts | ForEach-Object {
        "$([string] (Get-PropertyValue -Object $_ -Name "kind")):$([string] (Get-PropertyValue -Object $_ -Name "direction"))"
    })
    foreach ($required in $RequiredContracts) {
        if (!($actual -contains $required)) {
            throw "platform-core-boundary: missing allowed integration contract: $required"
        }
    }
}

function Assert-ForbiddenContracts {
    param([object[]] $Contracts)

    $actual = @($Contracts | ForEach-Object { [string] (Get-PropertyValue -Object $_ -Name "kind") })
    foreach ($required in $RequiredForbiddenContracts) {
        if (!($actual -contains $required)) {
            throw "platform-core-boundary: missing forbidden integration contract: $required"
        }
    }
}

function Assert-ForbiddenCanonicalCatalogTables {
    param([object[]] $Tables)

    $actual = @($Tables | ForEach-Object { [string] (Get-PropertyValue -Object $_ -Name "table") })
    foreach ($required in $RequiredForbiddenCanonicalCatalogTables) {
        if (!($actual -contains $required)) {
            throw "platform-core-boundary: missing forbidden canonical Catalog table entry: $required"
        }
    }

    $seen = New-Object System.Collections.Generic.HashSet[string]
    foreach ($entry in $Tables) {
        $table = Get-RequiredString -Object $entry -Name "table"
        $owner = Get-RequiredString -Object $entry -Name "owner"
        $reason = Get-RequiredString -Object $entry -Name "reason"

        if (!$seen.Add($table)) {
            throw "platform-core-boundary: duplicate forbidden canonical Catalog table entry: $table"
        }
        if ($owner -ne "platform-core") {
            throw "platform-core-boundary: forbidden canonical Catalog table owner must be platform-core: $table"
        }
        if ($reason.Length -lt 16) {
            throw "platform-core-boundary: forbidden canonical Catalog table reason is too weak: $table"
        }
    }
}

function Assert-RootEnvExampleContractDefinition {
    param([object] $Contract)

    if ($null -eq $Contract) {
        throw "platform-core-boundary: missing root_env_example_contract"
    }

    $requiredHttpEnv = @(Get-RequiredArray -Object $Contract -Name "required_http_env" | ForEach-Object { [string] $_ })
    foreach ($required in @("PLATFORM_CORE_API_BASE_URL", "NEXT_PUBLIC_PLATFORM_CORE_BASE_URL")) {
        if (!($requiredHttpEnv -contains $required)) {
            throw "platform-core-boundary: root_env_example_contract missing required HTTP env: $required"
        }
    }

    $requiredServiceAuthEnv = @(Get-RequiredArray -Object $Contract -Name "required_service_auth_env" | ForEach-Object { [string] $_ })
    foreach ($required in @("PLATFORM_CORE_SERVICE_TOKEN", "PLATFORM_CORE_WEBHOOK_SECRET")) {
        if (!($requiredServiceAuthEnv -contains $required)) {
            throw "platform-core-boundary: root_env_example_contract missing required service auth env: $required"
        }
    }

    $forbiddenEnv = @(Get-RequiredArray -Object $Contract -Name "forbidden_env" | ForEach-Object { [string] $_ })
    foreach ($required in @("VWORLD_*", "ODP_SERVICE_KEY", "DATA_GO_KR_*", "ETL_*", "R2_<ENV>_*", "R2_*", "GEMINI_API_KEY")) {
        if (!($forbiddenEnv -contains $required)) {
            throw "platform-core-boundary: root_env_example_contract missing forbidden env: $required"
        }
    }
}

function Assert-RequiredCiGates {
    param([string[]] $Gates)

    foreach ($required in $RequiredCiGates) {
        if (!($Gates -contains $required)) {
            throw "platform-core-boundary: missing required CI gate: $required"
        }
    }
}

function Assert-ForbiddenActiveDocumentationTokens {
    param([string] $RootPath, [object[]] $Rules)

    $ruleKeys = New-Object System.Collections.Generic.HashSet[string]
    foreach ($rule in $Rules) {
        $path = Normalize-RelativePath -Path (Get-RequiredString -Object $rule -Name "path")
        $token = Get-RequiredString -Object $rule -Name "token"
        $reason = Get-RequiredString -Object $rule -Name "reason"
        $exitCriteria = Get-RequiredString -Object $rule -Name "exit_criteria"

        if ($reason.Length -lt 16) {
            throw "platform-core-boundary: forbidden_active_documentation_tokens reason is too weak: $path contains $token"
        }
        if ($exitCriteria.Length -lt 16) {
            throw "platform-core-boundary: forbidden_active_documentation_tokens exit_criteria is too weak: $path contains $token"
        }

        if (!$ruleKeys.Add("$path::$token")) {
            throw "platform-core-boundary: duplicate forbidden_active_documentation_tokens entry: $path contains $token"
        }
    }

    foreach ($required in $RequiredForbiddenActiveDocumentationTokens) {
        $path = [string] $required.Path
        $token = [string] $required.Token
        if (!$ruleKeys.Contains("$path::$token")) {
            throw "platform-core-boundary: missing forbidden_active_documentation_tokens entry: $path contains $token"
        }
    }

    foreach ($rule in $Rules) {
        $path = Normalize-RelativePath -Path (Get-RequiredString -Object $rule -Name "path")
        $token = Get-RequiredString -Object $rule -Name "token"
        $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $path
        if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "platform-core-boundary: forbidden active documentation path is missing: $path"
        }

        $content = Get-Content -LiteralPath $fullPath -Raw
        if ($content.Contains($token)) {
            throw "platform-core-boundary: forbidden active documentation token '$token' in $path"
        }
    }
}

function Get-ActiveDocumentationSection {
    param(
        [string] $Content,
        [string] $SectionStart,
        [string] $SectionEnd,
        [string] $Path
    )

    $startIndex = $Content.IndexOf($SectionStart, [System.StringComparison]::Ordinal)
    if ($startIndex -lt 0) {
        throw "platform-core-boundary: active documentation section start is missing: $Path contains $SectionStart"
    }

    $searchStart = $startIndex + $SectionStart.Length
    $endIndex = if ([string]::IsNullOrWhiteSpace($SectionEnd)) {
        -1
    } else {
        $Content.IndexOf($SectionEnd, $searchStart, [System.StringComparison]::Ordinal)
    }

    if ($endIndex -lt 0) {
        return $Content.Substring($startIndex)
    }

    return $Content.Substring($startIndex, $endIndex - $startIndex)
}

function Assert-ForbiddenActiveDocumentationSectionTokens {
    param([string] $RootPath, [object[]] $Rules)

    $ruleKeys = New-Object System.Collections.Generic.HashSet[string]
    foreach ($rule in $Rules) {
        $path = Normalize-RelativePath -Path (Get-RequiredString -Object $rule -Name "path")
        $sectionStart = Get-RequiredString -Object $rule -Name "section_start"
        $sectionEnd = Get-RequiredString -Object $rule -Name "section_end"
        $token = Get-RequiredString -Object $rule -Name "token"
        $reason = Get-RequiredString -Object $rule -Name "reason"
        $exitCriteria = Get-RequiredString -Object $rule -Name "exit_criteria"

        if ($reason.Length -lt 16) {
            throw "platform-core-boundary: forbidden_active_documentation_section_tokens reason is too weak: $path contains $token"
        }
        if ($exitCriteria.Length -lt 16) {
            throw "platform-core-boundary: forbidden_active_documentation_section_tokens exit_criteria is too weak: $path contains $token"
        }

        if (!$ruleKeys.Add("$path::$sectionStart::$sectionEnd::$token")) {
            throw "platform-core-boundary: duplicate forbidden_active_documentation_section_tokens entry: $path contains $token"
        }
    }

    foreach ($required in $RequiredForbiddenActiveDocumentationSectionTokens) {
        $path = [string] $required.Path
        $sectionStart = [string] $required.SectionStart
        $sectionEnd = [string] $required.SectionEnd
        $token = [string] $required.Token
        if (!$ruleKeys.Contains("$path::$sectionStart::$sectionEnd::$token")) {
            throw "platform-core-boundary: missing forbidden_active_documentation_section_tokens entry: $path contains $token"
        }
    }

    foreach ($rule in $Rules) {
        $path = Normalize-RelativePath -Path (Get-RequiredString -Object $rule -Name "path")
        $sectionStart = Get-RequiredString -Object $rule -Name "section_start"
        $sectionEnd = Get-RequiredString -Object $rule -Name "section_end"
        $token = Get-RequiredString -Object $rule -Name "token"
        $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $path
        if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "platform-core-boundary: forbidden active documentation section path is missing: $path"
        }

        $content = Get-Content -LiteralPath $fullPath -Raw
        $section = Get-ActiveDocumentationSection `
            -Content $content `
            -SectionStart $sectionStart `
            -SectionEnd $sectionEnd `
            -Path $path
        if ($section.Contains($token)) {
            throw "platform-core-boundary: forbidden active documentation section token '$token' in $path"
        }
    }
}

function Assert-CiGateWiring {
    param([string] $RootPath, [string[]] $Gates)

    foreach ($gate in $Gates) {
        Assert-PathExists -RootPath $RootPath -RelativePath $gate
    }

    $ciPath = Resolve-RepoPath -RootPath $RootPath -RelativePath ".github/workflows/ci.yml"
    if (!(Test-Path -LiteralPath $ciPath)) {
        throw "platform-core-boundary: CI workflow is missing"
    }
    $ci = Get-Content -LiteralPath $ciPath -Raw
    foreach ($gate in $RequiredCiGates) {
        $gateName = Split-Path -Leaf $gate
        if (!$ci.Contains($gateName)) {
            throw "platform-core-boundary: CI workflow must run $gateName"
        }
    }

    $lefthookPath = Resolve-RepoPath -RootPath $RootPath -RelativePath "lefthook.yml"
    if (!(Test-Path -LiteralPath $lefthookPath)) {
        throw "platform-core-boundary: lefthook.yml is missing"
    }
    $lefthook = Get-Content -LiteralPath $lefthookPath -Raw
    foreach ($gate in $RequiredCiGates) {
        $gateName = Split-Path -Leaf $gate
        if (!$lefthook.Contains($gateName)) {
            throw "platform-core-boundary: lefthook.yml must run $gateName"
        }
    }
}

function Assert-NoForbiddenCodeTokens {
    param([string] $RootPath, [string[]] $Tokens)

    $roots = @("apps", "services", "crates", "packages", ".github/workflows")
    $extensions = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md", ".sql", ".toml", ".env", ".yml", ".yaml")
    foreach ($root in $roots) {
        $scanRoot = Resolve-RepoPath -RootPath $RootPath -RelativePath $root
        if (!(Test-Path -LiteralPath $scanRoot)) {
            continue
        }

        foreach ($file in Get-ChildItem -LiteralPath $scanRoot -Recurse -File) {
            if (!($extensions -contains $file.Extension)) {
                continue
            }
            $content = Get-Content -LiteralPath $file.FullName -Raw
            foreach ($token in $Tokens) {
                if ([string]::IsNullOrWhiteSpace($token)) {
                    continue
                }
                if ($content.Contains($token)) {
                    $rootPrefix = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
                    $fullName = [System.IO.Path]::GetFullPath($file.FullName)
                    $relative = if ($fullName.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                        $fullName.Substring($rootPrefix.Length)
                    } else {
                        $fullName
                    }
                    $relative = Normalize-RelativePath -Path $relative
                    throw "platform-core-boundary: forbidden direct Platform Core coupling token '$token' in $relative"
                }
            }
        }
    }
}

function Find-DirectPlatformCoreDatabaseReference {
    param([string] $Content)

    $patterns = @(
        "(?i)\bPLATFORM_CORE_(?:DATABASE|DB|POSTGRES|PG)_(?:URL|URI|DSN)\b",
        "(?i)\b(?:DATABASE|DB|POSTGRES|PG)_(?:URL|URI|DSN)_PLATFORM_CORE\b",
        "(?i)\bplatform[-_]?core[-_]?(?:database|db|postgres|pg)(?:[-_]?(?:url|uri|dsn))?\b",
        "(?i)\b(?:postgres|postgresql)://\S*platform[-_]?core\S*"
    )

    foreach ($pattern in $patterns) {
        $match = [regex]::Match($Content, $pattern)
        if ($match.Success) {
            return $match.Value
        }
    }
    return $null
}

function Assert-NoDirectPlatformCoreDatabaseReferences {
    param([string] $RootPath)

    $roots = @("apps", "services", "crates", "packages", ".github/workflows")
    $extensions = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md", ".sql", ".toml", ".env", ".yml", ".yaml")
    foreach ($root in $roots) {
        $scanRoot = Resolve-RepoPath -RootPath $RootPath -RelativePath $root
        if (!(Test-Path -LiteralPath $scanRoot)) {
            continue
        }

        foreach ($file in Get-ChildItem -LiteralPath $scanRoot -Recurse -File) {
            if (!($extensions -contains $file.Extension)) {
                continue
            }

            $content = Get-Content -LiteralPath $file.FullName -Raw
            $match = Find-DirectPlatformCoreDatabaseReference -Content $content
            if ($null -eq $match) {
                continue
            }

            $relative = Get-RepoRelativePath -RootPath $RootPath -FullPath $file.FullName
            throw "platform-core-boundary: direct Platform Core database reference '$match' in $relative"
        }
    }

    $rootConfigNames = @(
        "Cargo.toml",
        "package.json",
        "pnpm-workspace.yaml",
        "turbo.json",
        "docker-compose.yml",
        "compose.yml",
        "compose.yaml"
    )
    foreach ($file in Get-ChildItem -LiteralPath $RootPath -File -Force) {
        if (!($file.Name.StartsWith(".env", [System.StringComparison]::OrdinalIgnoreCase)) -and
            !($rootConfigNames -contains $file.Name)) {
            continue
        }

        $content = Get-Content -LiteralPath $file.FullName -Raw
        $match = Find-DirectPlatformCoreDatabaseReference -Content $content
        if ($null -eq $match) {
            continue
        }

        $relative = Get-RepoRelativePath -RootPath $RootPath -FullPath $file.FullName
        throw "platform-core-boundary: direct Platform Core database reference '$match' in $relative"
    }
}

function Assert-NoRootCatalogSourceEnvExamples {
    param([string] $RootPath)

    foreach ($relativePath in $RootEnvExamplePaths) {
        $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $relativePath
        if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }

        $content = Get-Content -LiteralPath $fullPath -Raw
        foreach ($rule in $ForbiddenRootEnvExamplePatterns) {
            $match = [regex]::Match($content, [string] $rule.Pattern)
            if ($match.Success) {
                throw "platform-core-boundary: root env example must not expose Platform Core-owned Catalog/ETL env '$($rule.Token)' in $relativePath"
            }
        }
    }
}

function Assert-RootEnvExamplePlatformCoreContract {
    param([string] $RootPath)

    $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath ".env.example"
    if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "platform-core-boundary: .env.example is missing"
    }

    $content = Get-Content -LiteralPath $fullPath -Raw
    foreach ($required in @("PLATFORM_CORE_API_BASE_URL=", "NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=", "PLATFORM_CORE_SERVICE_TOKEN=", "PLATFORM_CORE_WEBHOOK_SECRET=")) {
        if (!$content.Contains($required)) {
            throw "platform-core-boundary: .env.example must document Platform Core contract env '$required'"
        }
    }
}

function Assert-LocalGongzzangPostgresPortContract {
    param([string] $RootPath)

    $composePath = Resolve-RepoPath -RootPath $RootPath -RelativePath "infrastructure/docker/docker-compose.yml"
    if (!(Test-Path -LiteralPath $composePath -PathType Leaf)) {
        throw "platform-core-boundary: local Docker Compose file is missing"
    }
    $compose = Get-Content -LiteralPath $composePath -Raw
    if (!$compose.Contains('${POSTGRES_HOST_PORT:-15432}:5432')) {
        throw "platform-core-boundary: local Gongzzang Postgres must use POSTGRES_HOST_PORT default 15432, not Windows-reserved 5500"
    }

    $dockerEnvExamplePath = Resolve-RepoPath -RootPath $RootPath -RelativePath "infrastructure/docker/.env.example"
    if (!(Test-Path -LiteralPath $dockerEnvExamplePath -PathType Leaf)) {
        throw "platform-core-boundary: infrastructure/docker/.env.example is missing"
    }
    $dockerEnvExample = Get-Content -LiteralPath $dockerEnvExamplePath -Raw
    if (!$dockerEnvExample.Contains("POSTGRES_HOST_PORT=15432")) {
        throw "platform-core-boundary: infrastructure/docker/.env.example must set POSTGRES_HOST_PORT=15432"
    }

    $rootEnvExamplePath = Resolve-RepoPath -RootPath $RootPath -RelativePath ".env.example"
    $rootEnvExample = Get-Content -LiteralPath $rootEnvExamplePath -Raw
    if (!$rootEnvExample.Contains("@localhost:15432/gongzzang")) {
        throw "platform-core-boundary: .env.example DATABASE_URL must target local Gongzzang Postgres on port 15432"
    }
}

function Get-RepoRelativePath {
    param([string] $RootPath, [string] $FullPath)

    $rootPrefix = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    $resolvedPath = [System.IO.Path]::GetFullPath($FullPath)
    if ($resolvedPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return Normalize-RelativePath -Path ($resolvedPath.Substring($rootPrefix.Length))
    }
    return Normalize-RelativePath -Path $resolvedPath
}

function Test-ContentContainsCanonicalCatalogSqlUsage {
    param([string] $Content, [string] $Table)

    $quotedTable = '"?' + [regex]::Escape($Table) + '"?'
    $schemaPrefix = '(?:"?[A-Za-z_][A-Za-z0-9_]*"?\s*\.\s*)?'
    $tableRef = $schemaPrefix + $quotedTable
    $tableTerminator = '(?=$|[\s(,;])'
    $patterns = @(
        "(?im)\bcreate\s+table\s+(?:if\s+not\s+exists\s+)?$tableRef$tableTerminator",
        "(?im)\balter\s+table\s+$tableRef$tableTerminator",
        "(?im)\bdrop\s+table\s+(?:if\s+exists\s+)?$tableRef$tableTerminator",
        "(?im)\btruncate\s+(?:table\s+)?$tableRef$tableTerminator",
        "(?im)\binsert\s+into\s+$tableRef$tableTerminator",
        "(?im)\bupdate\s+$tableRef$tableTerminator",
        "(?im)\bdelete\s+from\s+$tableRef$tableTerminator",
        "(?im)\bfrom\s+$tableRef$tableTerminator",
        "(?im)\bjoin\s+$tableRef$tableTerminator",
        "(?im)\breferences\s+$tableRef$tableTerminator"
    )

    foreach ($pattern in $patterns) {
        if ([regex]::IsMatch($Content, $pattern)) {
            return $true
        }
    }
    return $false
}

function Assert-NoCanonicalCatalogTableSqlUsage {
    param([string] $RootPath)

    $roots = @("apps", "services", "crates", "packages", "migrations", ".github/workflows")
    $extensions = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md", ".sql", ".yml", ".yaml")
    foreach ($root in $roots) {
        $scanRoot = Resolve-RepoPath -RootPath $RootPath -RelativePath $root
        if (!(Test-Path -LiteralPath $scanRoot)) {
            continue
        }

        foreach ($file in Get-ChildItem -LiteralPath $scanRoot -Recurse -File) {
            if (!($extensions -contains $file.Extension)) {
                continue
            }

            $content = Get-Content -LiteralPath $file.FullName -Raw
            foreach ($table in $RequiredForbiddenCanonicalCatalogTables) {
                if (Test-ContentContainsCanonicalCatalogSqlUsage -Content $content -Table $table) {
                    $relative = Get-RepoRelativePath -RootPath $RootPath -FullPath $file.FullName
                    throw "platform-core-boundary: forbidden canonical Catalog table SQL usage '$table' in $relative"
                }
            }
        }
    }
}

function Test-ContentContainsLegacySchemaToken {
    param([string] $Content, [string] $Token)

    $pattern = "(?<![A-Za-z0-9_])" + [regex]::Escape($Token) + "(?![A-Za-z0-9_])"
    return [regex]::IsMatch($Content, $pattern)
}

function Assert-LegacySchemaTokenLedger {
    param([string] $RootPath, [object[]] $Allowances)

    $allowanceByToken = @{}
    $allowedPairs = New-Object System.Collections.Generic.HashSet[string]

    foreach ($allowance in $Allowances) {
        $token = Get-RequiredString -Object $allowance -Name "token"
        $path = Normalize-RelativePath -Path (Get-RequiredString -Object $allowance -Name "path")
        $owner = Get-RequiredString -Object $allowance -Name "owner"
        $reason = Get-RequiredString -Object $allowance -Name "reason"
        $exitCriteria = Get-RequiredString -Object $allowance -Name "exit_criteria"

        if ($owner -ne "platform-core") {
            throw "platform-core-boundary: allowed_legacy_schema_tokens owner must be platform-core: $path contains $token"
        }
        if (!$path.StartsWith("migrations/", [System.StringComparison]::OrdinalIgnoreCase) -or !$path.EndsWith(".sql", [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "platform-core-boundary: allowed_legacy_schema_tokens path must be a migration sql file: $path contains $token"
        }
        if ($reason.Length -lt 16) {
            throw "platform-core-boundary: allowed_legacy_schema_tokens reason is too weak: $path contains $token"
        }
        if ($exitCriteria.Length -lt 16) {
            throw "platform-core-boundary: allowed_legacy_schema_tokens exit_criteria is too weak: $path contains $token"
        }

        $key = "$token::$path"
        if (!$allowedPairs.Add($key)) {
            throw "platform-core-boundary: duplicate allowed_legacy_schema_tokens entry: $path contains $token"
        }
        if (!$allowanceByToken.ContainsKey($token)) {
            $allowanceByToken[$token] = @()
        }
        $allowanceByToken[$token] = @($allowanceByToken[$token]) + $path

        $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $path
        if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "platform-core-boundary: stale allowed_legacy_schema_tokens entry: missing file $path contains $token"
        }
        $content = Get-Content -LiteralPath $fullPath -Raw
        if (!(Test-ContentContainsLegacySchemaToken -Content $content -Token $token)) {
            throw "platform-core-boundary: stale allowed_legacy_schema_tokens entry: $path no longer contains $token"
        }
    }

    $migrationsRoot = Resolve-RepoPath -RootPath $RootPath -RelativePath "migrations"
    if (!(Test-Path -LiteralPath $migrationsRoot)) {
        return
    }

    foreach ($file in Get-ChildItem -LiteralPath $migrationsRoot -Filter "*.sql" -File) {
        $relative = Normalize-RelativePath -Path ("migrations/" + $file.Name)
        $content = Get-Content -LiteralPath $file.FullName -Raw
        foreach ($token in $allowanceByToken.Keys) {
            if (!(Test-ContentContainsLegacySchemaToken -Content $content -Token ([string] $token))) {
                continue
            }
            $allowedPaths = @($allowanceByToken[$token])
            if (!($allowedPaths -contains $relative)) {
                throw "platform-core-boundary: unapproved legacy schema token '$token' in $relative"
            }
        }
    }
}

function Assert-CleanupMigrationContract {
    param([string] $RootPath)

    $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $RequiredCleanupMigrationPath
    if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "platform-core-boundary: cleanup migration is missing: $RequiredCleanupMigrationPath"
    }

    $content = Get-Content -LiteralPath $fullPath -Raw
    if ([regex]::IsMatch($content, "(?i)\bcascade\b")) {
        throw "platform-core-boundary: cleanup migration must not use CASCADE: $RequiredCleanupMigrationPath"
    }

    $previousIndex = -1
    foreach ($table in $RequiredCleanupMigrationDrops) {
        $pattern = "(?im)^\s*drop\s+table\s+if\s+exists\s+" + [regex]::Escape($table) + "\s*;"
        $match = [regex]::Match($content, $pattern)
        if (!$match.Success) {
            throw "platform-core-boundary: cleanup migration must drop table with DROP TABLE IF EXISTS: $table"
        }
        if ($match.Index -lt $previousIndex) {
            throw "platform-core-boundary: cleanup migration table drop order mismatch: $table"
        }
        $previousIndex = $match.Index
    }
}

function Get-ShellArrayBlock {
    param([string] $Content, [string] $Name, [string] $Path)

    $pattern = "(?ms)^\s*" + [regex]::Escape($Name) + "\s*=\s*\((.*?)^\s*\)"
    $match = [regex]::Match($Content, $pattern)
    if (!$match.Success) {
        throw "platform-core-boundary: migration smoke array is missing: $Path contains $Name"
    }
    return $match.Groups[1].Value
}

function Assert-MigrationSmokeContract {
    param([string] $RootPath)

    $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $RequiredMigrationSmokePath
    if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "platform-core-boundary: migration smoke test is missing: $RequiredMigrationSmokePath"
    }

    $content = Get-Content -LiteralPath $fullPath -Raw
    $expectedTables = Get-ShellArrayBlock -Content $content -Name "EXPECTED_TABLES" -Path $RequiredMigrationSmokePath
    $forbiddenTables = Get-ShellArrayBlock -Content $content -Name "FORBIDDEN_TABLES" -Path $RequiredMigrationSmokePath
    $contentOutsideForbiddenTables = $content.Replace($forbiddenTables, "")

    foreach ($table in $RequiredCleanupMigrationDrops) {
        if (Test-ContentContainsLegacySchemaToken -Content $expectedTables -Token $table) {
            throw "platform-core-boundary: migration smoke must not expect dropped Platform Core legacy table: $table"
        }
        if (!(Test-ContentContainsLegacySchemaToken -Content $forbiddenTables -Token $table)) {
            throw "platform-core-boundary: migration smoke must assert Platform Core legacy table is absent: $table"
        }
        if (Test-ContentContainsLegacySchemaToken -Content $contentOutsideForbiddenTables -Token $table) {
            throw "platform-core-boundary: migration smoke must mention dropped Platform Core legacy table only in FORBIDDEN_TABLES: $table"
        }
    }
}

function Assert-MigrationSmokeWorkflowContract {
    param([string] $RootPath)

    $fullPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $RequiredMigrationSmokeWorkflowPath
    if (!(Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "platform-core-boundary: migration smoke workflow is missing: $RequiredMigrationSmokeWorkflowPath"
    }

    $content = Get-Content -LiteralPath $fullPath -Raw
    $requiredTokens = @(
        "postgis/postgis:17-3.5",
        "POSTGRES_DB: gongzzang",
        "DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang"
    )
    foreach ($token in $requiredTokens) {
        if (!$content.Contains($token)) {
            throw "platform-core-boundary: migration smoke workflow must contain '$token'"
        }
    }

    $runsLegacySmokeScript = [regex]::IsMatch($content, "(?im)^\s*-?\s*run:\s+bash\s+tests/migrations/test_v001_full\.sh\s*$")
    $runsBazelSmokeTarget = $content.Contains($RequiredMigrationSmokeBazelTarget)
    if (!$runsLegacySmokeScript -and !$runsBazelSmokeTarget) {
        throw "platform-core-boundary: migration smoke workflow must run $RequiredMigrationSmokeBazelTarget or bash tests/migrations/test_v001_full.sh"
    }
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$boundaryPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $BoundaryRelativePath
if (!(Test-Path -LiteralPath $boundaryPath)) {
    throw "platform-core-boundary: missing boundary SSOT: $BoundaryRelativePath"
}

$boundary = Get-Content -LiteralPath $boundaryPath -Raw | ConvertFrom-Json
Assert-Equal -Actual ([string] (Get-PropertyValue -Object $boundary -Name "schema_version")) -Expected $ExpectedSchemaVersion -Message "schema_version mismatch"
Assert-Equal -Actual ([string] (Get-PropertyValue -Object $boundary -Name "repo_slug")) -Expected "gongzzang" -Message "repo_slug mismatch"
Assert-Equal -Actual ([string] (Get-PropertyValue -Object $boundary -Name "phase")) -Expected "m3_2_physical_extraction_enforced" -Message "phase mismatch"

$entries = Get-RequiredArray -Object $boundary -Name "path_ownership"
$contracts = Get-RequiredArray -Object $boundary -Name "allowed_integration_contracts"
$forbiddenContracts = Get-RequiredArray -Object $boundary -Name "forbidden_integration_contracts"
$forbiddenCanonicalCatalogTables = Get-RequiredArray -Object $boundary -Name "forbidden_canonical_catalog_tables"
$rootEnvExampleContract = Get-PropertyValue -Object $boundary -Name "root_env_example_contract"
$gates = @(Get-RequiredArray -Object $boundary -Name "required_ci_gates" | ForEach-Object { [string] $_ })
$legacySchemaAllowances = Get-RequiredArray -Object $boundary -Name "allowed_legacy_schema_tokens"
$tokens = @(Get-RequiredArray -Object $boundary -Name "forbidden_code_tokens" | ForEach-Object { [string] $_ })
$forbiddenActiveDocTokens = Get-RequiredArray -Object $boundary -Name "forbidden_active_documentation_tokens"
$forbiddenActiveDocSectionTokens = Get-RequiredArray -Object $boundary -Name "forbidden_active_documentation_section_tokens"

Assert-RequiredPathOwnership -Entries $entries
foreach ($required in $RequiredPathOwnership) {
    if (!($required.Owner -eq "platform-core" -and ([string] $required.Classification).StartsWith("extracted_", [System.StringComparison]::Ordinal))) {
        Assert-PathExists -RootPath $resolvedRoot -RelativePath $required.Path
    }
}
foreach ($entry in $entries) {
    $entryOwner = [string] (Get-PropertyValue -Object $entry -Name "owner")
    $entryClassification = [string] (Get-PropertyValue -Object $entry -Name "classification")
    $entryPath = [string] (Get-PropertyValue -Object $entry -Name "path")
    if ($entryOwner -eq "platform-core" -and $entryClassification.StartsWith("extracted_", [System.StringComparison]::Ordinal)) {
        Assert-PathAbsent -RootPath $resolvedRoot -RelativePath $entryPath
    }
}
Assert-RequiredContracts -Contracts $contracts
Assert-ForbiddenContracts -Contracts $forbiddenContracts
Assert-ForbiddenCanonicalCatalogTables -Tables $forbiddenCanonicalCatalogTables
Assert-RootEnvExampleContractDefinition -Contract $rootEnvExampleContract
Assert-RequiredCiGates -Gates $gates
Assert-ForbiddenActiveDocumentationTokens -RootPath $resolvedRoot -Rules $forbiddenActiveDocTokens
Assert-ForbiddenActiveDocumentationSectionTokens -RootPath $resolvedRoot -Rules $forbiddenActiveDocSectionTokens
Assert-CiGateWiring -RootPath $resolvedRoot -Gates $gates
Assert-LegacySchemaTokenLedger -RootPath $resolvedRoot -Allowances $legacySchemaAllowances
Assert-CleanupMigrationContract -RootPath $resolvedRoot
Assert-MigrationSmokeContract -RootPath $resolvedRoot
Assert-MigrationSmokeWorkflowContract -RootPath $resolvedRoot
Assert-NoCanonicalCatalogTableSqlUsage -RootPath $resolvedRoot
Assert-NoDirectPlatformCoreDatabaseReferences -RootPath $resolvedRoot
Assert-NoRootCatalogSourceEnvExamples -RootPath $resolvedRoot
Assert-RootEnvExamplePlatformCoreContract -RootPath $resolvedRoot
Assert-LocalGongzzangPostgresPortContract -RootPath $resolvedRoot
Assert-NoForbiddenCodeTokens -RootPath $resolvedRoot -Tokens $tokens

Write-Host "platform-core-boundary-ok entries=$($entries.Count) contracts=$($contracts.Count) gates=$($gates.Count) legacy_schema_allowances=$($legacySchemaAllowances.Count)"
