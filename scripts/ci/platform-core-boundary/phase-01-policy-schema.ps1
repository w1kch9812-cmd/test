$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$boundaryPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $BoundaryRelativePath
if (!(Test-Path -LiteralPath $boundaryPath)) {
    throw "platform-core-boundary: missing boundary SSOT: $BoundaryRelativePath"
}

$boundary = Read-Utf8Text -Path $boundaryPath | ConvertFrom-Json
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
