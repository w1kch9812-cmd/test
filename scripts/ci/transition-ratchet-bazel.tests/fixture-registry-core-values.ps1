    $sunset = if ($ExpiredSunset) { "2020-01-01" } else { "2026-07-31" }
    $nodeAuditApprovalGates = if ($MissingAdvisoryApprovalGate) { "[]" } elseif ($UnknownApprovalGate) { '["typo_gate"]' } else { '["external_advisory_collection"]' }
    $approvalGatesLine = if ($MissingApprovalGates) { "" } else { "`"approval_gates`": []," }
    $frontendE2eApprovalGates = if ($MissingBrowserRuntimeGate) { "[]" } else { '["browser_runtime_provisioning"]' }
    $rustCheckExitTarget = if ($InvalidExitTarget) { "rust_verification" } elseif ($TransitionExitTarget) { "//tools/bazel:next_transition" } else { "//:rust_verification" }
    $rustCheckRunnerTaskLine = if ($MissingRunnerTask) { "" } elseif ($MismatchedRunnerTask) { '"runner_task": "rustfmt-check",' } else { '"runner_task": "rust-check",' }
    $nodeAuditRequiredCommands = if ($MissingRequiredCommand) { "[]" } else { '["pnpm"]' }
    $migrationRequiredServices = if ($MissingRequiredService) { "[]" } else { '["postgres"]' }
    $exitStateLine = if ($MissingExitState) { "" } elseif ($UnknownExitState) { '"exit_state": "done",' } elseif ($AvailableMissingExitTarget) { '"exit_state": "ready_to_retire",' } else { '"exit_state": "blocked",' }
    $rustCheckEvidenceRequirements = if ($MissingExitEvidenceRequirements) { "[]" } else { '["native_bazel_test_target"]' }
    $nodeAuditBlockingApprovalGates = if ($MissingBlockingApprovalGate) { "[]" } else { '["external_advisory_collection"]' }
    $dependencyScaExitEvidenceRequirements = if ($MismatchedExitTargetEvidence) { '["native_bazel_evidence_target"]' } else { '["native_bazel_evidence_target", "pinned_advisory_evidence"]' }
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
