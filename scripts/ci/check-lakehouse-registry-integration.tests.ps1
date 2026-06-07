Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-lakehouse-registry-integration.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-lakehouse-registry-integration-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
    $ErrorActionPreference = $previousErrorActionPreference

    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
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

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $WrongLakehouseBucket,
        [switch] $OmitIndexGuardrail,
        [switch] $OmitBoundaryContract,
        [switch] $UnmanagedSharedRootWrite,
        [switch] $PhotoKeyOutsideMediaNamespace
    )

    $guardrailLine = if ($OmitIndexGuardrail) {
        ""
    } else {
        '    "scripts/ci/check-lakehouse-registry-integration.ps1",'
    }
    $boundaryContract = if ($OmitBoundaryContract) {
        ""
    } else {
        ',{"kind":"lakehouse_registry_registration","direction":"gongzzang_to_platform_core","purpose":"register Gongzzang-owned governed lakehouse artifacts after verified writes"}'
    }
    $lakehouseBucket = if ($WrongLakehouseBucket) { "gongzzang" } else { "gongzzang-lakehouse-prod" }
    $photoKey = if ($PhotoKeyOutsideMediaNamespace) {
        "listing-photo/listings/lst_1/photos/lph_1.jpg"
    } else {
        "media/listing-photo/listings/lst_1/photos/lph_1.jpg"
    }

    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\index.v1.json" -Content @"
{
  "schema_version": "gongzzang.platform_integration.index.v1",
  "repo_slug": "gongzzang",
  "components": [
    {"id":"platform_integration.lakehouse_registry","path":"docs/architecture/platform-integration/lakehouse-registry-policy.v1.json","schema_version":"gongzzang.platform_integration.lakehouse_registry_policy.v1"}
  ],
  "required_guardrails": [
$guardrailLine
    "scripts/ci/check-platform-integration-policy.ps1"
  ]
}
"@

    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\lakehouse-registry-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.lakehouse_registry_policy.v1",
  "repo_slug": "gongzzang",
  "owner_service": "gongzzang",
  "platform_core_registry": {
    "contract_ref": "../platform-core/docs/adr/0009-cross-service-lakehouse-registry-control-plane.md",
    "allowed_call_id": "gongzzang_pipeline_to_platform_core_lakehouse_registry",
    "api_surfaces": [
      "POST /internal/lakehouse/ingestion-runs",
      "POST /internal/lakehouse/artifacts",
      "POST /internal/lakehouse/quality-checks",
      "POST /internal/lakehouse/lineage",
      "POST /internal/lakehouse/promotions",
      "GET /internal/lakehouse/assets/{qualified_name}/active"
    ]
  },
  "storage_namespaces": [
    {
      "id": "gongzzang_r2_production",
      "provider": "r2",
      "environment": "production",
      "owner_service": "gongzzang",
      "bucket_name": "gongzzang-lakehouse-prod",
      "root_prefix": null,
      "catalog_provider": "r2_data_catalog",
      "status": "active",
      "allowed_root_prefixes": ["bronze/", "silver/", "gold/", "media/", "__r2_data_catalog/"]
    }
  ],
  "governed_assets": [
    {
      "qualified_name": "gongzzang.bronze.onbid_sale",
      "layer": "bronze",
      "asset_kind": "raw_object_set",
      "namespace_id": "gongzzang_r2_production",
      "allowed_object_prefixes": ["bronze/source=onbid-sale/"],
      "registry_required": true
    },
    {
      "qualified_name": "gongzzang.bronze.court_auction",
      "layer": "bronze",
      "asset_kind": "raw_object_set",
      "namespace_id": "gongzzang_r2_production",
      "allowed_object_prefixes": ["bronze/source=court-auction/"],
      "registry_required": true
    },
    {
      "qualified_name": "gongzzang.gold.listing_marker_tiles",
      "layer": "gold",
      "asset_kind": "pbf_tile_set",
      "namespace_id": "gongzzang_r2_production",
      "allowed_object_prefixes": ["gold/listing-marker-tiles/"],
      "registry_required": true
    },
    {
      "qualified_name": "gongzzang.gold.listing_marker_serving_index",
      "layer": "gold",
      "asset_kind": "manifest",
      "namespace_id": "gongzzang_r2_production",
      "allowed_object_prefixes": ["gold/listing-marker-serving-index/"],
      "registry_required": true
    },
    {
      "qualified_name": "gongzzang.gold.listing_photo_media",
      "layer": "gold",
      "asset_kind": "media_set",
      "namespace_id": "gongzzang_r2_production",
      "allowed_object_prefixes": ["media/listing-photo/"],
      "registry_required": true
    }
  ],
  "forbidden_unowned_root_prefix_writes": ["bronze/", "silver/", "gold/"],
  "required_env_contract": {
    "GONGZZANG_LAKEHOUSE_R2_BUCKET": "gongzzang-lakehouse-prod",
    "LISTING_PHOTO_R2_BUCKET": "gongzzang-lakehouse-prod"
  }
}
'@

    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content @"
{
  "schema_version": "gongzzang.platform_core_boundary.v1",
  "repo_slug": "gongzzang",
  "allowed_integration_contracts": [
    {"kind":"http_api","direction":"gongzzang_to_platform_core","purpose":"published Platform Core API consumption"}$boundaryContract
  ]
}
"@

    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\allowed-call-matrix.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.allowed_call_matrix.v1",
  "allowed_calls": [
    {
      "id": "gongzzang_pipeline_to_platform_core_lakehouse_registry",
      "status": "planned",
      "source_repo": "gongzzang",
      "source_service": "gongzzang-worker",
      "target_repo": "platform-core",
      "target_service": "platform-core-api",
      "allowed_surfaces": [
        "POST /internal/lakehouse/ingestion-runs",
        "POST /internal/lakehouse/artifacts",
        "POST /internal/lakehouse/quality-checks",
        "POST /internal/lakehouse/lineage",
        "POST /internal/lakehouse/promotions",
        "GET /internal/lakehouse/assets/{qualified_name}/active"
      ],
      "current_required_controls": [
        "registry_contract_defined",
        "object_checksum_verified",
        "no_direct_database"
      ],
      "service_auth_policy_id": "gongzzang_worker_to_platform_core_api"
    }
  ]
}
'@

    Write-File -Root $Root -RelativePath "docs\architecture\platform-integration\service-auth-policy.v1.json" -Content @'
{
  "schema_version": "gongzzang.platform_integration.service_auth_policy.v1",
  "outbound_identities": [
    {
      "id": "gongzzang_worker_to_platform_core_api",
      "source_service": "gongzzang-worker",
      "target_service": "platform-core-api",
      "token_metadata": {
        "required_scope": "lakehouse:write"
      },
      "authorization_policy": {
        "default_decision": "deny",
        "allowed_call_id": "gongzzang_pipeline_to_platform_core_lakehouse_registry"
      },
      "runtime_files": [
        "services/api/src/platform_core_auth.rs",
        "services/api/src/platform_core_lakehouse_registry.rs"
      ]
    }
  ]
}
'@

    Write-File -Root $Root -RelativePath ".env.example" -Content @"
PLATFORM_CORE_API_BASE_URL=http://localhost:18080
GONGZZANG_LAKEHOUSE_R2_ACCOUNT_ID=
GONGZZANG_LAKEHOUSE_R2_ACCESS_KEY=
GONGZZANG_LAKEHOUSE_R2_SECRET_KEY=
GONGZZANG_LAKEHOUSE_R2_BUCKET=$lakehouseBucket
LISTING_PHOTO_R2_ACCOUNT_ID=
LISTING_PHOTO_R2_ACCESS_KEY=
LISTING_PHOTO_R2_SECRET_KEY=
LISTING_PHOTO_R2_BUCKET=gongzzang-lakehouse-prod
"@

    Write-File -Root $Root -RelativePath "services\api\src\photo_upload\tests.rs" -Content @"
const PHOTO_KEY: &str = "$photoKey";
"@

    Write-File -Root $Root -RelativePath "docs\adr\0039-service-owned-lakehouse-registry-integration.md" -Content @'
# ADR 0039

Enterprise benchmark: [benchmark](../research/2026-06-07-enterprise-lakehouse-media-registry-benchmark.md)
'@
    Write-File -Root $Root -RelativePath "docs\superpowers\specs\2026-06-05-gongzzang-service-owned-lakehouse-integration-design.md" -Content @'
# Gongzzang Service-Owned Lakehouse Integration Design

Enterprise benchmark: `../../research/2026-06-07-enterprise-lakehouse-media-registry-benchmark.md`
'@

    if ($UnmanagedSharedRootWrite) {
        Write-File -Root $Root -RelativePath "scripts\bad-writer.ps1" -Content @'
aws s3 cp source.json s3://gongzzang/bronze/source=onbid-sale/part-0001.json
'@
    } else {
        Write-File -Root $Root -RelativePath "scripts\good-writer.ps1" -Content @'
aws s3 cp source.json s3://gongzzang-lakehouse-prod/bronze/source=onbid-sale/part-0001.json
'@
    }
}

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-Equals $success.ExitCode 0 "success exit code mismatch"
    Assert-Contains $success.Output "lakehouse-registry-integration-ok namespaces=1 assets=5 media_sets=1"

    $missingGuardrailRoot = Join-Path $TempRoot "missing-guardrail"
    Write-MinimalRepo -Root $missingGuardrailRoot -OmitIndexGuardrail
    $missingGuardrail = Invoke-Checker -Root $missingGuardrailRoot
    Assert-Equals $missingGuardrail.ExitCode 1 "missing guardrail exit code mismatch"
    Assert-Contains $missingGuardrail.Output "index required_guardrails missing scripts/ci/check-lakehouse-registry-integration.ps1"

    $wrongBucketRoot = Join-Path $TempRoot "wrong-bucket"
    Write-MinimalRepo -Root $wrongBucketRoot -WrongLakehouseBucket
    $wrongBucket = Invoke-Checker -Root $wrongBucketRoot
    Assert-Equals $wrongBucket.ExitCode 1 "wrong bucket exit code mismatch"
    Assert-Contains $wrongBucket.Output "GONGZZANG_LAKEHOUSE_R2_BUCKET must be gongzzang-lakehouse-prod"

    $missingBoundaryContractRoot = Join-Path $TempRoot "missing-boundary-contract"
    Write-MinimalRepo -Root $missingBoundaryContractRoot -OmitBoundaryContract
    $missingBoundaryContract = Invoke-Checker -Root $missingBoundaryContractRoot
    Assert-Equals $missingBoundaryContract.ExitCode 1 "missing boundary contract exit code mismatch"
    Assert-Contains $missingBoundaryContract.Output "missing allowed integration contract missing lakehouse_registry_registration:gongzzang_to_platform_core"

    $badWriterRoot = Join-Path $TempRoot "bad-writer"
    Write-MinimalRepo -Root $badWriterRoot -UnmanagedSharedRootWrite
    $badWriter = Invoke-Checker -Root $badWriterRoot
    Assert-Equals $badWriter.ExitCode 1 "bad writer exit code mismatch"
    Assert-Contains $badWriter.Output "unmanaged shared-root lakehouse write"

    $badPhotoRoot = Join-Path $TempRoot "bad-photo"
    Write-MinimalRepo -Root $badPhotoRoot -PhotoKeyOutsideMediaNamespace
    $badPhoto = Invoke-Checker -Root $badPhotoRoot
    Assert-Equals $badPhoto.ExitCode 1 "bad photo exit code mismatch"
    Assert-Contains $badPhoto.Output "listing photo object keys must stay under media/listing-photo/"

    Write-Host "lakehouse-registry-integration-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
