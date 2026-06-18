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
        $content = Read-Utf8Text -Path $fullPath
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
        $content = Read-Utf8Text -Path $file.FullName
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

    $content = Read-Utf8Text -Path $fullPath
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

    $content = Read-Utf8Text -Path $fullPath
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

    $content = Read-Utf8Text -Path $fullPath
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

Assert-LegacySchemaTokenLedger -RootPath $resolvedRoot -Allowances $legacySchemaAllowances
Assert-CleanupMigrationContract -RootPath $resolvedRoot
Assert-MigrationSmokeContract -RootPath $resolvedRoot
Assert-MigrationSmokeWorkflowContract -RootPath $resolvedRoot
