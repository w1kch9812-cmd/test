    if (!$OmitDeployAdmissionWorkflow) {
        $productionEdgeAdmissionWorkflow = if ($OmitProductionEdgeAdmission) {
            ""
        } else {
            @'
Production edge admission
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
check-production-edge-admission.ps1
-RequirePulumiAssociationPreview
pnpm install --frozen-lockfile
'@
        }
        $loadEvidenceAdmissionWorkflow = if ($OmitLoadEvidenceAdmission) {
            ""
        } else {
            @'
load-evidence-run-id
load-evidence-artifact-name
Download load-test capacity evidence
Verify load-test capacity evidence
verify-load-test-capacity-evidence.ps1
target/admission/load-test-capacity-evidence
'@
        }
        Write-File -Root $Root -RelativePath ".github\workflows\production-deploy-admission.yml" -Content (@'
workflow_call:
workflow_dispatch:
verify-production-deploy-candidates:
environment: production
bazel-bin/gongzzang-web-next-build.tgz
bazel-bin/gongzzang-api-release/api
actions: read
attestations: read
actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c
verify-production-deploy-candidate.ps1
-PredicateType https://cyclonedx.org/bom
run-id
'@ + "`n$loadEvidenceAdmissionWorkflow`n$productionEdgeAdmissionWorkflow`n")
    }
    if (!$OmitDeployCandidateVerifier) {
        Write-File -Root $Root -RelativePath "scripts\ci\verify-production-deploy-candidate.ps1" -Content @'
gh
attestation
verify
RequiredWorkflow
RequiredRef
--predicate-type
production-deploy-candidate-ok
'@
    }
    if (!$OmitProductionEdgeAdmission) {
        Write-File -Root $Root -RelativePath "scripts\ci\check-production-edge-admission.ps1" -Content @'
GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
wafRegionalResourceArn
aws-wafv2-edge-policy.generated.json
RequirePulumiAssociationPreview
check-pulumi-local-preview.ps1
regional_association=planned
production-edge-admission-ok
must be an AWS ARN
'@
        Write-File -Root $Root -RelativePath "scripts\ci\check-pulumi-local-preview.ps1" -Content @'
regional_association=planned
'@
        Write-File -Root $Root -RelativePath "infrastructure\security\aws-wafv2-edge-policy.generated.json" -Content @'
{"schema_version":"gongzzang.aws_wafv2_edge_policy_manifest.v1"}
'@
        Write-File -Root $Root -RelativePath "infrastructure\Pulumi.yaml" -Content @'
runtime: nodejs
'@
        Write-File -Root $Root -RelativePath "infrastructure\index.ts" -Content @'
wafRegionalResourceArn
aws.wafv2.WebAclAssociation
'@
    }
    if (!$OmitLoadEvidenceAdmission) {
        Write-File -Root $Root -RelativePath "scripts\ci\verify-load-test-capacity-evidence.ps1" -Content @'
EvidenceRoot
run.json
spec.json
k6-summary.json
Classification: healthy
load-test-capacity-evidence-ok
capacity evidence environment must be perf or staging
profile must be baseline, stress, spike, or soak
production targets are not valid load-test capacity evidence
target host must match capacity evidence environment
missing required load-test capacity scenario
RequiredScenarios
'@
    }
