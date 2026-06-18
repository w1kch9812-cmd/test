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
            $content = Read-Utf8Text -Path $file.FullName
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

            $content = Read-Utf8Text -Path $file.FullName
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

        $content = Read-Utf8Text -Path $file.FullName
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

        $content = Read-Utf8Text -Path $fullPath
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

    $content = Read-Utf8Text -Path $fullPath
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
    $compose = Read-Utf8Text -Path $composePath
    if (!$compose.Contains('${POSTGRES_HOST_PORT:-15432}:5432')) {
        throw "platform-core-boundary: local Gongzzang Postgres must use POSTGRES_HOST_PORT default 15432, not Windows-reserved 5500"
    }

    $dockerEnvExamplePath = Resolve-RepoPath -RootPath $RootPath -RelativePath "infrastructure/docker/.env.example"
    if (!(Test-Path -LiteralPath $dockerEnvExamplePath -PathType Leaf)) {
        throw "platform-core-boundary: infrastructure/docker/.env.example is missing"
    }
    $dockerEnvExample = Read-Utf8Text -Path $dockerEnvExamplePath
    if (!$dockerEnvExample.Contains("POSTGRES_HOST_PORT=15432")) {
        throw "platform-core-boundary: infrastructure/docker/.env.example must set POSTGRES_HOST_PORT=15432"
    }

    $rootEnvExamplePath = Resolve-RepoPath -RootPath $RootPath -RelativePath ".env.example"
    $rootEnvExample = Read-Utf8Text -Path $rootEnvExamplePath
    if (!$rootEnvExample.Contains("@localhost:15432/gongzzang")) {
        throw "platform-core-boundary: .env.example DATABASE_URL must target local Gongzzang Postgres on port 15432"
    }
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

            $content = Read-Utf8Text -Path $file.FullName
            foreach ($table in $RequiredForbiddenCanonicalCatalogTables) {
                if (Test-ContentContainsCanonicalCatalogSqlUsage -Content $content -Table $table) {
                    $relative = Get-RepoRelativePath -RootPath $RootPath -FullPath $file.FullName
                    throw "platform-core-boundary: forbidden canonical Catalog table SQL usage '$table' in $relative"
                }
            }
        }
    }
}

Assert-NoCanonicalCatalogTableSqlUsage -RootPath $resolvedRoot
Assert-NoDirectPlatformCoreDatabaseReferences -RootPath $resolvedRoot
Assert-NoRootCatalogSourceEnvExamples -RootPath $resolvedRoot
Assert-RootEnvExamplePlatformCoreContract -RootPath $resolvedRoot
Assert-LocalGongzzangPostgresPortContract -RootPath $resolvedRoot
Assert-NoForbiddenCodeTokens -RootPath $resolvedRoot -Tokens $tokens
