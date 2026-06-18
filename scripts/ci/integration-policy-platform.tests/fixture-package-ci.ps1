    $viteOverride = if ($OmitViteOverride) { "" } else { ',"vite":"6.4.2"' }
    Write-File -Root $Root -RelativePath "package.json" -Content @"
{
  "pnpm": {
    "overrides": {
      "brace-expansion": "5.0.6",
      "postcss": "8.5.15"$viteOverride
    }
  }
}
"@
    Write-File -Root $Root -RelativePath "deny.toml" -Content "deny"
    Write-File -Root $Root -RelativePath ".gitleaks.toml" -Content "gitleaks"
    foreach ($guardrailScript in @(
        "scripts\ci\check-platform-integration-policy.ps1",
        "scripts\ci\check-lakehouse-registry-integration.ps1",
        "scripts\ci\check-traffic-auth-policy-registry.ps1",
        "scripts\ci\check-platform-core-boundary.ps1",
        "scripts\ci\check-platform-core-event-receiver-contract.ps1",
        "scripts\ci\check-platform-core-catalog-api-contract.ps1",
        "scripts\ci\check-platform-core-dependency-boundary.ps1",
        "scripts\ci\check-pnu-anchor-pbf-marker-contract.ps1",
        "scripts\ci\check-migration-version-prefixes.ps1",
        "scripts\ci\check-platform-core-anchor-inbox-db-approval.ps1",
        "scripts\ci\check-load-test-assets.ps1",
        "scripts\ci\verify-load-test-capacity-evidence.ps1"
    )) {
        Write-File -Root $Root -RelativePath $guardrailScript -Content "guardrail"
    }
    Write-File -Root $Root -RelativePath "tools\bazel\generate_release_file_sbom.sh" -Content "generate-release-file-sbom"
    $integrationCi = if ($OmitCiWiring) { "" } else { "check-platform-integration-policy.ps1" }
    $lakehouseRegistryCi = if ($OmitCiWiring) { "" } else {
        "check-lakehouse-registry-integration.ps1`ncheck-lakehouse-registry-integration.tests.ps1"
    }
    $migrationPrefixCi = if ($OmitMigrationPrefixCi) { "" } else { "check-migration-version-prefixes.ps1" }
    $supplyChainCi = if ($OmitSupplyChainCi) {
        ""
    } else {
        @"
supply-chain-provenance:
id-token: write
attestations: write
artifact-metadata: write
//:supply_chain_evidence_artifacts
//:verify_supply_chain
actions/attest@281a49d4cbb0a72c9575a50d18f6deb515a11deb
path: bazel-bin/apps/web/.next
path: bazel-bin/gongzzang-api-release
bazel-bin/supply-chain/evidence-manifest.json
//:web_release_sbom
//:api_release_sbom
subject-path: bazel-bin/gongzzang-web-next-build.tgz
subject-path: bazel-bin/gongzzang-api-release/api
sbom-path: bazel-bin/supply-chain/gongzzang-node-workspace-sbom.cdx.json
sbom-path: bazel-bin/supply-chain/gongzzang-rust-workspace-sbom.cdx.json
check-production-edge-admission.ps1
$migrationPrefixCi
check-platform-core-anchor-inbox-db-approval.ps1
check-load-test-assets.ps1
verify-load-test-capacity-evidence.tests.ps1
"@
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
//tools/bazel:ci_node_audit_transition
//tools/bazel:ci_cargo_deny_transition
//:web_release_candidate_archive
//:api_release_candidate_binary
gitleaks-action
$integrationCi
$lakehouseRegistryCi
$supplyChainCi
"@
