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
