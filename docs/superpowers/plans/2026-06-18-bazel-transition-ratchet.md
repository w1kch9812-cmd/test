# Bazel Transition Ratchet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deny untracked Bazel transition wrappers and make every remaining transition target explicit, owned, sunsetted, and tied to a final Bazel exit target.

**Architecture:** A JSON policy is the SSOT for allowed transition targets. A PowerShell guardrail scans Bazel BUILD files for `_transition` targets and scans CI/hook files for `_transition` references, then fails on missing, stale, expired, or unsafe transition policy entries. Bazel, lefthook, and CI run the guardrail.

**Tech Stack:** Bazel, PowerShell guardrails, JSON policy, GitHub Actions, lefthook.

---

## File Structure

- Create: `docs/architecture/verification-transition-ratchet.v1.json`
  - Transition target SSOT with owner, reason, sunset, exit target, and external-collection approval flag.
- Create: `scripts/ci/check-bazel-transition-ratchet.ps1`
  - Guardrail implementation.
- Create: `scripts/ci/check-bazel-transition-ratchet.tests.ps1`
  - TDD fixture tests for success, missing policy, stale policy, expired sunset, external advisory flag, and CI references.
- Modify: `tools/bazel/run_guardrail_task.sh`
  - Add dispatch cases for the new guardrail and tests.
- Modify: `tools/bazel/BUILD.bazel`
  - Add Bazel guardrail targets.
- Modify: `BUILD.bazel`
  - Add guardrail targets to policy suites.
- Modify: `.github/workflows/ci.yml`
  - Run the ratchet guardrail beside verification-control-plane.
- Modify: `lefthook.yml`
  - Enforce the ratchet locally before commit and push.

## Tasks

- [x] **Task 1: Write failing ratchet tests**
  - Add a fixture test that expects a missing checker to fail before implementation.
  - Verify RED with `check-bazel-transition-ratchet.tests.ps1`.

- [x] **Task 2: Add policy SSOT and checker**
  - Add `verification-transition-ratchet.v1.json`.
  - Add checker logic for BUILD scanning, CI reference scanning, stale policy, sunset, and external advisory approval flags.

- [x] **Task 3: Wire enforcement**
  - Add Bazel guardrail targets.
  - Add CI and lefthook wiring.

- [x] **Task 4: Verify, commit, push**
  - Run targeted tests, guardrails, full Bazel graph, hooks, commit, push, and confirm clean status.
