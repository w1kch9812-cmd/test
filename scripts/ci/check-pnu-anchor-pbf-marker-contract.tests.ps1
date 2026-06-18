Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-pnu-anchor-pbf-marker-contract.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-pnu-anchor-pbf-marker-contract-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

. (Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract.tests.helpers.ps1")

Assert-FileLineCountAtMost -Path $PSCommandPath -MaxLines 600
Assert-FileLineCountAtMost -Path $ScriptPath -MaxLines 600

$checkerModuleRoot = Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract"
Get-ChildItem -LiteralPath $checkerModuleRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

$testHelperPath = Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract.tests.helpers.ps1"
Assert-FileLineCountAtMost -Path $testHelperPath -MaxLines 600

$testFixtureRoot = Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract.tests"
Get-ChildItem -LiteralPath $testFixtureRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-ContractFiles -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean PNU anchor PBF marker contract check exit code mismatch"
Assert-Contains $clean.Output "pnu-anchor-pbf-marker-contract-ok"

$missingAnchorInboxMigrationRoot = Join-Path $TempRoot "missing-anchor-inbox-migration"
Write-ContractFiles -Root $missingAnchorInboxMigrationRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorInboxMigrationRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql") -Force
$missingAnchorInboxMigration = Invoke-Checker -Root $missingAnchorInboxMigrationRoot
Assert-Equals $missingAnchorInboxMigration.ExitCode 1 "Missing anchor inbox migration contract check exit code mismatch"
Assert-Contains $missingAnchorInboxMigration.Output "missing PNU anchor PBF marker contract file: migrations/30016_platform_core_event_inbox_anchor_import.sql"

$missingAnchorImportRepositoryRoot = Join-Path $TempRoot "missing-anchor-import-repository"
Write-ContractFiles -Root $missingAnchorImportRepositoryRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImportRepositoryRoot "crates\db\src\platform_core_anchor.rs") -Force
$missingAnchorImportRepository = Invoke-Checker -Root $missingAnchorImportRepositoryRoot
Assert-Equals $missingAnchorImportRepository.ExitCode 1 "Missing anchor import repository contract check exit code mismatch"
Assert-Contains $missingAnchorImportRepository.Output "missing PNU anchor PBF marker contract file: crates/db/src/platform_core_anchor.rs"

$weakAnchorImportRepositoryRoot = Join-Path $TempRoot "weak-anchor-import-repository"
Write-ContractFiles -Root $weakAnchorImportRepositoryRoot
Write-File -Root $weakAnchorImportRepositoryRoot -RelativePath "crates\db\src\platform_core_anchor.rs" -Content @'
insert_inbox_event
import_anchor_rows
'@
$weakAnchorImportRepository = Invoke-Checker -Root $weakAnchorImportRepositoryRoot
Assert-Equals $weakAnchorImportRepository.ExitCode 1 "Weak anchor import repository contract check exit code mismatch"
Assert-Contains $weakAnchorImportRepository.Output "missing token"

$missingAnchorImporterRoot = Join-Path $TempRoot "missing-anchor-importer"
Write-ContractFiles -Root $missingAnchorImporterRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImporterRoot "services\api\src\platform_core_anchor_import.rs") -Force
$missingAnchorImporter = Invoke-Checker -Root $missingAnchorImporterRoot
Assert-Equals $missingAnchorImporter.ExitCode 1 "Missing anchor importer parser contract check exit code mismatch"
Assert-Contains $missingAnchorImporter.Output "missing PNU anchor PBF marker contract file: services/api/src/platform_core_anchor_import.rs"

$weakAnchorImporterRoot = Join-Path $TempRoot "weak-anchor-importer"
Write-ContractFiles -Root $weakAnchorImporterRoot
Write-File -Root $weakAnchorImporterRoot -RelativePath "services\api\src\platform_core_anchor_import.rs" -Content @'
parse_anchor_manifest
parse_anchor_rows
parse_anchor_entry
EPSG:4326
'@
$weakAnchorImporter = Invoke-Checker -Root $weakAnchorImporterRoot
Assert-Equals $weakAnchorImporter.ExitCode 1 "Weak anchor importer parser contract check exit code mismatch"
Assert-Contains $weakAnchorImporter.Output "missing token"

$missingAnchorImporterBinRoot = Join-Path $TempRoot "missing-anchor-importer-bin"
Write-ContractFiles -Root $missingAnchorImporterBinRoot
Remove-Item -LiteralPath (Join-Path $missingAnchorImporterBinRoot "services\api\src\bin\platform_core_anchor_import.rs") -Force
$missingAnchorImporterBin = Invoke-Checker -Root $missingAnchorImporterBinRoot
Assert-Equals $missingAnchorImporterBin.ExitCode 1 "Missing anchor importer binary contract check exit code mismatch"
Assert-Contains $missingAnchorImporterBin.Output "missing PNU anchor PBF marker contract file: services/api/src/bin/platform_core_anchor_import.rs"

$weakAnchorImporterBinRoot = Join-Path $TempRoot "weak-anchor-importer-bin"
Write-ContractFiles -Root $weakAnchorImporterBinRoot
Write-File -Root $weakAnchorImporterBinRoot -RelativePath "services\api\src\bin\platform_core_anchor_import.rs" -Content @'
PlatformCoreAnchorImport
parse_anchor_manifest
parse_anchor_rows
'@
$weakAnchorImporterBin = Invoke-Checker -Root $weakAnchorImporterBinRoot
Assert-Equals $weakAnchorImporterBin.ExitCode 1 "Weak anchor importer binary contract check exit code mismatch"
Assert-Contains $weakAnchorImporterBin.Output "missing token"

$missingRoot = Join-Path $TempRoot "missing"
New-Item -ItemType Directory -Force -Path $missingRoot | Out-Null
$missing = Invoke-Checker -Root $missingRoot
Assert-Equals $missing.ExitCode 1 "Missing contract file check exit code mismatch"
Assert-Contains $missing.Output "missing PNU anchor PBF marker contract file"

$weakRoot = Join-Path $TempRoot "weak-listing-source"
Write-ContractFiles -Root $weakRoot
Write-File -Root $weakRoot -RelativePath "apps\web\lib\map\marker-tile-contract.ts" -Content @'
response_format: z.literal("mvt_pbf")
position_source: z.literal("pnu_anchor")
bbox_marker_runtime_forbidden: z.literal(true)
dropped_marker_success_forbidden: z.literal(true)
PARCEL_ANCHOR_MARKER_TILE_LAYER
ALL_ACTIVE_MARKER_FILTER_HASH
buildMarkerTileSource
'@
$weak = Invoke-Checker -Root $weakRoot
Assert-Equals $weak.ExitCode 1 "Missing listing tile source token check exit code mismatch"
Assert-Contains $weak.Output "missing token"

$bboxRoot = Join-Path $TempRoot "bbox"
Write-ContractFiles -Root $bboxRoot
Add-Content -LiteralPath (Join-Path $bboxRoot "apps\web\lib\map\marker-tile-contract.ts") -Value "bbox="
$bbox = Invoke-Checker -Root $bboxRoot
Assert-Equals $bbox.ExitCode 1 "Forbidden bbox marker contract check exit code mismatch"
Assert-Contains $bbox.Output "forbidden token"

$legacyDbRoot = Join-Path $TempRoot "legacy-db"
Write-ContractFiles -Root $legacyDbRoot
Add-Content -LiteralPath (Join-Path $legacyDbRoot "crates\db\src\listing\marker_tile.rs") -Value "ST_MakeEnvelope"
$legacyDb = Invoke-Checker -Root $legacyDbRoot
Assert-Equals $legacyDb.ExitCode 1 "Legacy bbox DB query check exit code mismatch"
Assert-Contains $legacyDb.Output "forbidden token"

$unanchoredRoot = Join-Path $TempRoot "missing-unanchored-readiness"
Write-ContractFiles -Root $unanchoredRoot
Write-File -Root $unanchoredRoot -RelativePath "crates\db\src\listing\marker_tile.rs" -Content @'
find_listing_marker_tile
parcel_marker_anchor
ST_AsMVTGeom
ST_AsMVT
listing marker tile completeness violation
eligible_count
represented_count
'@
$unanchored = Invoke-Checker -Root $unanchoredRoot
Assert-Equals $unanchored.ExitCode 1 "Missing unanchored readiness check exit code mismatch"
Assert-Contains $unanchored.Output "missing token"

$missingAnchorInboxSmokeRoot = Join-Path $TempRoot "missing-anchor-inbox-migration-smoke"
Write-ContractFiles -Root $missingAnchorInboxSmokeRoot
Write-File -Root $missingAnchorInboxSmokeRoot -RelativePath "tests\migrations\test_v001_full.sh" -Content @'
parcel_marker_anchor
parcel_marker_anchor_srid_chk
parcel_marker_anchor_point_gist_idx
must not duplicate anchor_lng/anchor_lat columns
listing_marker_projection
listing_marker_filter_registry
listing_marker_projection_anchor_srid_chk
listing_marker_filter_registry_spec_shape_chk
'@
$missingAnchorInboxSmoke = Invoke-Checker -Root $missingAnchorInboxSmokeRoot
Assert-Equals $missingAnchorInboxSmoke.ExitCode 1 "Missing anchor inbox migration smoke check exit code mismatch"
Assert-Contains $missingAnchorInboxSmoke.Output "missing token"

$staleGateRoot = Join-Path $TempRoot "stale-review-gate"
Write-ContractFiles -Root $staleGateRoot
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\next-actions.md") -Value "implementation-approved"
Add-Content -LiteralPath (Join-Path $staleGateRoot "docs\superpowers\roadmap.md") -Value "waiting for user review"
$staleGate = Invoke-Checker -Root $staleGateRoot
Assert-Equals $staleGate.ExitCode 1 "Stale review-gate wording check exit code mismatch"
Assert-Contains $staleGate.Output "forbidden token"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-pnu-anchor-pbf-marker-contract-tests-ok"
