# Verification Task Registry SSOT Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make guardrail execution metadata single-source by introducing a `verification-task-registry` that enforces Bazel, lefthook, and GitHub Actions wiring from one declarative policy document.
The registry must be bidirectional: every registered task must be projected, and every Bazel
guardrail target, root guardrail suite label, and `run_guardrail_task.sh` case must be registered.

**Architecture:** Add `docs/architecture/verification-task-registry.v1.json` as the authoritative task registry. Add a PowerShell guardrail that reads the registry and validates the existing projections in `tools/bazel/BUILD.bazel`, root `BUILD.bazel`, `tools/bazel/run_guardrail_task.sh`, `lefthook.yml`, and `.github/workflows/ci.yml`.

**Tech Stack:** PowerShell 5.1-compatible guardrails, Bazel `transition_shell_test`, lefthook, GitHub Actions YAML text validation.

---

## Task 1: Registry Guardrail Tests

**Files:**
- Create: `scripts/ci/check-verification-task-registry.tests.ps1`
- Create later: `scripts/ci/check-verification-task-registry.ps1`

- [x] **Step 1: Write the failing test**

Add a PowerShell test runner that builds fixture repos and asserts:

```powershell
$success = Invoke-Checker -Root $successRoot
Assert-Equals $success.ExitCode 0 "success exit code mismatch"
Assert-Contains $success.Output "verification-task-registry-ok"

$missingBazelTarget = Invoke-Checker -Root $missingBazelTargetRoot
Assert-Equals $missingBazelTarget.ExitCode 1 "missing Bazel target exit code mismatch"
Assert-Contains $missingBazelTarget.Output "Bazel guardrail target is missing"

$missingRunGuardrailCase = Invoke-Checker -Root $missingRunGuardrailCaseRoot
Assert-Equals $missingRunGuardrailCase.ExitCode 1 "missing runner case exit code mismatch"
Assert-Contains $missingRunGuardrailCase.Output "run_guardrail_task.sh missing task case"
```

- [x] **Step 2: Run test to verify it fails**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-task-registry.tests.ps1`

Expected: FAIL because `scripts/ci/check-verification-task-registry.ps1` does not exist yet.

## Task 2: Registry Document And Checker

**Files:**
- Create: `docs/architecture/verification-task-registry.v1.json`
- Create: `scripts/ci/check-verification-task-registry.ps1`

- [x] **Step 1: Add registry**

Create a JSON registry with:

```json
{
  "schema_version": "gongzzang.verification_task_registry.v1",
  "repo_slug": "gongzzang",
  "tasks": [
    {
      "id": "coverage-transition-ssot",
      "bazel_target": "//tools/bazel:guardrail_coverage_transition_ssot",
      "bazel_suite": "policy",
      "script": "scripts/ci/check-coverage-transition-ssot.ps1",
      "shell": "powershell",
      "root_argument": true,
      "lefthook": { "pre_commit": true, "pre_push": true },
      "ci": { "required": true, "run": "./scripts/ci/check-coverage-transition-ssot.ps1" }
    }
  ]
}
```

- [x] **Step 2: Add checker**

Implement `check-verification-task-registry.ps1` to:

```powershell
$registry = Read-JsonFile -RelativePath "docs/architecture/verification-task-registry.v1.json"
Assert-Equals $registry.schema_version "gongzzang.verification_task_registry.v1" "schema mismatch"
foreach ($task in @($registry.tasks)) {
    Assert-String $task.id "task.id"
    Assert-String $task.bazel_target "task.bazel_target"
    Assert-String $task.script "task.script"
}
```

Then validate the registry against Bazel, lefthook, and CI text projections.
The checker also rejects orphan projections so guardrails cannot be added beside the registry.

- [x] **Step 3: Run tests**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-task-registry.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-task-registry.ps1
```

Expected: both PASS.

## Task 3: Wire Registry Guardrail

**Files:**
- Modify: `tools/bazel/run_guardrail_task.sh`
- Modify: `tools/bazel/BUILD.bazel`
- Modify: `BUILD.bazel`
- Modify: `lefthook.yml`
- Modify: `.github/workflows/ci.yml`

- [x] **Step 1: Add Bazel targets**

Add:

```python
transition_shell_test(
    name = "guardrail_verification_task_registry",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["verification-task-registry"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)
```

and the corresponding tests target.

- [x] **Step 2: Add lefthook and CI gates**

Add `verification-task-registry` to pre-commit and pre-push, and add direct GitHub Actions steps for the checker and tests.

- [x] **Step 3: Verify full guardrails**

Run:

```bash
bash scripts/ci/run-bazel.sh test //:guardrails_all --config=ci --verbose_failures
```

Expected: all guardrail targets pass.

Verified on 2026-06-18:

- `check-verification-task-registry.tests.ps1` passes.
- `check-verification-task-registry.ps1` passes with `tasks=32`.
- `bash scripts/ci/run-bazel.sh test //tools/bazel:guardrail_verification_task_registry //tools/bazel:guardrail_verification_task_registry_tests //:guardrails_all --config=ci --verbose_failures`
  passed with 32/32 guardrail targets.

Follow-up hardening verified on 2026-06-18:

- Added tests for unregistered Bazel guardrail targets, root guardrail suite labels, and
  `run_guardrail_task.sh` cases.
- The checker now rejects those orphan projections before validating per-task wiring.
- The same Bazel command passed again with 32/32 guardrail targets.
