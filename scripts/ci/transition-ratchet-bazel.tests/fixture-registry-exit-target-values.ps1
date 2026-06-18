    $deletedExitTargetRegistryEntry = if ($AddStalePolicy) {
        @'
,
    {
      "bazel_target": "//:deleted",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": [],
      "blocking_approval_gates": [],
      "evidence_status": []
    }
'@
    } else {
        ""
    }
    $pinnedAdvisoryBlockedBy = if ($MissingPlannedEvidenceBlockedBy) {
        ""
    } else {
        @'
,
          "blocked_by": ["external_advisory_collection_required"]
'@
    }
    $nativeTestBlockedBy = if ($MissingPlannedEvidenceBlockedBy) {
        ""
    } else {
        @'
,
          "blocked_by": ["native_bazel_test_target_missing"]
'@
    }
    $databaseServiceDecisionBlockedBy = if ($MissingPlannedEvidenceBlockedBy) {
        ""
    } else {
        @'
,
          "blocked_by": ["database_service_provisioning_decision_required"]
'@
    }
    $nativeBazelDatabaseBlockedBy = if ($MissingPlannedEvidenceBlockedBy) {
        ""
    } else {
        @'
,
          "blocked_by": ["native_bazel_database_test_missing"]
'@
    }
    $rustVerificationExitTargetState = if ($AvailableMissingExitTarget) { "available" } else { "planned" }
    $dependencyScaEvidenceStatus = if ($MissingExitTargetEvidenceStatus) {
        ""
    } elseif ($AvailableMissingEvidenceTarget) {
        @"
,
      "evidence_status": [
        {
          "requirement": "native_bazel_evidence_target",
          "state": "available",
          "bazel_target": "//:missing_supply_chain_evidence",
          "reason": "fixture"
        },
        {
          "requirement": "pinned_advisory_evidence",
          "state": "planned",
          "reason": "fixture"
$pinnedAdvisoryBlockedBy
        }
      ]
"@
    } elseif ($MismatchedExitTargetEvidence) {
        @"
,
      "evidence_status": [
        {
          "requirement": "native_bazel_evidence_target",
          "state": "available",
          "bazel_target": "//:verify_supply_chain",
          "reason": "fixture"
        }
      ]
"@
    } else {
        @"
,
      "evidence_status": [
        {
          "requirement": "native_bazel_evidence_target",
          "state": "available",
          "bazel_target": "//:verify_supply_chain",
          "reason": "fixture"
        },
        {
          "requirement": "pinned_advisory_evidence",
          "state": "planned",
          "reason": "fixture"
$pinnedAdvisoryBlockedBy
        }
      ]
"@
    }
    $nativeTestEvidenceStatus = @"
,
      "evidence_status": [
        {
          "requirement": "native_bazel_test_target",
          "state": "planned",
          "reason": "fixture"
$nativeTestBlockedBy
        }
      ]
"@
    $rustVerificationEvidenceStatus = if ($AvailableMissingExitTarget) {
        @'
,
      "evidence_status": [
        {
          "requirement": "native_bazel_test_target",
          "state": "available",
          "bazel_target": "//:rust_verification",
          "reason": "fixture"
        }
      ]
'@
    } else {
        $nativeTestEvidenceStatus
    }
    $databaseEvidenceStatus = @"
,
      "evidence_status": [
        {
          "requirement": "database_service_provisioning_decision",
          "state": "planned",
          "reason": "fixture"
$databaseServiceDecisionBlockedBy
        },
        {
          "requirement": "native_bazel_database_test",
          "state": "planned",
          "reason": "fixture"
$nativeBazelDatabaseBlockedBy
        }
      ]
"@
    $registeredExitTargets = if ($MissingExitTargetRegistry) {
        ""
    } elseif ($MissingRegisteredExitTarget) {
        @"
  "exit_targets": [
    {
      "bazel_target": "//:rust_verification",
      "state": "$rustVerificationExitTargetState",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
$rustVerificationEvidenceStatus
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
$rustVerificationEvidenceStatus
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
$nativeTestEvidenceStatus
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
$databaseEvidenceStatus
    }
$deletedExitTargetRegistryEntry
  ],
"@
    } else {
        @"
  "exit_targets": [
    {
      "bazel_target": "//:dependency_sca_evidence",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": $dependencyScaExitEvidenceRequirements,
      "blocking_approval_gates": ["external_advisory_collection"]
$dependencyScaEvidenceStatus
    },
    {
      "bazel_target": "//:rust_verification",
      "state": "$rustVerificationExitTargetState",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
$rustVerificationEvidenceStatus
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
$rustVerificationEvidenceStatus
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
$nativeTestEvidenceStatus
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
$databaseEvidenceStatus
    }
$deletedExitTargetRegistryEntry
  ],
"@
    }
    $nodeAuditPolicy = if ($OmitNodeAuditPolicy) {
        ""
    } else {
        $externalCollection = if ($MissingExternalCollectionFlag) { "false" } else { "true" }
        @"
    {
      "bazel_target": "//tools/bazel:ci_node_audit_transition",
      "category": "external-advisory-sca",
      "owner": "build-platform",
      "reason": "pnpm audit still shells out until advisory SCA is represented by a pinned Bazel evidence target.",
      "exit_target": "//:dependency_sca_evidence",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["native_bazel_evidence_target", "pinned_advisory_evidence"],
      "blocking_approval_gates": $nodeAuditBlockingApprovalGates,
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "node-audit",
      "required_commands": $nodeAuditRequiredCommands,
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": $nodeAuditApprovalGates,
      "external_collection_approval_required": $externalCollection
    },
"@
    }
    $stalePolicy = if ($AddStalePolicy) {
        @'
    {
      "bazel_target": "//tools/bazel:deleted_transition",
      "category": "stale-fixture",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//:deleted",
      "exit_state": "blocked",
      "exit_evidence_requirements": [],
      "blocking_approval_gates": [],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "deleted",
      "required_commands": [],
      "required_services": [],
      "sunset": "2026-07-31",
      "approval_gates": [],
      "external_collection_approval_required": false
    },
'@
    } else {
        ""
    }
    $retiredTargets = if ($RetiredRustfmtTransition) {
        @'
  "retired_transition_targets": [
    "//tools/bazel:ci_rustfmt_transition"
  ],
'@
    } else {
        @'
  "retired_transition_targets": [],
'@
    }
