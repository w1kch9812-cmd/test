$checkedFiles = 0
$violations = @()
foreach ($contract in $contracts) {
    $relativePaths = @()
    if ($null -ne $contract.PSObject.Properties["RelativePaths"]) {
        $relativePaths = @($contract.RelativePaths)
    }
    else {
        $relativePaths = @([string] $contract.RelativePath)
    }
    $relativePathLabel = $relativePaths -join ", "
    $contentParts = @()
    foreach ($relativePath in $relativePaths) {
        $path = Join-Path $resolvedRoot ($relativePath -replace "/", "\")
        if (!(Test-Path -LiteralPath $path)) {
            throw "missing PNU anchor PBF marker contract file: $relativePath"
        }

        $checkedFiles += 1
        if (Test-Path -LiteralPath $path -PathType Container) {
            $contentParts += (Get-ChildItem -LiteralPath $path -Recurse -File -Filter "*.rs" |
                    Sort-Object -Property FullName |
                    ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw } |
                    Out-String)
        }
        else {
            $contentParts += Get-Content -LiteralPath $path -Raw
        }
    }
    $content = $contentParts -join "`n"

    foreach ($token in @($contract.Tokens)) {
        if ($content.Contains($token)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "missing token"
            Value = $token
        }
    }

    $forbiddenTokens = @()
    if ($null -ne $contract.PSObject.Properties["Forbidden"]) {
        $forbiddenTokens = @($contract.Forbidden)
    }
    foreach ($token in $forbiddenTokens) {
        if ([string]::IsNullOrEmpty($token) -or !$content.Contains($token)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "forbidden token"
            Value = $token
        }
    }

    $forbiddenPatterns = @()
    if ($null -ne $contract.PSObject.Properties["ForbiddenRegex"]) {
        $forbiddenPatterns = @($contract.ForbiddenRegex)
    }
    foreach ($pattern in $forbiddenPatterns) {
        if ([string]::IsNullOrEmpty($pattern) -or ![regex]::IsMatch($content, $pattern)) {
            continue
        }
        $violations += [pscustomobject]@{
            Path = $relativePathLabel
            Kind = "forbidden pattern"
            Value = $pattern
        }
    }
}

if (@($violations).Count -gt 0) {
    foreach ($violation in $violations) {
        [Console]::Error.WriteLine(
            "PNU anchor PBF marker contract {0}: {1}: {2}",
            $violation.Kind,
            $violation.Path,
            $violation.Value
        )
    }
    throw "PNU anchor PBF marker contract violations found"
}
