    Write-File -Root $Root -RelativePath "docs\architecture\verification-transition-ratchet.v1.json" -Content @"
{
  "schema_version": "gongzzang.verification_transition_ratchet.v1",
  "repo_slug": "gongzzang",
  "default_decision": "deny_new_transition_without_policy",
$retiredTargets
$registeredEvidenceKinds
$registeredApprovalGates
$registeredExitEvidenceRequirements
$registeredPlannedEvidenceBlockers
$registeredTransitionCategories
$registeredRequiredCommands
$registeredRequiredServices
$registeredRunnerTasks
$registeredExitTargetStates
$registeredExitTargets
$registeredTransitionExitStates
  "transition_targets": [
$nodeAuditPolicy$stalePolicy    {
      "bazel_target": "//tools/bazel:ci_rust_check_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "cargo check transition until Rust check is a native Bazel rule target.",
      "exit_target": "$rustCheckExitTarget",
$exitStateLine
      "exit_evidence_requirements": $rustCheckEvidenceRequirements,
      "blocking_approval_gates": [],
$rustCheckRunnerTaskLine
      "runner_script": "run_ci_transition_task.sh",
      "required_commands": ["cargo"],
      "required_services": [],
      "sunset": "$sunset",
$approvalGatesLine
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:ci_rustfmt_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//tools/bazel:rustfmt_check",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": [],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "rustfmt-check",
      "required_commands": ["cargo"],
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": [],
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:frontend_e2e_transition",
      "category": "frontend-release-verification",
      "owner": "build-platform",
      "reason": "Playwright transition retained until browser provisioning and e2e execution are native Bazel targets.",
      "exit_target": "//:frontend_e2e",
      "exit_state": "blocked",
      "exit_evidence_requirements": $frontendE2eEvidenceRequirements,
      "blocking_approval_gates": $frontendE2eApprovalGates,
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "frontend-e2e",
      "required_commands": ["pnpm"],
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": $frontendE2eApprovalGates,
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:ci_migration_v001_full_transition",
      "category": "database-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//:migration_verification",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test", "toolchain_provisioning_decision"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "migration-v001-full",
      "required_commands": ["pg_isready", "psql", "sqlx"],
      "required_services": $migrationRequiredServices,
      "sunset": "$sunset",
      "approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "external_collection_approval_required": false
    }
  ]
}
"@
    Write-File `
        -Root $Root `
        -RelativePath "docs\adr\0043-bazel-transition-provisioning-decisions.md" `
        -Content "# ADR-0043: Fixture Bazel Transition Provisioning Decisions`n"
