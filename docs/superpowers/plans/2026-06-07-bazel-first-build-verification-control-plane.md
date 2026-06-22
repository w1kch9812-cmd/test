# Bazel-first Build Verification Control Plane Implementation Plan

> ⛔ **[ADR-0044](../../adr/0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 cargo+pnpm/Turbo가 영구 빌드 SSOT다. 이 문서는 (취소된) 결정의 역사적 기록일 뿐 — 구현하지 말 것.
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Bazel the canonical build and verification control-plane direction for Gongzzang, then expand the first wave to the full Rust Cargo workspace.

**Architecture:** Bazelisk pins the Bazel version, Bzlmod owns external build rules, rules_rust builds Rust targets, and crate_universe reads Cargo workspace metadata during the migration. Cargo/pnpm/Turbo remain transitional execution surfaces until their targets are fully represented in Bazel.

**Tech Stack:** Bazel 9.1.1, Bazelisk 1.29.0, Bzlmod, rules_rust 0.70.0, Cargo workspace, local Bazel disk cache.

---

## Task 1: Decision Record

**Files:**
- Create: `docs/adr/0040-bazel-first-build-verification-control-plane.md`
- Modify: `docs/adr/README.md`

- [x] **Step 1: Document why Bazel is chosen**

Capture the decision that Gongzzang adopts Bazel-first build and verification, with Buck2 and custom planners rejected for now.

- [x] **Step 2: Record enterprise references**

Reference Bazel users, remote caching, Bzlmod, and rules_rust official docs.

- [x] **Step 3: Add ADR index row**

Add ADR-0040 to `docs/adr/README.md`.

## Task 2: Root Bazel Bootstrap

**Files:**
- Create: `.bazelversion`
- Create: `.bazelignore`
- Create: `.bazelrc`
- Create: `MODULE.bazel`
- Create: `BUILD.bazel`
- Modify: `.gitignore`

- [x] **Step 1: Pin Bazel**

Set `.bazelversion` to `9.1.1`.

- [x] **Step 2: Configure Bzlmod and rules_rust**

Create `MODULE.bazel` with `rules_rust` and `crate_universe` reading the root Cargo workspace.

- [x] **Step 3: Add local cache baseline**

Add `.bazelrc` with local disk cache, repository cache, test output, and CI profile defaults.

- [x] **Step 4: Ignore generated outputs**

Add `.bazelignore` and `.gitignore` entries for Bazel generated paths.

- [x] **Step 5: Keep Bzlmod lockfile out until line-limit strategy exists**

Set `--lockfile_mode=off` because generated `MODULE.bazel.lock` exceeded the repo's
1500-line file limit. Ignore `MODULE.bazel.lock` until an ADR-approved partition or exception
strategy exists.

## Task 3: First Rust Target

**Files:**
- Create: `crates/domain/core/shared-kernel/BUILD.bazel`

- [x] **Step 1: Add rust_library target**

Create `shared_kernel` from `src/**/*.rs`, with dependencies supplied by `crate_universe`.

- [x] **Step 2: Add rust_test target**

Create `shared_kernel_unit_test` as the first Bazel Rust smoke test.

## Task 4: Verification

**Files:**
- Read: Bazel/Cargo/doc outputs

- [x] **Step 1: Verify Bazelisk**

Run: `bazelisk version`

Observed: Windows Bazelisk 1.29.0 and Bazel 9.1.1 printed successfully. WSL2 Bazelisk
1.29.0 was installed in `$HOME/.local/bin` and also printed Bazel 9.1.1.

- [x] **Step 2: Query Bazel graph**

Run: `bazelisk query //...`

Observed: Windows direct query hit a symlink privilege failure in `crate_universe`, matching
the documented Windows support risk. WSL2 query succeeded and listed root filegroups plus
`//crates/domain/core/shared-kernel:shared_kernel` and
`//crates/domain/core/shared-kernel:shared_kernel_unit_test`.

- [x] **Step 3: Test first Rust target**

Run: `bazelisk test //crates/domain/core/shared-kernel:shared_kernel_unit_test`

Observed: `bazelisk test //crates/domain/core/shared-kernel:shared_kernel_unit_test`
passed on WSL2. A second `bazelisk test //...` also passed with a cached test result.

- [x] **Step 4: Run repository-safe checks**

Run: `cargo fmt --check` and `git diff --check`

Observed: `cargo fmt --check` passed through the installed Windows Cargo path, and
`git diff --check` passed.

## Task 5: Rust Workspace First-Wave Expansion

**Files:**
- Create: `tools/bazel/BUILD.bazel`
- Create: `tools/bazel/rust_workspace.bzl`
- Create: crate-specific `BUILD.bazel` files
- Create: service-specific `BUILD.bazel` files

- [x] **Step 1: Add shared Rust Bazel macros**

Added `gongzzang_rust_library_with_unit_test` and `gongzzang_rust_binary_with_unit_test`
as the SSOT for Rust library/binary Bazel targets.

- [x] **Step 2: Convert Cargo workspace crates**

Converted all Cargo workspace crates under `crates/`, including domain, operations, auth,
circuit-breaker, parcel-lookup, outbox-publisher, and db.

- [x] **Step 3: Make SQLx offline macros work hermetically**

`crates/db` needed `.sqlx`, Cargo workspace metadata, Bazel control-plane metadata, and a
small `cargo metadata` shim as declared compile inputs. The issue was not runtime DB access;
it was missing compile-time sandbox inputs for SQLx proc macros.

- [x] **Step 4: Convert Rust service binaries**

Converted `services/api`, `services/outbox-publisher`, and `services/etl-base-layer`.
`services/api` exposes both `api_service` and `platform_core_anchor_import` Bazel binaries.

- [x] **Step 5: Verify full WSL2/Linux Bazel Rust graph**

Run: `bazelisk query //...`

Observed: WSL2 query passed and listed 56 targets.

Run: `bazelisk test //...`

Observed: WSL2 test passed with 26 passing tests, 0 skipped, and 0 failing. This includes
all Rust Cargo workspace crates and Rust service binaries currently represented by Bazel.

## Task 6: Next Migration Slices

**Files:**
- Future: CI/lefthook Bazel entrypoints
- Future: guardrail wrapper targets
- Future: frontend/package Bazel targets

- [ ] **Step 1: Wrap current guardrails as Bazel tests**

Represent existing PowerShell/bash guardrails as Bazel test targets before deleting wrappers
that still serve local developer convenience.

- [ ] **Step 2: Add managed remote cache**

Choose a managed provider after credentials and write policy exist. Do not self-host remote execution first.

- [x] **Step 3: Add TypeScript/package targets**

Move pnpm/Turborepo verification into Bazel only after the Rust graph remains stable.

Observed: first package slice completed with hermetic `rules_js` / `rules_ts` targets for
`packages/api-types` and `packages/ui`. Full `apps/web` Next.js build remains a later slice.

## Task 7: Frontend and Guardrail Transition Wrappers

**Files:**
- Modify: `BUILD.bazel`
- Modify: `.bazelignore`
- Modify: `tools/bazel/BUILD.bazel`
- Create: `tools/bazel/shell_test_compat.bzl`
- Create: `tools/bazel/run_pnpm_task.sh`
- Create: `tools/bazel/run_guardrail_task.sh`

- [x] **Step 1: Add a repository-owned shell test compatibility rule**

Added `transition_shell_test` because Bazel 9 in this repository does not expose native
`sh_test` in BUILD files. The rule wraps exactly one script into a Bazel test executable
without adding external dependencies.

- [x] **Step 2: Add frontend pnpm/Turbo Bazel wrappers**

Added manual local Bazel targets for lint, typecheck, unit test, production build, bundle
budget, and e2e:

- `//:frontend_lint`
- `//:frontend_typecheck`
- `//:frontend_unit_test`
- `//:frontend_build`
- `//:frontend_bundle`
- `//:frontend_e2e`
- `//:frontend_verification`
- `//:frontend_release_verification`

These are transitional local wrappers around the existing pnpm/Turbo commands. They are not
yet hermetic JS rules.

- [x] **Step 3: Add guardrail Bazel wrappers**

Added manual local Bazel targets for existing bash/PowerShell guardrails and grouped them as:

- `//:guardrails_fast`
- `//:guardrails_policy`
- `//:guardrails_policy_tests`
- `//:guardrails_all`

- [x] **Step 4: Keep wildcard Rust verification stable**

All transition wrappers are tagged `manual`, `local`, and `no-sandbox`, so `bazelisk test //...`
continues to run the hermetic Rust graph without pulling in local Node/PowerShell guardrails
implicitly.

- [x] **Step 5: Verify transition targets**

Run: `bazelisk query 'set(//:frontend_typecheck //:guardrails_fast //:guardrails_all //tools/bazel:frontend_typecheck //tools/bazel:guardrail_file_line_limit)'`

Observed: query passed and listed the requested transition targets.

Run: `bazelisk test //tools/bazel:guardrail_file_line_limit`

Observed: passed.

Run: `bazelisk test //tools/bazel:guardrail_workspace_typecheck_coverage`

Observed: passed.

Run: `bazelisk test //...`

Observed: Rust workspace wildcard verification still passed with 26 passing tests, 0 skipped,
and 0 failing.

## Task 8: Remaining Bazel Hardening

**Files:**
- Future: `MODULE.bazel`
- Future: `.github/workflows/*.yml`
- Future: frontend/package `BUILD.bazel` files

- [ ] **Step 1: Adopt hermetic JavaScript Bazel rules**

Evaluate `aspect_rules_js` / `rules_ts` / Next.js build integration before replacing local
pnpm wrappers. This should be a separate ADR because it adds a new Bazel dependency surface.

Observed: ADR-0041 adopted `rules_nodejs`, `aspect_rules_js`, and `aspect_rules_ts` for the
package layer. `//:frontend_hermetic_typechecks` is now a real Bazel build entrypoint.
`apps/web` now participates in a root-owned hermetic `//:web_typecheck` target and has a
hermetic `//apps/web:next_production_build` target. These are exposed through
`//:web_typecheck_bazel`, `//:web_next_production_build_bazel`, `//:frontend_hermetic_typechecks`,
and `//:frontend_hermetic_full_builds`.

The web typecheck target is intentionally rooted at the repository root because it spans
`apps/web`, `packages/ui`, and `packages/api-types`. The app and package BUILD files expose
explicit `copy_to_bin` inputs and declaration targets; the root target owns the cross-package
TypeScript action.

`next_production_build` is intentionally compile-mode only. Full Next.js static/prerender production
build remains a tracked gap because Next 16 webpack currently reaches `/_global-error`
prerendering and fails on a Next workStore invariant inside the Bazel sandbox. Vitest,
Playwright, and bundle-budget runtime checks remain transition wrappers.

Follow-up resolution: the full Next.js production build gap was resolved in
`2026-06-07-bazel-enterprise-hardening.md`. The root cause was the
`use_execroot_entry_point = False` override on Next Bazel targets, which split Next tool and
runtime resolution. The full build is now represented by `//apps/web:next_production_build`
and root aggregate `//:frontend_hermetic_full_builds`.

- [ ] **Step 2: Make CI call Bazel entrypoints**

After hermetic JS strategy is decided, switch CI jobs from direct pnpm/Cargo guardrail calls
to Bazel targets where practical.

- [ ] **Step 3: Add managed remote cache**

Choose a managed provider after credentials and write policy exist. Do not self-host remote execution first.
