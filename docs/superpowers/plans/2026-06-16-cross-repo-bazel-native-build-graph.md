# Cross-Repo Bazel-Native Build Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> or superpowers:executing-plans to implement this plan task-by-task. Keep each repo's
> existing worktree ownership intact.

**Goal:** Make Bazel the final canonical build, test, lint, guardrail, contract-generation,
and release verification graph across `gongzzang`, `platform-core`, and `dawneer`.

**Architecture:** Each repo owns its own Bazel module and package targets. Cross-service
contracts are verified through Bazel targets, not direct database coupling or ad hoc scripts.
Transition wrappers are allowed only while a hermetic target is being introduced and must have
documented exit conditions.

**Tech Stack:** Bazelisk, Bazel 9.1.1 baseline, Bzlmod, rules_rust, crate_universe,
aspect_rules_js, aspect_rules_ts, managed remote cache/execution later.

## Task 1: Decision SSOT

**Files:**
- Create: `docs/adr/0042-cross-repo-bazel-native-build-graph.md`
- Modify: `docs/adr/README.md`

- [x] **Step 1: Record the final-state decision**

State that all three repositories converge to Bazel-native verification. Wrapper-only Bazel is
explicitly rejected as the final architecture.

- [x] **Step 2: Preserve the three-repo boundary**

State that the decision does not merge Git repositories and does not move product ownership
between Gongzzang, Platform Core, and Dawneer.

## Task 2: Platform Core Bazel Bootstrap

**Files:**
- Create: `../platform-core/.bazelversion`
- Create: `../platform-core/.bazelignore`
- Create: `../platform-core/.bazelrc`
- Create: `../platform-core/MODULE.bazel`
- Create: `../platform-core/BUILD.bazel`
- Modify: `../platform-core/.gitignore`
- Create: first package-local `../platform-core/**/BUILD.bazel` files

- [x] **Step 1: Add root Bazel control plane**

Pin the Bazelisk version path and configure Bzlmod, local disk cache, repository cache,
and CI profile settings without committing remote-cache credentials.

- [x] **Step 2: Add the first Rust package targets**

Represent the `platform-core` Cargo workspace with `rules_rust` and `crate_universe`. Start
with a minimal compile/test slice, then expand crate by crate.

- [x] **Step 3: Verify the graph**

Run:

```bash
bazelisk query //...
bazelisk test //...
```

Expected: the initial Platform Core Bazel graph resolves and the represented test slice passes
on WSL2/Linux or Linux CI.

Observed on 2026-06-16:

- Windows direct `bazelisk query //...` reproduced the known `crate_universe` symlink privilege
  failure: `os error 1314`.
- WSL2/Linux `~/.local/bin/bazelisk query //...` passed and listed five targets.
- WSL2/Linux `~/.local/bin/bazelisk test //:rust_fast --verbose_failures` passed with one
  `shared-kernel` test target.

## Task 3: Platform Core Full Rust Graph

**Files:**
- Modify: `../platform-core/BUILD.bazel`
- Create or modify: `../platform-core/crates/**/BUILD.bazel`
- Create or modify: `../platform-core/services/**/BUILD.bazel`

- [x] **Step 1: Add package-local targets for every Cargo workspace member**

Each workspace crate gets explicit library and test targets. Each service gets an explicit
binary target and unit-test target where the local crate structure supports it.

Observed on 2026-06-16:

- Added Bazel targets for every current Platform Core Cargo workspace member:
  `shared-kernel`, `api-types`, Catalog crates, Workforce crates, `outbox-publisher`,
  `services/api`, and `services/outbox-publisher`.
- Added `//:rust_fast` as the Platform Core Rust suite.
- Added `pipeline_graph_runtime_artifacts` as declared Bazel test data for service API tests.
- Made the service API pipeline graph artifact loader Bazel test runfiles-aware while preserving
  the existing Cargo repository-root path.

- [x] **Step 1 verification**

Run:

```bash
~/.local/bin/bazelisk query //...
~/.local/bin/bazelisk test //:rust_fast --verbose_failures
```

Observed: query listed 26 targets and `//:rust_fast` passed with 11 passing test targets.

- [x] **Step 2: Move guardrails into Bazel entrypoints**

Represent existing cutover, supply-chain, and readiness checks as Bazel targets. PowerShell
may remain as the script body during migration, but Bazel owns the invocation graph.

Observed on 2026-06-16:

- Added Platform Core Bazel PowerShell guardrail runner compatibility under `tools/bazel`.
- Added portable guardrail runner self-test suite:
  `//:guardrails_fast` -> lakehouse/R2 runner self-tests.
- Added explicit transition suites for cargo/environment-dependent runner checks:
  `//:guardrails_transition_checks` and `//tools/bazel:guardrail_cargo_runner_tests`.
- Updated the Platform Core file line-limit guardrail to ignore Bazel generated output
  symlink directories.
- Kept the actual legacy CI runner invocations in place until all underlying PowerShell tests are
  Linux-portable under Bazel. WSL evidence showed actual lakehouse/R2 runner checks still fail in
  existing subtests, while their runner self-tests pass.
- Verified:
  - `~/.local/bin/bazelisk test //:guardrails_fast --config=ci --verbose_failures`
  - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\ci\run-sss-guardrails.tests.ps1`
  - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-file-line-limits.tests.ps1`
  - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-file-line-limits.ps1`

- [x] **Step 3: Make CI call Bazel**

Add CI jobs that call Platform Core Bazel targets directly and upload BEP/profile evidence.

Observed on 2026-06-16:

- Added `bazel-fast-graph` to Platform Core CI.
- CI installs Bazelisk from the pinned `BAZELISK_VERSION` environment value.
- CI runs:
  - `bazelisk test //:rust_fast --config=ci --verbose_failures`
  - `bazelisk test //:guardrails_fast --config=ci --verbose_failures`
- CI uploads Bazel BEP/profile evidence from `target/bazel/`.
- Existing direct Cargo/clippy/full guardrail jobs remain as transition coverage until their
  Bazel-native equivalents are complete.

## Task 4: Gongzzang Remaining Wrapper Exit

**Files:**
- Modify: `BUILD.bazel`
- Modify: `tools/bazel/BUILD.bazel`
- Modify: package/app BUILD files as needed

- [ ] **Step 1: Replace Biome wrapper with a declared Bazel target**

Biome config, source inputs, and npm toolchain inputs must be declared.

- [ ] **Step 2: Replace Vitest wrapper with Bazel-compatible test lanes**

Split pure unit tests from Redis/browser/service-backed tests. Service-backed lanes need an
explicit integration-test harness.

- [ ] **Step 3: Replace bundle and Playwright wrappers**

Browser binaries, app startup, auth dependencies, and reports become declared test inputs and
outputs.

## Task 5: Dawneer Protected Bootstrap

**Files:**
- Future: `../dawneer/.bazelversion`
- Future: `../dawneer/.bazelignore`
- Future: `../dawneer/.bazelrc`
- Future: `../dawneer/MODULE.bazel`
- Future: `../dawneer/BUILD.bazel`
- Future: package-local BUILD files

- [ ] **Step 1: Protect existing worktree**

Do not edit Dawneer Bazel files until the current unrelated dirty worktree is committed,
stashed, or isolated in a dedicated worktree.

- [ ] **Step 2: Bootstrap Rust first**

Represent Dawneer Rust services/crates under Bazel before moving pnpm/Turbo app targets.

- [ ] **Step 3: Bootstrap TypeScript packages and apps**

Use the Gongzzang rules_js/rules_ts pattern, adjusted only where Dawneer package boundaries
require it.

## Task 6: Cross-Repo Contracts And Remote Cache

**Files:**
- Future: repo-local contract BUILD targets
- Future: CI workflow files
- Future: `.bazelrc.remote.example` per repo

- [ ] **Step 1: Add contract verification targets**

Platform Core API/OpenAPI/event artifacts and Gongzzang/Dawneer consumer pins are generated
or verified by Bazel targets.

- [ ] **Step 2: Add managed remote cache policy**

Adopt managed remote cache only after credential ownership, cache-write policy, artifact
retention, and BEP observability are defined.

- [ ] **Step 3: Define final completion gate**

The migration is complete only when all three repositories can run their canonical Bazel
verification entrypoints in CI and no wrapper target remains without an exit condition.
