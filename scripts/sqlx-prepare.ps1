$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

if (-not $env:DATABASE_URL) {
    $envFile = Join-Path $repoRoot ".env"
    if (Test-Path $envFile) {
        $line = Get-Content $envFile | Where-Object { $_ -match "^DATABASE_URL=" } | Select-Object -First 1
        if ($line) {
            $env:DATABASE_URL = $line.Substring("DATABASE_URL=".Length)
        }
    }
}

if (-not $env:DATABASE_URL) {
    throw "DATABASE_URL is required. Set it directly or add DATABASE_URL to .env."
}

$sqlx = (Get-Command sqlx -ErrorAction SilentlyContinue).Source
if (-not $sqlx) {
    $cargoSqlx = Join-Path $env:USERPROFILE ".cargo\bin\sqlx.exe"
    if (Test-Path $cargoSqlx) {
        $sqlx = $cargoSqlx
    }
}

if (-not $sqlx) {
    throw "sqlx-cli is required. Run: cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls"
}

$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) {
    $cargoExe = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $cargoExe) {
        $cargo = $cargoExe
    }
}

if (-not $cargo) {
    throw "cargo is required to run cargo sqlx prepare."
}

& $sqlx database create
& $sqlx migrate run --source migrations
& $cargo sqlx prepare --workspace
