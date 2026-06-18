    $fixtureDecisionReference = "docs/adr/0043-bazel-transition-provisioning-decisions.md"
    $browserRuntimeDecisionReference = if ($InvalidApprovalGateDecisionReference) {
        "fixture"
    } else {
        $fixtureDecisionReference
    }
    $externalAdvisoryGateEntry = @"
    {
      "id": "external_advisory_collection",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "$fixtureDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": true
    },
"@
    $browserRuntimeGateEntry = if ($MissingRegisteredApprovalGate) {
        ""
    } else {
        @"
    {
      "id": "browser_runtime_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "$browserRuntimeDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
"@
    }
    $duplicateApprovalGateEntry = if ($DuplicateApprovalGateRegistry) {
        @"
    {
      "id": "toolchain_provisioning",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "decision_reference": "$fixtureDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
"@
    } else {
        ""
    }
    $registeredApprovalGates = if ($MissingApprovalGateRegistry) {
        ""
    } else {
        @"
  "approval_gate_registry": [
$externalAdvisoryGateEntry$browserRuntimeGateEntry$duplicateApprovalGateEntry
    {
      "id": "toolchain_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "$fixtureDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
    {
      "id": "database_service_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "$fixtureDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
    {
      "id": "service_orchestration_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "$fixtureDecisionReference",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    }
  ],
"@
    }
    $frontendReleaseCategoryEntry = if ($MissingRegisteredTransitionCategory) {
        ""
    } else {
        @'
    {
      "id": "frontend-release-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["browser_runtime_provisioning_decision", "native_bazel_test_target"],
      "required_approval_gates": ["browser_runtime_provisioning"],
      "external_collection_approval_required": false
    },
'@
    }
    $registeredTransitionCategories = if ($MissingTransitionCategoryRegistry) {
        ""
    } else {
        @"
  "transition_category_registry": [
    {
      "id": "external-advisory-sca",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": $externalAdvisoryCategoryEvidence,
      "required_approval_gates": ["external_advisory_collection"],
      "external_collection_approval_required": true
    },
    {
      "id": "rust-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["native_bazel_test_target"],
      "required_approval_gates": [],
      "external_collection_approval_required": false
    },
$frontendReleaseCategoryEntry    {
      "id": "database-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test", "toolchain_provisioning_decision"],
      "required_approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "external_collection_approval_required": false
    },
    {
      "id": "stale-fixture",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": [],
      "required_approval_gates": [],
      "external_collection_approval_required": false
    }
  ],
"@
    }
