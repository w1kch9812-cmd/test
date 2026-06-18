    $sunset = if ($ExpiredSunset) { "2020-01-01" } else { "2026-07-31" }
    $nodeAuditApprovalGates = if ($MissingAdvisoryApprovalGate) { "[]" } elseif ($UnknownApprovalGate) { '["typo_gate"]' } else { '["external_advisory_collection"]' }
    $approvalGatesLine = if ($MissingApprovalGates) { "" } else { "`"approval_gates`": []," }
    $frontendE2eApprovalGates = if ($MissingBrowserRuntimeGate) { "[]" } else { '["browser_runtime_provisioning"]' }
    $frontendE2eEvidenceRequirements = '["browser_runtime_provisioning_decision", "native_bazel_test_target"]'
    $rustCheckExitTarget = if ($InvalidExitTarget) { "rust_verification" } elseif ($TransitionExitTarget) { "//tools/bazel:next_transition" } else { "//:rust_verification" }
    $rustCheckRunnerTaskLine = if ($MissingRunnerTask) { "" } elseif ($MismatchedRunnerTask) { '"runner_task": "rustfmt-check",' } else { '"runner_task": "rust-check",' }
    $nodeAuditRequiredCommands = if ($MissingRequiredCommand) { "[]" } else { '["pnpm"]' }
    $migrationRequiredServices = if ($MissingRequiredService) { "[]" } else { '["postgres"]' }
    $exitStateLine = if ($MissingExitState) { "" } elseif ($UnknownExitState) { '"exit_state": "done",' } elseif ($AvailableMissingExitTarget) { '"exit_state": "ready_to_retire",' } else { '"exit_state": "blocked",' }
    $rustCheckEvidenceRequirements = if ($MissingExitEvidenceRequirements) { "[]" } else { '["native_bazel_test_target"]' }
    $nodeAuditBlockingApprovalGates = if ($MissingBlockingApprovalGate) { "[]" } else { '["external_advisory_collection"]' }
    $dependencyScaExitEvidenceRequirements = if ($MismatchedExitTargetEvidence) { '["native_bazel_evidence_target"]' } else { '["native_bazel_evidence_target", "pinned_advisory_evidence"]' }
    $dependencyScaBlockingApprovalGates = if ($MismatchedExitTargetEvidence) {
        "[]"
    } elseif ($ExtraUncoveredExitBlockingGate) {
        '["external_advisory_collection", "browser_runtime_provisioning"]'
    } else {
        '["external_advisory_collection"]'
    }
    $nativeEvidencePlannedTarget = if ($AvailableMissingEvidenceTarget) {
        "//:missing_supply_chain_evidence"
    } else {
        "//:verify_supply_chain"
    }
    $rustfmtPlannedTarget = if ($AvailableMissingExitTarget) {
        "//:rust_verification"
    } else {
        "//tools/bazel:rustfmt_check"
    }
    $pinnedAdvisoryPlannedTarget = if ($InvalidPlannedExitEvidenceTarget) {
        "not-a-bazel-label"
    } elseif ($TransitionPlannedExitEvidenceTarget) {
        "//tools/bazel:ci_node_audit_transition"
    } else {
        "//:pinned_advisory_evidence"
    }
    $externalAdvisoryCategoryEvidence = if ($MismatchedCategoryEvidence) { '["native_bazel_database_test"]' } else { '["native_bazel_evidence_target", "pinned_advisory_evidence"]' }
    $nativeBazelEvidenceKindEntry = if ($MissingRegisteredEvidenceKind) {
        ""
    } else {
        @'
    {
      "id": "native_bazel_evidence",
      "owner": "build-platform",
      "reason": "fixture"
    },
'@
    }
    $duplicateEvidenceKindEntry = if ($DuplicateEvidenceKindRegistry) {
        @'
    {
      "id": "provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredEvidenceKinds = if ($MissingEvidenceKindRegistry) {
        ""
    } else {
        @"
  "evidence_kind_registry": [
$nativeBazelEvidenceKindEntry    {
      "id": "pinned_external_evidence",
      "owner": "build-platform",
      "reason": "fixture"
    },
$duplicateEvidenceKindEntry    {
      "id": "provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture"
    }
  ],
"@
    }
    $nativeBazelTestEvidenceEntry = if ($MissingRegisteredExitEvidenceRequirement) {
        ""
    } else {
        @'
    {
      "id": "native_bazel_test_target",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
'@
    }
    $duplicateExitEvidenceRequirementEntry = if ($DuplicateExitEvidenceRequirementRegistry) {
        @'
    {
      "id": "native_bazel_database_test",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "evidence_kind": "native_bazel_evidence"
    },
'@
    } else {
        ""
    }
    $registeredExitEvidenceRequirements = if ($MissingExitEvidenceRequirementRegistry) {
        ""
    } else {
        @"
  "exit_evidence_requirement_registry": [
    {
      "id": "browser_runtime_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    },
    {
      "id": "database_service_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    },
    {
      "id": "native_bazel_coverage_evidence",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
$duplicateExitEvidenceRequirementEntry    {
      "id": "native_bazel_database_test",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
    {
      "id": "native_bazel_evidence_target",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
    {
      "id": "native_bazel_service_orchestration",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
$nativeBazelTestEvidenceEntry    {
      "id": "pinned_advisory_evidence",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "pinned_external_evidence"
    },
    {
      "id": "service_orchestration_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    },
    {
      "id": "toolchain_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    }
  ],
"@
    }
    $nativeTestTargetMissingBlockerEntry = if ($MissingRegisteredPlannedEvidenceBlocker) {
        ""
    } else {
        @'
    {
      "id": "native_bazel_test_target_missing",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirement": "native_bazel_test_target"
    },
'@
    }
    $duplicatePlannedEvidenceBlockerEntry = if ($DuplicatePlannedEvidenceBlockerRegistry) {
        @'
    {
      "id": "native_bazel_database_test_missing",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "exit_evidence_requirement": "native_bazel_database_test"
    },
'@
    } else {
        ""
    }
    $browserRuntimeDecisionBlockerEntry = if ($MissingRegisteredApprovalGate) {
        ""
    } else {
        @'
    {
      "id": "browser_runtime_provisioning_decision_required",
      "owner": "build-platform",
      "reason": "fixture",
      "approval_gate": "browser_runtime_provisioning"
    },
'@
    }
    $registeredPlannedEvidenceBlockers = if ($MissingPlannedEvidenceBlockerRegistry) {
        ""
    } else {
        @"
  "planned_evidence_blocker_registry": [
    {
      "id": "external_advisory_collection_required",
      "owner": "build-platform",
      "reason": "fixture",
      "approval_gate": "external_advisory_collection"
    },
    {
      "id": "database_service_provisioning_decision_required",
      "owner": "build-platform",
      "reason": "fixture",
      "approval_gate": "database_service_provisioning"
    },
    {
      "id": "toolchain_provisioning_decision_required",
      "owner": "build-platform",
      "reason": "fixture",
      "approval_gate": "toolchain_provisioning"
    },
$browserRuntimeDecisionBlockerEntry$duplicatePlannedEvidenceBlockerEntry    {
      "id": "native_bazel_database_test_missing",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirement": "native_bazel_database_test"
    },
$nativeTestTargetMissingBlockerEntry    {
      "id": "native_bazel_coverage_target_missing",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirement": "native_bazel_coverage_evidence"
    }
  ],
"@
    }
    $dependencyScaNativeEvidenceTargetEntry = if ($MissingRegisteredExitTarget) {
        ""
    } else {
        @"
    {
      "exit_target": "//:dependency_sca_evidence",
      "requirement": "native_bazel_evidence_target",
      "planned_bazel_target": "$nativeEvidencePlannedTarget",
      "owner": "build-platform",
      "reason": "fixture"
    },
"@
    }
    $dependencyScaPinnedEvidenceTargetEntry = if (
        $MissingRegisteredExitTarget -or
        $MissingRegisteredExitEvidenceTarget -or
        $MismatchedExitTargetEvidence
    ) {
        ""
    } else {
        @"
    {
      "exit_target": "//:dependency_sca_evidence",
      "requirement": "pinned_advisory_evidence",
      "planned_bazel_target": "$pinnedAdvisoryPlannedTarget",
      "owner": "build-platform",
      "reason": "fixture"
    },
"@
    }
    $registeredExitEvidenceTargets = if ($MissingExitEvidenceTargetRegistry) {
        ""
    } else {
        @"
  "exit_evidence_target_registry": [
$dependencyScaNativeEvidenceTargetEntry$dependencyScaPinnedEvidenceTargetEntry    {
      "exit_target": "//:rust_verification",
      "requirement": "native_bazel_test_target",
      "planned_bazel_target": "//:rust_verification",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//tools/bazel:rustfmt_check",
      "requirement": "native_bazel_test_target",
      "planned_bazel_target": "$rustfmtPlannedTarget",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//:frontend_e2e",
      "requirement": "browser_runtime_provisioning_decision",
      "planned_bazel_target": "//:frontend_e2e_browser_runtime_evidence",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//:frontend_e2e",
      "requirement": "native_bazel_test_target",
      "planned_bazel_target": "//:frontend_e2e",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//:migration_verification",
      "requirement": "database_service_provisioning_decision",
      "planned_bazel_target": "//:migration_database_service_provisioning_evidence",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//:migration_verification",
      "requirement": "native_bazel_database_test",
      "planned_bazel_target": "//:migration_verification",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "exit_target": "//:migration_verification",
      "requirement": "toolchain_provisioning_decision",
      "planned_bazel_target": "//:migration_toolchain_provisioning_evidence",
      "owner": "build-platform",
      "reason": "fixture"
    }
  ],
"@
    }
