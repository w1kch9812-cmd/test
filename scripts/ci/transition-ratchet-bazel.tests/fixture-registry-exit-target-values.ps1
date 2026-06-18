    $deletedExitTargetRegistryEntry = if ($AddStalePolicy) {
        @'
,
    {
      "bazel_target": "//:deleted",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": [],
      "blocking_approval_gates": []
    }
'@
    } else {
        ""
    }
    $registeredExitTargets = if ($MissingExitTargetRegistry) {
        ""
    } elseif ($MissingRegisteredExitTarget) {
        @"
  "exit_targets": [
    {
      "bazel_target": "//:rust_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
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
    },
    {
      "bazel_target": "//:rust_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
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
$exitStateLine
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
