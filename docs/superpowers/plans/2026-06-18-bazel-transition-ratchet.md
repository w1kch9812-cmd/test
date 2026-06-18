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

- [x] **Task 5: Retire Rustfmt Transition**
  - Replace `//tools/bazel:ci_rustfmt_transition` with `//tools/bazel:rustfmt_check`.
  - Move `//tools/bazel:ci_rustfmt_transition` to `retired_transition_targets`.
  - Re-run targeted ratchet tests, rustfmt Bazel target, guardrails, and full Bazel graph.

- [x] **Task 6: Retire Rust Check Transition**
  - Replace `//tools/bazel:ci_rust_check_transition` with `//:rust_check_verification`.
  - Move `//tools/bazel:ci_rust_check_transition` to `retired_transition_targets`.
  - Use `//tools/bazel:rust_targets.bzl` as the Rust verification target set for format and check suites.
  - Re-run targeted ratchet tests, Rust check Bazel target, guardrails, and full Bazel graph.

- [x] **Task 7: Retire Rust Clippy Transition**
  - Replace `//tools/bazel:ci_rust_clippy_transition` with `//:rust_lint_verification`.
  - Move `//tools/bazel:ci_rust_clippy_transition` to `retired_transition_targets`.
  - Use `rules_rust` `rust_clippy` over `RUST_CRATE_TARGETS`, with `clippy.toml` exported as Bazel config.
  - Re-run targeted ratchet tests, Rust lint Bazel build, guardrails, and full Bazel graph.

- [x] **Task 8: Retire SQLx Prepare Transition**
  - Replace `//tools/bazel:ci_sqlx_prepare_check_transition` with `//:sqlx_prepare_verification`.
  - Move `//tools/bazel:ci_sqlx_prepare_check_transition` to `retired_transition_targets`.
  - Verify committed `.sqlx/query-*.json` offline metadata as Bazel runfiles without DB or cargo shell execution.
  - Re-run targeted ratchet tests, SQLx metadata Bazel target, guardrails, and full Bazel graph.

- [x] **Task 9: Retire Frontend Typecheck Transition**
  - Replace `//tools/bazel:frontend_typecheck_transition` with the existing `//:frontend_typecheck` and `//:workspace_typecheck` Bazel suites.
  - Move `//tools/bazel:frontend_typecheck_transition` to `retired_transition_targets`.
  - Route local `pnpm typecheck` and lefthook pre-push `typecheck` through Bazel instead of the pnpm transition wrapper.
  - Re-run targeted ratchet tests, workspace typecheck, guardrails, and full Bazel graph.

- [x] **Task 10: Retire Frontend Unit Test Transition**
  - Replace `//tools/bazel:frontend_unit_test_transition` with the existing `//:frontend_unit_test` and `//apps/web:vitest_unit_test` Bazel suites.
  - Move `//tools/bazel:frontend_unit_test_transition` to `retired_transition_targets`.
  - Route root and app-level `pnpm test` through Bazel instead of the pnpm transition wrapper.
  - Re-run targeted ratchet tests, frontend unit tests, guardrails, and full Bazel graph.
