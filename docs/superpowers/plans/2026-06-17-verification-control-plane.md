# Verification Control Plane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a policy-as-data verification control plane that prevents CI and Git hooks from reintroducing direct verification commands outside Bazel-owned targets.

**Architecture:** A checked-in JSON policy defines allowed and forbidden verification command patterns. A PowerShell guardrail reads workflow/hook files, rejects forbidden direct commands, and permits structured transition allowlist entries. Bazel owns the guardrail invocation through `//tools/bazel:guardrail_verification_control_plane`.

**Tech Stack:** Bazel, PowerShell guardrails, JSON policy, lefthook, GitHub Actions.

**Status:** Implemented on 2026-06-17. Targeted guardrail tests, real-repo guardrail execution,
and the default Bazel `//...` graph passed. Follow-up hardening on 2026-06-17 removed the
pre-existing `//:guardrails_all` blockers by making forbidden-marker scanning independent of
`rg`, isolating the traffic/auth API-control-plane fixture from the live repo, and making the
Bazel WSL PowerShell runner prefer Windows PowerShell for `/mnt/<drive>` workspaces. The same
follow-up removed pre-push direct `cargo check`, `cargo clippy`, and `cargo sqlx prepare`
invocations in favor of a Bazel-owned workspace graph entrypoint. A later 2026-06-17 batch
moved CI fast verification commands for Node audit, Biome, markdown links, Rust fmt, Rust
clippy, Rust check, and cargo-deny behind Bazel transition targets, and tightened the
verification-control-plane guardrail so unused allowlist entries fail closed.
A later continuation moved release candidate packaging, coverage, SQLx drift checks, migration
smoke checks, and the walking-skeleton E2E body behind Bazel transition targets; CI workflows now
retain bootstrap/orchestration steps while Bazel owns those verification entrypoints.
A subsequent release hardening step promoted web/API release candidates from transition
side-effects to declared Bazel outputs under `bazel-bin`, added `//:verify_release`, and made
the supply-chain policy/admission workflow consume those Bazel-owned subjects.

---

## File Structure

- Create: `docs/architecture/verification-control-plane.v1.json`
  - Policy SSOT for forbidden direct verification commands and explicit allowlist entries.
- Create: `scripts/ci/check-verification-control-plane.ps1`
  - Guardrail implementation.
- Create: `scripts/ci/check-verification-control-plane.tests.ps1`
  - TDD fixtures for accepted and rejected workflow/hook examples.
- Modify: `tools/bazel/run_guardrail_task.sh`
  - Add `verification-control-plane` and `verification-control-plane-tests` dispatch cases.
- Modify: `tools/bazel/BUILD.bazel`
  - Add Bazel guardrail targets.
- Modify: `BUILD.bazel`
  - Add the new guardrail to the policy guardrail suite.
- Modify: `lefthook.yml`
  - Add the guardrail to pre-commit/pre-push after tests pass.
- Modify: `.github/workflows/ci.yml`
  - Add guardrail invocation only if it is not already covered through Bazel suites.
- Modify: `docs/superpowers/plans/2026-06-16-cross-repo-bazel-native-build-graph.md`
  - Mark the control-plane guardrail step complete after implementation.

## Task 1: Policy SSOT

**Files:**
- Create: `docs/architecture/verification-control-plane.v1.json`

- [ ] **Step 1: Write the policy file**

Create a JSON file with these top-level keys:

```json
{
  "schema_version": 1,
  "forbidden_direct_verification_commands": [],
  "allowed_direct_commands": []
}
```

- [ ] **Step 2: Add forbidden command patterns**

Add patterns for direct verification commands:

```json
[
  "pnpm lint",
  "pnpm test",
  "pnpm build",
  "pnpm typecheck",
  "pnpm biome check",
  "pnpm markdownlint-cli2",
  "cargo check",
  "cargo clippy",
  "cargo test",
  "cargo build"
]
```

- [ ] **Step 3: Add allowlist entries**

Each allowlist entry must include:

```json
{
  "pattern": "pnpm install --frozen-lockfile",
  "scope": ".github/workflows/*.yml",
  "owner": "build-platform",
  "reason": "Dependency bootstrap, not verification semantics.",
  "exit_target": "Keep until Bazel toolchain bootstrap covers package manager fetch.",
  "sunset": "2026-07-31"
}
```

Expected initial allowlist categories:

- package install/bootstrap
- Playwright browser install
- cargo-deny
- cargo-tarpaulin
- sqlx prepare/migration checks
- supply-chain release packaging
- Pulumi local preview

## Task 2: Guardrail Tests First

**Files:**
- Create: `scripts/ci/check-verification-control-plane.tests.ps1`

- [ ] **Step 1: Add fixture helper functions**

Create helpers:

```powershell
function New-TestRoot { ... }
function Write-File { param($Root, $RelativePath, $Content) ... }
function Invoke-Guardrail { param($Root) ... }
function Assert-Success { param($Result, $ExpectedText) ... }
function Assert-Failure { param($Result, $ExpectedText) ... }
```

- [ ] **Step 2: Add accepted Bazel workflow test**

Fixture:

```yaml
jobs:
  verify:
    steps:
      - run: bazelisk test //:verify_pr --config=ci --verbose_failures
```

Expected: success with `verification-control-plane-ok`.

- [ ] **Step 3: Add rejected direct pnpm test**

Fixture:

```yaml
jobs:
  verify:
    steps:
      - run: pnpm test
```

Expected: failure containing `forbidden direct verification command`.

- [ ] **Step 4: Add rejected direct cargo clippy test**

Fixture:

```yaml
jobs:
  verify:
    steps:
      - run: cargo clippy --workspace --all-features
```

Expected: failure containing `cargo clippy`.

- [ ] **Step 5: Add allowlisted bootstrap test**

Fixture:

```yaml
jobs:
  verify:
    steps:
      - run: pnpm install --frozen-lockfile
```

Expected: success only when the policy allowlist contains the entry.

- [ ] **Step 6: Run RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-control-plane.tests.ps1
```

Expected: FAIL because the guardrail script does not exist yet.

## Task 3: Guardrail Implementation

**Files:**
- Create: `scripts/ci/check-verification-control-plane.ps1`

- [ ] **Step 1: Load policy JSON**

Read `docs/architecture/verification-control-plane.v1.json`, validate required keys, and fail if malformed.

- [ ] **Step 2: Find inspected files**

Inspect:

- `.github/workflows/*.yml`
- `.github/workflows/*.yaml`
- `lefthook.yml`

- [ ] **Step 3: Scan command lines**

For each file, scan lines containing `run:` or lefthook `run:` command values. Normalize whitespace but preserve the original line for reporting.

- [ ] **Step 4: Apply allowlist first**

If a command line contains an allowlisted pattern for the file scope, skip it.

- [ ] **Step 5: Reject forbidden direct verification commands**

If a command line contains a forbidden pattern, print:

```text
verification-control-plane: forbidden direct verification command in <file>:<line>: <command>
```

Return non-zero.

- [ ] **Step 6: Print success**

On success:

```text
verification-control-plane-ok files=<N> allowlisted=<N>
```

- [ ] **Step 7: Run GREEN**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-control-plane.tests.ps1
```

Expected: PASS.

## Task 4: Bazel Wiring

**Files:**
- Modify: `tools/bazel/run_guardrail_task.sh`
- Modify: `tools/bazel/BUILD.bazel`
- Modify: `BUILD.bazel`

- [ ] **Step 1: Add runner cases**

Add cases:

```bash
verification-control-plane)
  run_pwsh scripts/ci/check-verification-control-plane.ps1 -Root "$repo_root"
  ;;
verification-control-plane-tests)
  run_pwsh scripts/ci/check-verification-control-plane.tests.ps1
  ;;
```

- [ ] **Step 2: Add Bazel targets**

Add:

```bzl
transition_shell_test(
    name = "guardrail_verification_control_plane",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["verification-control-plane"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)

transition_shell_test(
    name = "guardrail_verification_control_plane_tests",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["verification-control-plane-tests"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)
```

- [ ] **Step 3: Add to root guardrail suites**

Add implementation guardrail to `//:guardrails_policy` and tests to `//:guardrails_policy_tests`.

- [ ] **Step 4: Run Bazel guardrail**

Run:

```bash
wsl -d Ubuntu --cd /mnt/c/Users/admin/Desktop/gongzzang ~/.local/bin/bazelisk test //tools/bazel:guardrail_verification_control_plane_tests --config=ci --verbose_failures
```

Expected: PASS.

## Task 5: Hook and CI Integration

**Files:**
- Modify: `lefthook.yml`
- Modify: `.github/workflows/ci.yml` if needed

- [ ] **Step 1: Add pre-commit hook**

Add `verification-control-plane` near other policy guardrails.

- [ ] **Step 2: Add pre-push hook**

Add the same guardrail before long-running cargo/typecheck checks.

- [ ] **Step 3: Ensure CI runs the guardrail**

Prefer coverage through `bazelisk test //...`. If direct explicit CI coverage is needed, call the Bazel target, not the PowerShell script directly.

- [ ] **Step 4: Verify hooks do not reject current allowlisted commands**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-control-plane.ps1
```

Expected: PASS with current allowlist.

## Task 6: Final Verification and Commit

**Files:**
- All files above

- [ ] **Step 1: Run targeted tests**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-verification-control-plane.tests.ps1
bash scripts/ci/check-workspace-typecheck-coverage.tests.sh
pnpm typecheck
```

Expected: all PASS.

- [ ] **Step 2: Run Bazel full graph**

Run:

```powershell
wsl -d Ubuntu --cd /mnt/c/Users/admin/Desktop/gongzzang ~/.local/bin/bazelisk test //... --config=ci --verbose_failures
```

Expected: all tests PASS.

- [ ] **Step 3: Run diff hygiene**

Run:

```powershell
git diff --check
git status --short --branch
```

Expected: no whitespace errors; only intended files changed.

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/architecture/verification-control-plane.v1.json scripts/ci/check-verification-control-plane.ps1 scripts/ci/check-verification-control-plane.tests.ps1 tools/bazel/run_guardrail_task.sh tools/bazel/BUILD.bazel BUILD.bazel lefthook.yml .github/workflows/ci.yml docs/superpowers/plans/2026-06-16-cross-repo-bazel-native-build-graph.md
git commit -m "build: add verification control plane guardrail"
```

- [ ] **Step 5: Push**

Run:

```powershell
git push --verbose
```

Expected: pre-push hooks pass and `HEAD == origin/main`.
