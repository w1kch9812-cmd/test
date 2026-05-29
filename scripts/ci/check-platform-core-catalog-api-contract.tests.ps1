Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-catalog-api-contract.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-catalog-api-contract-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Invoke-Checker {
    param([string] $Root, [string] $PlatformCoreRoot = "")

    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    if (![string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
        $args += @("-PlatformCoreRoot", $PlatformCoreRoot)
    }

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe @args 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    $normalizedOutput = @($output | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) {
            $_.Exception.Message
        } else {
            [string] $_
        }
    })
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($normalizedOutput -join [Environment]::NewLine)
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

    $actualCompact = $Text -replace "\s+", ""
    $expectedCompact = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$actualCompact.Contains($expectedCompact)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function New-PinnedContractJson {
    param([string] $ParcelEnv = "PLATFORM_CORE_API_BASE_URL")

    return @"
{
  "schema_version": "gongzzang.platform_core_catalog_api_contract_pin.v1",
  "source_repo": "platform-core",
  "source_path": "docs/openapi/catalog.v1.yaml",
  "source_schema_version": "platform-core.openapi.catalog.v1",
  "consumer_slug": "gongzzang",
  "base_env": "$ParcelEnv",
  "allowed_reference_modules": [
    "services/api/src/startup.rs"
  ],
  "endpoints": [
    {
      "operation_id": "getParcelByPnu",
      "client_module": "services/api/src/platform_core_parcel_lookup.rs",
      "method": "GET",
      "path_template": "/catalog/v1/parcels/by-pnu/{pnu}",
      "path_pattern_literal": "catalog/v1/parcels/by-pnu/",
      "success_shape": "object",
      "not_found_behavior": "none",
      "required_response_fields": ["pnu", "kind"]
    },
    {
      "operation_id": "listParcelBuildingsByPnu",
      "client_module": "services/api/src/building_reader.rs",
      "method": "GET",
      "path_template": "/catalog/v1/parcels/by-pnu/{pnu}/buildings",
      "path_pattern_literal": "catalog/v1/parcels/by-pnu/{}/buildings",
      "success_shape": "array",
      "required_response_fields": [
        "id",
        "parcel_id",
        "purpose_code",
        "structure_code",
        "floor_area_m2",
        "stories",
        "built_year",
        "updated_at"
      ]
    }
  ]
}
"@
}

function New-PlatformCoreOpenApiYaml {
    param(
        [string] $ParcelFields = "id, complex_id, pnu, kind, area_m2, version, updated_at",
        [string] $BuildingFields = "id, parcel_id, purpose_code, structure_code, floor_area_m2, stories, built_year, updated_at",
        [bool] $OmitBuildingPath = $false
    )

    $buildingPath = if ($OmitBuildingPath) { "" } else { @"
  /catalog/v1/parcels/by-pnu/{pnu}/buildings:
    get:
      operationId: listParcelBuildingsByPnu
      responses:
        '200':
          content:
            application/json:
              schema:
                type: array
                items:
                  `$ref: '#/components/schemas/BuildingResponse'
"@ }

    return @"
openapi: 3.0.3
info:
  title: Platform Core Catalog API
  version: v1
paths:
  /catalog/v1/parcels/by-pnu/{pnu}:
    get:
      operationId: getParcelByPnu
      responses:
        '200':
          content:
            application/json:
              schema:
                `$ref: '#/components/schemas/ParcelResponse'
        '404':
          description: Parcel not found.
$buildingPath
components:
  schemas:
    ParcelResponse:
      type: object
      required: [$ParcelFields]
      properties:
        pnu: { type: string }
        kind: { type: string }
    BuildingResponse:
      type: object
      required: [$BuildingFields]
      properties:
        id: { type: string }
        parcel_id: { type: string }
        purpose_code: { type: string }
        structure_code: { type: string }
        floor_area_m2: { type: number }
        stories: { type: integer }
        built_year: { type: integer }
        updated_at: { type: string }
"@
}

function Write-ParcelClient {
    param(
        [string] $Root,
        [bool] $OmitKindField = $false,
        [bool] $WrongEnv = $false,
        [bool] $WrongPath = $false,
        [bool] $OmitCircuitBreaker = $false
    )

    $envName = if ($WrongEnv) { "PLATFORM_CORE_DATABASE_URL" } else { "PLATFORM_CORE_API_BASE_URL" }
    $pathLiteral = if ($WrongPath) { "catalog/v1/parcels/" } else { "catalog/v1/parcels/by-pnu/" }
    $kindField = if ($OmitKindField) { "" } else { "kind: String," }
    $call = if ($OmitCircuitBreaker) {
        "let response = client.get(url).send().await?;"
    } else {
        @"
    let policy = Policy::platform_core_default();
    let response = execute(&breaker, &policy, "platform_core.catalog.get_parcel_by_pnu", || async {
        client.get(url).send().await
    }).await?;
"@
    }

    Write-File -Root $Root -RelativePath "services\api\src\platform_core_parcel_lookup.rs" -Content @"
const PLATFORM_CORE_API_BASE_URL_ENV: &str = "$envName";

async fn lookup_pnu(pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError> {
    let url = base.join(&format!("$pathLiteral{}", pnu.as_str()))?;
    $call
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let parcel = response.json::<PlatformCoreParcelResponse>().await?;
    Ok(Some(parcel.try_into()?))
}

struct PlatformCoreParcelResponse {
    pnu: String,
    $kindField
}
"@
}

function Write-BuildingReader {
    param(
        [string] $Root,
        [bool] $OmitFloorArea = $false,
        [bool] $WrongPath = $false,
        [bool] $OmitCircuitBreaker = $false
    )

    $floorAreaField = if ($OmitFloorArea) { "" } else { "floor_area_m2: f64," }
    $pathLiteral = if ($WrongPath) {
        "{}/api/buildings?parcel_pnu={}"
    } else {
        "{}/catalog/v1/parcels/by-pnu/{}/buildings"
    }
    $call = if ($OmitCircuitBreaker) {
        "let response = client.get(url).send().await?;"
    } else {
        @"
    let policy = Policy::platform_core_default();
    let response = execute(&breaker, &policy, "platform_core.catalog.list_parcel_buildings_by_pnu", || async {
        client.get(url).send().await
    }).await?;
"@
    }

    Write-File -Root $Root -RelativePath "services\api\src\building_reader.rs" -Content @"
const PLATFORM_CORE_API_BASE_URL_ENV: &str = "PLATFORM_CORE_API_BASE_URL";

async fn list_by_pnu(pnu: &Pnu) -> Result<Vec<BuildingRegisterRecord>, BuildingLookupError> {
    let url = format!("$pathLiteral", base, pnu.as_str());
    $call
    let buildings = response.json::<Vec<PlatformCoreBuildingResponse>>().await?;
    Ok(buildings.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()?)
}

struct PlatformCoreBuildingResponse {
    id: String,
    parcel_id: String,
    purpose_code: String,
    structure_code: String,
    $floorAreaField
    stories: i32,
    built_year: i32,
    updated_at: DateTime<Utc>,
}
"@
}

function Write-Fixture {
    param(
        [string] $Root,
        [string] $PlatformCoreRoot = "",
        [bool] $OmitParcelKind = $false,
        [bool] $ParcelWrongEnv = $false,
        [bool] $ParcelOmitCircuitBreaker = $false,
        [bool] $BuildingWrongPath = $false,
        [bool] $OpenApiMissingBuildingPath = $false,
        [string] $OpenApiBuildingFields = "id, parcel_id, purpose_code, structure_code, floor_area_m2, stories, built_year, updated_at"
    )

    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-catalog-api-contract.v1.pin.json" -Content (New-PinnedContractJson)
    Write-ParcelClient -Root $Root -OmitKindField $OmitParcelKind -WrongEnv $ParcelWrongEnv -OmitCircuitBreaker $ParcelOmitCircuitBreaker
    Write-BuildingReader -Root $Root -WrongPath $BuildingWrongPath
    Write-File -Root $Root -RelativePath "services\api\src\startup.rs" -Content '"parcel_lookup: Platform Core Catalog live (/catalog/v1/parcels/by-pnu/:pnu)"'

    if (![string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
        Write-File -Root $PlatformCoreRoot -RelativePath "docs\openapi\catalog.v1.yaml" -Content (New-PlatformCoreOpenApiYaml -BuildingFields $OpenApiBuildingFields -OmitBuildingPath $OpenApiMissingBuildingPath)
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-Fixture -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean Catalog API contract check exit code mismatch"
Assert-Contains $clean.Output "platform-core-catalog-api-contract-ok"

$crossRepoRoot = Join-Path $TempRoot "cross-repo"
$crossCoreRoot = Join-Path $TempRoot "platform-core"
Write-Fixture -Root $crossRepoRoot -PlatformCoreRoot $crossCoreRoot
$crossRepo = Invoke-Checker -Root $crossRepoRoot -PlatformCoreRoot $crossCoreRoot
Assert-Equals $crossRepo.ExitCode 0 "Cross-repo Catalog API contract check exit code mismatch"
Assert-Contains $crossRepo.Output "source_checked=True"

$missingLocalFieldRoot = Join-Path $TempRoot "missing-local-field"
Write-Fixture -Root $missingLocalFieldRoot -OmitParcelKind $true
$missingLocalField = Invoke-Checker -Root $missingLocalFieldRoot
Assert-Equals $missingLocalField.ExitCode 1 "Missing local response field check exit code mismatch"
Assert-Contains $missingLocalField.Output "missing local response field 'kind'"

$wrongEnvRoot = Join-Path $TempRoot "wrong-env"
Write-Fixture -Root $wrongEnvRoot -ParcelWrongEnv $true
$wrongEnv = Invoke-Checker -Root $wrongEnvRoot
Assert-Equals $wrongEnv.ExitCode 1 "Wrong base env check exit code mismatch"
Assert-Contains $wrongEnv.Output "direct Platform Core database configuration"

$wrongPathRoot = Join-Path $TempRoot "wrong-path"
Write-Fixture -Root $wrongPathRoot -BuildingWrongPath $true
$wrongPath = Invoke-Checker -Root $wrongPathRoot
Assert-Equals $wrongPath.ExitCode 1 "Wrong path literal check exit code mismatch"
Assert-Contains $wrongPath.Output "missing local path literal"

$missingBreakerRoot = Join-Path $TempRoot "missing-circuit-breaker"
Write-Fixture -Root $missingBreakerRoot -ParcelOmitCircuitBreaker $true
$missingBreaker = Invoke-Checker -Root $missingBreakerRoot
Assert-Equals $missingBreaker.ExitCode 1 "Missing Platform Core circuit breaker check exit code mismatch"
Assert-Contains $missingBreaker.Output "local client must use circuit_breaker::execute"

$unpinnedConsumerRoot = Join-Path $TempRoot "unpinned-consumer"
Write-Fixture -Root $unpinnedConsumerRoot
Write-File -Root $unpinnedConsumerRoot -RelativePath "services\api\src\rogue_catalog_client.rs" -Content 'let url = format!("{}/catalog/v1/parcels/by-pnu/{}", base, pnu);'
$unpinnedConsumer = Invoke-Checker -Root $unpinnedConsumerRoot
Assert-Equals $unpinnedConsumer.ExitCode 1 "Unpinned Catalog API consumer check exit code mismatch"
Assert-Contains $unpinnedConsumer.Output "unpinned local Catalog API consumer"
Assert-Contains $unpinnedConsumer.Output "rogue_catalog_client.rs"

$missingOpenApiPathRoot = Join-Path $TempRoot "missing-openapi-path"
$missingOpenApiCoreRoot = Join-Path $TempRoot "missing-openapi-platform-core"
Write-Fixture -Root $missingOpenApiPathRoot -PlatformCoreRoot $missingOpenApiCoreRoot -OpenApiMissingBuildingPath $true
$missingOpenApiPath = Invoke-Checker -Root $missingOpenApiPathRoot -PlatformCoreRoot $missingOpenApiCoreRoot
Assert-Equals $missingOpenApiPath.ExitCode 1 "Missing Platform Core OpenAPI path check exit code mismatch"
Assert-Contains $missingOpenApiPath.Output "missing Platform Core OpenAPI path"

$missingOpenApiFieldRoot = Join-Path $TempRoot "missing-openapi-field"
$missingOpenApiFieldCoreRoot = Join-Path $TempRoot "missing-openapi-field-platform-core"
Write-Fixture `
    -Root $missingOpenApiFieldRoot `
    -PlatformCoreRoot $missingOpenApiFieldCoreRoot `
    -OpenApiBuildingFields "id, parcel_id, purpose_code, structure_code, stories, built_year, updated_at"
$missingOpenApiField = Invoke-Checker -Root $missingOpenApiFieldRoot -PlatformCoreRoot $missingOpenApiFieldCoreRoot
Assert-Equals $missingOpenApiField.ExitCode 1 "Missing Platform Core OpenAPI field check exit code mismatch"
Assert-Contains $missingOpenApiField.Output "floor_area_m2"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-platform-core-catalog-api-contract-tests-ok"
