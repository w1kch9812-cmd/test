$edgeProjection = $null
$awsWafRateRules = @()
$awsWafIdentityAwareRules = @()
$awsWafServiceIdentityRules = @()
$awsWafBlockedQueryShapeRules = @()

if ($IncludeProductionEdge) {
    $edgeGeneratedPath = Resolve-RepoPath -RelativePath "infrastructure/security/traffic-auth-edge-policy.generated.json"
    if (!(Test-Path -LiteralPath $edgeGeneratedPath -PathType Leaf)) {
        throw "traffic-auth edge policy projection missing: infrastructure/security/traffic-auth-edge-policy.generated.json"
    }
    $edgeGenerated = Get-Content -LiteralPath $edgeGeneratedPath -Raw -Encoding UTF8
    $edgeProjection = $edgeGenerated | ConvertFrom-Json
    $awsWafManifestPath = Resolve-RepoPath -RelativePath "infrastructure/security/aws-wafv2-edge-policy.generated.json"
    if (!(Test-Path -LiteralPath $awsWafManifestPath -PathType Leaf)) {
        throw "AWS WAFv2 edge manifest missing: infrastructure/security/aws-wafv2-edge-policy.generated.json"
    }
    $awsWafManifest = (Get-Content -LiteralPath $awsWafManifestPath -Raw -Encoding UTF8) | ConvertFrom-Json
    $pulumiProjectPath = Resolve-RepoPath -RelativePath "infrastructure/Pulumi.yaml"
    if (!(Test-Path -LiteralPath $pulumiProjectPath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 consumer missing: infrastructure/Pulumi.yaml"
    }
    $pulumiIndexPath = Resolve-RepoPath -RelativePath "infrastructure/index.ts"
    if (!(Test-Path -LiteralPath $pulumiIndexPath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 consumer missing: infrastructure/index.ts"
    }
    $pulumiPackagePath = Resolve-RepoPath -RelativePath "infrastructure/package.json"
    if (!(Test-Path -LiteralPath $pulumiPackagePath -PathType Leaf)) {
        throw "Pulumi AWS WAFv2 package missing: infrastructure/package.json"
    }
    $pulumiLocalPreviewPath = Resolve-RepoPath -RelativePath "infrastructure/Pulumi.local-preview.yaml"
    if (!(Test-Path -LiteralPath $pulumiLocalPreviewPath -PathType Leaf)) {
        throw "Pulumi local-preview stack missing: infrastructure/Pulumi.local-preview.yaml"
    }
    $pulumiProject = Read-TextFile -RelativePath "infrastructure/Pulumi.yaml"
    $pulumiIndex = Read-TextFile -RelativePath "infrastructure/index.ts"
    $pulumiPackage = Read-JsonFile -RelativePath "infrastructure/package.json"
    $pulumiLocalPreviewStack = Read-TextFile -RelativePath "infrastructure/Pulumi.local-preview.yaml"

    Assert-Equals `
        -Actual ([string] $edgeProjection.schema_version) `
        -Expected "gongzzang.traffic_auth_edge_policy_projection.v1" `
        -Message "traffic-auth edge policy schema_version"
    Assert-Equals `
        -Actual ([string] $edgeProjection.source_registry) `
        -Expected "docs/architecture/traffic-auth-policy-registry.v1.json" `
        -Message "traffic-auth edge policy source_registry"
    Assert-Equals `
        -Actual ([string] $edgeProjection.projection_kind) `
        -Expected "provider_neutral_edge_ingress" `
        -Message "traffic-auth edge policy projection_kind"
    foreach ($target in @("cloudfront", "aws_wafv2", "alb", "service_mesh")) {
        Assert-ArrayContains `
            -Values @($edgeProjection.generated_targets) `
            -Expected $target `
            -Message "traffic-auth edge policy generated_targets"
    }
    Assert-Equals `
        -Actual ([string] $awsWafManifest.schema_version) `
        -Expected "gongzzang.aws_wafv2_edge_policy_manifest.v1" `
        -Message "AWS WAFv2 edge manifest schema_version"
    Assert-Equals `
        -Actual ([string] $awsWafManifest.source_projection) `
        -Expected "infrastructure/security/traffic-auth-edge-policy.generated.json" `
        -Message "AWS WAFv2 edge manifest source_projection"
    Assert-Equals `
        -Actual ([string] $awsWafManifest.managed_by) `
        -Expected "pulumi" `
        -Message "AWS WAFv2 edge manifest managed_by"
    foreach ($scope in @("CLOUDFRONT", "REGIONAL")) {
        Assert-ArrayContains `
            -Values @($awsWafManifest.scope_options) `
            -Expected $scope `
            -Message "AWS WAFv2 edge manifest scope_options"
    }
    $awsWafRateRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "rate_based_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafIdentityAwareRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "identity_aware_application_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafServiceIdentityRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "service_identity_rules" `
            -Message "AWS WAFv2 edge manifest")
    $awsWafBlockedQueryShapeRules = @(Get-RequiredProperty `
            -Object $awsWafManifest `
            -Name "blocked_query_shape_rules" `
            -Message "AWS WAFv2 edge manifest")
    Assert-Unique -Values ($awsWafRateRules | ForEach-Object { $_.source_policy_id }) -Message "AWS WAFv2 edge rate rule source_policy_id must be unique"
    Assert-Unique -Values ($awsWafRateRules | ForEach-Object { $_.priority }) -Message "AWS WAFv2 edge rate rule priority must be unique"
    Assert-Contains -Content $pulumiProject -Needle "runtime: nodejs" -Message "Pulumi AWS WAFv2 consumer project runtime"
    Assert-Contains -Content $pulumiIndex -Needle "@pulumi/aws" -Message "Pulumi AWS WAFv2 consumer AWS provider import"
    Assert-Contains -Content $pulumiIndex -Needle "aws.wafv2.WebAcl" -Message "Pulumi AWS WAFv2 consumer WebAcl resource"
    Assert-Contains -Content $pulumiIndex -Needle "wafRegionalResourceArn" -Message "Pulumi AWS WAFv2 association regional resource config"
    Assert-Contains -Content $pulumiIndex -Needle "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN" -Message "Pulumi AWS WAFv2 association preview env fallback"
    Assert-Contains -Content $pulumiIndex -Needle "aws.wafv2.WebAclAssociation" -Message "Pulumi AWS WAFv2 association resource"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:region: ap-northeast-2" -Message "Pulumi local-preview stack AWS region"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipCredentialsValidation" -Message "Pulumi local-preview stack credential validation skip"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipRequestingAccountId" -Message "Pulumi local-preview stack account ID request skip"
    Assert-Contains -Content $pulumiLocalPreviewStack -Needle "aws:skipMetadataApiCheck" -Message "Pulumi local-preview stack metadata check skip"
    Assert-NotContains `
        -Content $pulumiLocalPreviewStack `
        -Needle "wafRegionalResourceArn" `
        -Message "Pulumi local-preview stack must not persist regional association target"
    Assert-Contains `
        -Content $pulumiIndex `
        -Needle "security/aws-wafv2-edge-policy.generated.json" `
        -Message "Pulumi AWS WAFv2 consumer generated manifest input"
    foreach ($requiredManifestMember in @(
            "rate_based_rules",
            "blocked_query_shape_rules",
            "identity_aware_application_rules",
            "service_identity_rules"
        )) {
        Assert-Contains `
            -Content $pulumiIndex `
            -Needle $requiredManifestMember `
            -Message "Pulumi AWS WAFv2 consumer manifest member"
    }
    $pulumiDependencyNames = @()
    if ($null -ne $pulumiPackage.PSObject.Properties["dependencies"]) {
        $pulumiDependencyNames += @($pulumiPackage.dependencies.PSObject.Properties.Name)
    }
    if ($null -ne $pulumiPackage.PSObject.Properties["devDependencies"]) {
        $pulumiDependencyNames += @($pulumiPackage.devDependencies.PSObject.Properties.Name)
    }
    foreach ($dependencyName in @("@pulumi/aws", "@pulumi/pulumi", "pulumi")) {
        Assert-ArrayContains `
            -Values $pulumiDependencyNames `
            -Expected $dependencyName `
            -Message "Pulumi AWS WAFv2 package dependency"
    }
}
