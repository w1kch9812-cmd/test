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

        $content = Read-Utf8Text -Path $fullPath
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

        $content = Read-Utf8Text -Path $fullPath
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
    $ci = Read-Utf8Text -Path $ciPath
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
    $lefthook = Read-Utf8Text -Path $lefthookPath
    foreach ($gate in $RequiredCiGates) {
        $gateName = Split-Path -Leaf $gate
        if (!$lefthook.Contains($gateName)) {
            throw "platform-core-boundary: lefthook.yml must run $gateName"
        }
    }
}

Assert-ForbiddenActiveDocumentationTokens -RootPath $resolvedRoot -Rules $forbiddenActiveDocTokens
Assert-ForbiddenActiveDocumentationSectionTokens -RootPath $resolvedRoot -Rules $forbiddenActiveDocSectionTokens
Assert-CiGateWiring -RootPath $resolvedRoot -Gates $gates
