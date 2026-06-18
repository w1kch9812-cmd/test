$npmSupplyChainPolicy = $supplyChainPolicy.npm
Assert-Equals `
    -Actual $npmSupplyChainPolicy.audit_bazel_target `
    -Expected "//tools/bazel:ci_node_audit_transition" `
    -Message "supply chain npm audit Bazel target mismatch"

$rustSupplyChainPolicy = $supplyChainPolicy.rust
Assert-Equals -Actual $rustSupplyChainPolicy.sca -Expected "cargo-deny" -Message "supply chain Rust SCA tool mismatch"
Assert-Equals -Actual $rustSupplyChainPolicy.config -Expected "deny.toml" -Message "supply chain Rust SCA config mismatch"
Assert-Equals `
    -Actual $rustSupplyChainPolicy.bazel_target `
    -Expected "//tools/bazel:ci_cargo_deny_transition" `
    -Message "supply chain Rust SCA Bazel target mismatch"

$releaseArtifacts = @($supplyChainPolicy.release_artifacts)
Assert-Equals -Actual $releaseArtifacts.Count -Expected 2 -Message "supply chain release artifact count mismatch"
Assert-Unique -Values ($releaseArtifacts | ForEach-Object { $_.id }) -Message "supply chain release artifact ids must be unique"
foreach ($artifact in $releaseArtifacts) {
    Assert-NotEmptyString -Value $artifact.ecosystem -Message "supply chain release artifact ecosystem for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.bazel_target -Message "supply chain release artifact Bazel target for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.subject_path -Message "supply chain release artifact subject_path for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.sbom_source_path -Message "supply chain release artifact sbom_source_path for $($artifact.id)"
}
foreach ($requiredEcosystem in @("node", "rust")) {
    Assert-JsonArrayContains -Values @($releaseArtifacts | ForEach-Object { [string] $_.ecosystem }) -Expected $requiredEcosystem -Message "supply chain release artifact ecosystems"
}

$sbomPolicy = $supplyChainPolicy.sbom
Assert-Equals -Actual $sbomPolicy.required -Expected $true -Message "supply chain SBOM requirement mismatch"
Assert-Equals -Actual $sbomPolicy.format -Expected "cyclonedx-json" -Message "supply chain SBOM format mismatch"
Assert-Equals -Actual $sbomPolicy.generator.tool -Expected "bazel_release_file_sbom" -Message "supply chain SBOM generator mismatch"
Assert-Equals `
    -Actual $sbomPolicy.generator.implementation `
    -Expected "tools/bazel/generate_release_file_sbom.sh" `
    -Message "supply chain SBOM generator implementation mismatch"
Assert-FileExists -RelativePath ([string] $sbomPolicy.generator.implementation)
$sbomArtifacts = @($sbomPolicy.artifacts)
Assert-Equals -Actual $sbomArtifacts.Count -Expected 2 -Message "supply chain SBOM artifact count mismatch"
Assert-Unique -Values ($sbomArtifacts | ForEach-Object { $_.id }) -Message "supply chain SBOM artifact ids must be unique"
foreach ($artifact in $sbomArtifacts) {
    Assert-NotEmptyString -Value $artifact.ecosystem -Message "supply chain SBOM ecosystem for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.source_path -Message "supply chain SBOM source_path for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.bazel_target -Message "supply chain SBOM Bazel target for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.output_file -Message "supply chain SBOM output_file for $($artifact.id)"
    Assert-NotEmptyString -Value $artifact.subject_path -Message "supply chain SBOM subject_path for $($artifact.id)"
    if (!([string] $artifact.output_file).StartsWith("bazel-bin/")) {
        throw "supply chain SBOM output_file must be a Bazel output for $($artifact.id)"
    }
}
foreach ($requiredEcosystem in @("node", "rust")) {
    Assert-JsonArrayContains -Values @($sbomArtifacts | ForEach-Object { [string] $_.ecosystem }) -Expected $requiredEcosystem -Message "supply chain SBOM ecosystems"
}

$evidenceManifest = Get-JsonProperty -Object $supplyChainPolicy -Name "evidence_manifest"
if ($null -eq $evidenceManifest) {
    throw "supply chain evidence manifest policy missing"
}
Assert-Equals `
    -Actual $evidenceManifest.bazel_target `
    -Expected "//:supply_chain_evidence_manifest" `
    -Message "supply chain evidence manifest Bazel target mismatch"
Assert-Equals `
    -Actual $evidenceManifest.artifact_group_target `
    -Expected "//:supply_chain_evidence_artifacts" `
    -Message "supply chain evidence artifact group target mismatch"
Assert-Equals `
    -Actual $evidenceManifest.contract_target `
    -Expected "//:verify_supply_chain" `
    -Message "supply chain evidence contract target mismatch"
Assert-Equals `
    -Actual $evidenceManifest.output_file `
    -Expected "bazel-bin/supply-chain/evidence-manifest.json" `
    -Message "supply chain evidence manifest output mismatch"

$provenancePolicy = $supplyChainPolicy.provenance
Assert-Equals -Actual $provenancePolicy.required -Expected $true -Message "supply chain provenance requirement mismatch"
Assert-Equals -Actual $provenancePolicy.provider -Expected "github_artifact_attestations" -Message "supply chain provenance provider mismatch"
Assert-Equals -Actual $provenancePolicy.predicate -Expected "slsa_build_provenance" -Message "supply chain provenance predicate mismatch"
Assert-Equals -Actual $provenancePolicy.ci_action -Expected "actions/attest" -Message "supply chain provenance action mismatch"
Assert-Equals -Actual $provenancePolicy.pinned_ref -Expected "281a49d4cbb0a72c9575a50d18f6deb515a11deb" -Message "supply chain provenance action pin mismatch"
foreach ($permission in @("contents: read", "id-token: write", "attestations: write", "artifact-metadata: write")) {
    Assert-JsonArrayContains -Values @($provenancePolicy.required_permissions) -Expected $permission -Message "supply chain provenance permissions"
}
foreach ($subjectPath in @($releaseArtifacts | ForEach-Object { [string] $_.subject_path })) {
    Assert-JsonArrayContains -Values @($provenancePolicy.production_subjects) -Expected $subjectPath -Message "supply chain provenance production subjects"
}

$deployGate = $supplyChainPolicy.deploy_gate

if ($IncludeProductionPromotion) {
    Assert-Equals -Actual $deployGate.required -Expected $true -Message "supply chain deploy gate requirement mismatch"
    Assert-Equals -Actual $deployGate.approved_workflow -Expected ".github/workflows/ci.yml" -Message "supply chain deploy gate workflow mismatch"
    Assert-Equals -Actual $deployGate.approved_job -Expected "supply-chain-provenance" -Message "supply chain deploy gate job mismatch"
    Assert-Equals -Actual $deployGate.candidate_policy -Expected "production_candidates_must_be_built_on_main_by_approved_workflow" -Message "supply chain deploy gate candidate policy mismatch"
    Assert-Contains -Content ([string] $deployGate.verification_command) -Needle "gh attestation verify" -Message "supply chain deploy gate verification command"
    Assert-FileExists -RelativePath ([string] $deployGate.verification_script)
    Assert-FileExists -RelativePath ([string] $deployGate.runbook)
    foreach ($forbiddenDeploy in @(
        "deploy_without_attestation",
        "deploy_from_unapproved_workflow",
        "deploy_from_unverified_subject_digest",
        "mutable_image_tag_without_digest"
    )) {
        Assert-JsonArrayContains -Values @($deployGate.forbidden) -Expected $forbiddenDeploy -Message "supply chain deploy gate forbidden list"
    }

    $deployGateScript = Read-TextFile -RelativePath ([string] $deployGate.verification_script)
    foreach ($needle in @(
        "gh",
        "attestation",
        "verify",
        "RequiredWorkflow",
        "RequiredRef",
        "--predicate-type",
        "production-deploy-candidate-ok"
    )) {
        Assert-Contains -Content $deployGateScript -Needle $needle -Message "supply chain deploy gate verification script"
    }

    Assert-Equals -Actual $deployGate.admission_workflow -Expected ".github/workflows/production-deploy-admission.yml" -Message "supply chain deploy gate admission workflow mismatch"
    Assert-Equals -Actual $deployGate.admission_job -Expected "verify-production-deploy-candidates" -Message "supply chain deploy gate admission job mismatch"
    Assert-Equals -Actual $deployGate.admission_environment -Expected "production" -Message "supply chain deploy gate admission environment mismatch"
    Assert-Equals -Actual $deployGate.download_artifact_action.ci_action -Expected "actions/download-artifact" -Message "supply chain deploy gate download action mismatch"
    Assert-Equals -Actual $deployGate.download_artifact_action.pinned_ref -Expected "3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c" -Message "supply chain deploy gate download action pin mismatch"
    Assert-FileExists -RelativePath ([string] $deployGate.admission_workflow)

    $loadTestCapacityAdmission = Get-JsonProperty -Object $deployGate -Name "load_test_capacity_admission"
    if ($null -eq $loadTestCapacityAdmission) {
        throw "load-test capacity admission missing"
    }
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.required `
        -Expected $true `
        -Message "load-test capacity admission requirement mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.verification_script `
        -Expected "scripts/ci/verify-load-test-capacity-evidence.ps1" `
        -Message "load-test capacity admission verification script mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.evidence_artifact_name `
        -Expected "load-test-capacity-evidence" `
        -Message "load-test capacity admission artifact name mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.workflow_input_run_id `
        -Expected "load-evidence-run-id" `
        -Message "load-test capacity admission run-id input mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.workflow_input_artifact_name `
        -Expected "load-evidence-artifact-name" `
        -Message "load-test capacity admission artifact input mismatch"
    Assert-Equals `
        -Actual $loadTestCapacityAdmission.required_classification `
        -Expected "healthy" `
        -Message "load-test capacity admission classification mismatch"
    Assert-FileExists -RelativePath ([string] $loadTestCapacityAdmission.verification_script)
    foreach ($requiredScenario in @("api-read-mix", "map-marker-mix", "platform-core-events")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.required_scenarios) `
            -Expected $requiredScenario `
            -Message "load-test capacity admission required scenarios"
    }
    foreach ($requiredEnvironment in @("perf", "staging")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.required_environments) `
            -Expected $requiredEnvironment `
            -Message "load-test capacity admission required environments"
    }
    $targetHostByEnvironment = Get-JsonProperty -Object $loadTestCapacityAdmission -Name "target_host_by_environment"
    Assert-Equals `
        -Actual ([string] (Get-JsonProperty -Object $targetHostByEnvironment -Name "perf")) `
        -Expected "perf.gongzzang.internal" `
        -Message "load-test capacity admission perf target host mismatch"
    Assert-Equals `
        -Actual ([string] (Get-JsonProperty -Object $targetHostByEnvironment -Name "staging")) `
        -Expected "staging.gongzzang.internal" `
        -Message "load-test capacity admission staging target host mismatch"
    foreach ($forbiddenEnvironment in @("local", "ci")) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.forbidden_environments) `
            -Expected $forbiddenEnvironment `
            -Message "load-test capacity admission forbidden environments"
    }
    foreach ($forbiddenLoadEvidenceDeploy in @(
        "production_deploy_without_perf_or_staging_load_evidence",
        "local_or_ci_smoke_used_as_launch_capacity_evidence",
        "capacity_evidence_from_production_target"
    )) {
        Assert-JsonArrayContains `
            -Values @($loadTestCapacityAdmission.forbidden) `
            -Expected $forbiddenLoadEvidenceDeploy `
            -Message "load-test capacity admission forbidden list"
    }

    $edgeAdmission = Get-JsonProperty -Object $deployGate -Name "edge_admission"
    if ($null -eq $edgeAdmission) {
        throw "production edge admission missing"
    }
    Assert-Equals -Actual $edgeAdmission.required -Expected $true -Message "production edge admission requirement mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.policy_source `
        -Expected "docs/architecture/traffic-auth-policy-registry.v1.json" `
        -Message "production edge admission policy source mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.generated_waf_manifest `
        -Expected "infrastructure/security/aws-wafv2-edge-policy.generated.json" `
        -Message "production edge admission WAF manifest mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.pulumi_project `
        -Expected "infrastructure/Pulumi.yaml" `
        -Message "production edge admission Pulumi project mismatch"
    Assert-Equals `
        -Actual $edgeAdmission.pulumi_program `
        -Expected "infrastructure/index.ts" `
        -Message "production edge admission Pulumi program mismatch"
    Assert-FileExists -RelativePath ([string] $edgeAdmission.generated_waf_manifest)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.pulumi_project)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.pulumi_program)
    Assert-FileExists -RelativePath ([string] $edgeAdmission.verification_script)

    $regionalAttachment = Get-JsonProperty -Object $edgeAdmission -Name "regional_attachment"
    Assert-Equals -Actual $regionalAttachment.supported -Expected $true -Message "production edge regional attachment support mismatch"
    Assert-Equals `
        -Actual $regionalAttachment.config_key `
        -Expected "wafRegionalResourceArn" `
        -Message "production edge regional attachment config mismatch"
    Assert-Equals `
        -Actual $regionalAttachment.required_env `
        -Expected "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN" `
        -Message "production edge regional attachment env mismatch"
    $pulumiAssociationPreview = Get-JsonProperty -Object $regionalAttachment -Name "pulumi_association_preview"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.required `
        -Expected $true `
        -Message "production edge Pulumi association preview requirement mismatch"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.preview_script `
        -Expected "scripts/ci/check-pulumi-local-preview.ps1" `
        -Message "production edge Pulumi association preview script mismatch"
    Assert-Equals `
        -Actual $pulumiAssociationPreview.evidence `
        -Expected "regional_association=planned" `
        -Message "production edge Pulumi association preview evidence mismatch"
    Assert-FileExists -RelativePath ([string] $pulumiAssociationPreview.preview_script)
    $cloudfrontAttachment = Get-JsonProperty -Object $edgeAdmission -Name "cloudfront_attachment"
    Assert-Equals `
        -Actual $cloudfrontAttachment.supported `
        -Expected $false `
        -Message "production edge CloudFront attachment support mismatch"
    Assert-Equals `
        -Actual $cloudfrontAttachment.required_before_production `
        -Expected $true `
        -Message "production edge CloudFront pre-production requirement mismatch"
    foreach ($forbiddenEdgeDeploy in @(
        "production_deploy_without_waf_attachment",
        "edge_policy_not_from_traffic_auth_registry",
        "manual_waf_console_change"
    )) {
        Assert-JsonArrayContains `
            -Values @($edgeAdmission.forbidden) `
            -Expected $forbiddenEdgeDeploy `
            -Message "production edge admission forbidden list"
    }

    $edgeAdmissionScript = Read-TextFile -RelativePath ([string] $edgeAdmission.verification_script)
    foreach ($needle in @(
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "wafRegionalResourceArn",
        "aws-wafv2-edge-policy.generated.json",
        "RequirePulumiAssociationPreview",
        "check-pulumi-local-preview.ps1",
        "regional_association=planned",
        "production-edge-admission-ok",
        "must be an AWS ARN"
    )) {
        Assert-Contains -Content $edgeAdmissionScript -Needle $needle -Message "production edge admission verification script"
    }

    $loadTestCapacityAdmissionScript = Read-TextFile -RelativePath ([string] $loadTestCapacityAdmission.verification_script)
    foreach ($needle in @(
        "EvidenceRoot",
        "run.json",
        "spec.json",
        "k6-summary.json",
        "Classification: healthy",
        "capacity evidence environment must be perf or staging",
        "profile must be baseline, stress, spike, or soak",
        "production targets are not valid load-test capacity evidence",
        "target host must match capacity evidence environment",
        "missing required load-test capacity scenario",
        "RequiredScenarios",
        "load-test-capacity-evidence-ok"
    )) {
        Assert-Contains -Content $loadTestCapacityAdmissionScript -Needle $needle -Message "load-test capacity admission verification script"
    }

    $deployGateWorkflow = Read-TextFile -RelativePath ([string] $deployGate.admission_workflow)
    foreach ($needle in @(
        "workflow_call:",
        "workflow_dispatch:",
        "verify-production-deploy-candidates:",
        "environment: production",
        "actions: read",
        "attestations: read",
        "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
        "pnpm install --frozen-lockfile",
        "verify-production-deploy-candidate.ps1",
        "check-production-edge-admission.ps1",
        "verify-load-test-capacity-evidence.ps1",
        "load-evidence-run-id",
        "load-evidence-artifact-name",
        "Download load-test capacity evidence",
        "Verify load-test capacity evidence",
        "target/admission/load-test-capacity-evidence",
        "-RequirePulumiAssociationPreview",
        "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN",
        "-PredicateType https://cyclonedx.org/bom",
        "run-id"
    )) {
        Assert-Contains -Content $deployGateWorkflow -Needle $needle -Message "supply chain deploy admission workflow"
    }
    foreach ($releaseArtifact in $releaseArtifacts) {
        Assert-Contains -Content $deployGateWorkflow -Needle ([string] $releaseArtifact.subject_path) -Message "supply chain deploy admission subject path"
    }
}
