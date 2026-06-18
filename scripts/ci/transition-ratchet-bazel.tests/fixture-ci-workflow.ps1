    $extraCi = if ($UntrackedCiTransition) {
        "      - run: bazelisk test //tools/bazel:unknown_transition --config=ci"
    } else {
        ""
    }
    $frontendE2eCi = if ($UnreferencedTransitionPolicy) {
        ""
    } else {
        "      - run: bazelisk test //tools/bazel:frontend_e2e_transition --config=ci"
    }
    $workflowPnpmInstall = if ($MissingWorkflowCommandProvisioning) { "" } else { "      - run: pnpm install --frozen-lockfile" }
    $workflowPostgresService = if ($MissingWorkflowServiceProvisioning) {
        ""
    } else {
        @'
    services:
      postgres:
        image: postgis/postgis:17-3.5
'@
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
jobs:
  verify:
$workflowPostgresService
    steps:
      - uses: pnpm/action-setup@0e279bb959325dab635dd2c09392533439d90093
$workflowPnpmInstall
      - uses: dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17
      - run: |
          sudo apt-get update -qq
          sudo apt-get install -y postgresql-client
          cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls
      - run: bazelisk test //tools/bazel:ci_node_audit_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_rust_check_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_rustfmt_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_migration_v001_full_transition --config=ci
$frontendE2eCi
$extraCi
"@
