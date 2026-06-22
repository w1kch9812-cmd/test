# Bazel Enterprise Hardening Implementation Plan

> ⛔ **[ADR-0044](../../adr/0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 cargo+pnpm/Turbo가 영구 빌드 SSOT다. 이 문서는 (취소된) 결정의 역사적 기록일 뿐 — 구현하지 말 것.
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the Gongzzang Bazel build and verification surface from a working migration slice to an enterprise-grade build platform baseline.

**Architecture:** Bazel remains the canonical graph owner. Hermetic targets are preferred over local wrappers; transition wrappers remain only where browsers, local services, or external credentials still make hermetic execution impractical. CI should call the same Bazel entrypoints used locally, and remote cache configuration should be explicit but disabled unless credentials are present.

**Tech Stack:** Bazelisk, Bazel 9.1.1, Bzlmod, rules_rust, aspect_rules_js, aspect_rules_ts, aspect_bazel_lib, Next.js 16, pnpm transition wrappers, GitHub Actions.

---

## Task 1: Fix The Enterprise Completion Criteria

**Files:**
- Modify: `docs/adr/0040-bazel-first-build-verification-control-plane.md`
- Modify: `docs/adr/0041-hermetic-javascript-package-bazel-rules.md`
- Modify: `docs/superpowers/specs/2026-06-07-bazel-first-build-verification-control-plane-design.md`
- Modify: `docs/superpowers/plans/2026-06-07-bazel-first-build-verification-control-plane.md`

- [x] **Step 1: Define the Bazel enterprise acceptance bar**

Record the acceptance bar:

- `bazelisk test //... --config=ci` is the canonical fast graph.
- `//:frontend_hermetic_typechecks` and `//:frontend_hermetic_full_builds` are first-class targets.
- CI uses Bazel entrypoints for Bazel-owned work.
- Remote cache flags are explicit and opt-in through environment-provided credentials.
- Remaining transition wrappers are documented with owner, reason, and exit condition.

- [x] **Step 2: Verify the bar**

Run:

```bash
bazelisk query //...
```

Expected: query completes without loading local `node_modules` or `.next` as source packages.

Observed: `bazelisk query 'set(//:frontend_hermetic_typechecks //:frontend_hermetic_full_builds //:guardrails_all)'` resolved all labels.

## Task 2: Add CI-Ready Bazel Entry Points

**Files:**
- Modify: `.bazelrc`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/frontend.yml`
- Modify: `BUILD.bazel`

- [x] **Step 1: Add explicit CI Bazel command profile**

Add CI profile settings for remote-cache-ready behavior without hardcoding credentials:

```text
build:ci --remote_download_minimal
build:ci --noshow_progress
build:ci --build_event_json_file=target/bazel/bep/build.json
build:ci --profile=target/bazel/profile/profile.gz
test:ci --test_output=errors
test:ci --test_summary=detailed
```

- [x] **Step 2: Add CI workflow Bazel smoke job**

Add a Bazel job that installs Bazelisk, runs `bazelisk test //... --config=ci`, and uploads BEP/profile artifacts.

- [x] **Step 3: Replace frontend CI typecheck/build path with Bazel where hermetic**

In `frontend.yml`, keep lint, Vitest, Playwright, and bundle budget as transition commands, but replace TypeScript typecheck with:

```bash
bazelisk build //:frontend_hermetic_typechecks --config=ci
```

and add:

```bash
bazelisk build //:frontend_hermetic_full_builds --config=ci
```

Observed: frontend CI runs `bazelisk build //:frontend_hermetic_full_builds --config=ci --verbose_failures`.

- [x] **Step 4: Verify CI YAML references valid Bazel targets**

Run:

```bash
bazelisk query 'set(//:frontend_hermetic_typechecks //:frontend_hermetic_full_builds //:guardrails_all)'
```

Expected: all labels resolve.

Observed: typecheck, full-build, and guardrail labels resolved.

## Task 3: Reproduce The Full Next Production Build Gap

**Files:**
- Read: `apps/web/BUILD.bazel`
- Read: `apps/web/bazel/next-cli.mjs`
- Read: `apps/web/next.config.ts`

- [x] **Step 1: Re-run the current production build target**

Run:

```bash
bazelisk build //apps/web:next_production_build --verbose_failures
```

Expected: pass.

Observed: `bazelisk build //apps/web:next_production_build --verbose_failures` passed after restoring the default Next execroot entrypoint behavior.

- [x] **Step 2: Reproduce full build failure outside completion claims**

Run a full `next build --webpack` path with the same deterministic build environment. Capture whether the blocker is still `/_global-error` `workStore` or has changed.

Observed: `bazelisk build //apps/web:next_production_build --verbose_failures` initially reproduced `/_global-error` `workStore` failure. `pnpm --filter @gongzzang/web exec next build --webpack` passed outside Bazel, proving the issue was Bazel execution-path specific.

- [x] **Step 3: Decide fix path from evidence**

If the failure is a real app static/dynamic boundary issue, fix route metadata or error boundary ownership. If it is a Next/Bazel execution-path incompatibility, fix the Bazel target shape and document the root cause.

Observed: root cause was the `use_execroot_entry_point = False` override, which split Next tool/runtime resolution between exec and target configurations. Removing the override made full Bazel `next build --webpack` pass. Added a self-contained `app/global-error.tsx` root-layout fallback using existing i18n keys.

## Task 4: Document Transition Wrapper Exit Conditions

**Files:**
- Modify: `docs/adr/0040-bazel-first-build-verification-control-plane.md`
- Modify: `docs/adr/0041-hermetic-javascript-package-bazel-rules.md`
- Modify: `docs/superpowers/plans/2026-06-07-bazel-first-build-verification-control-plane.md`

- [x] **Step 1: Inventory transition wrappers**

List wrappers still tagged `manual`, `local`, or `no-sandbox`.

- [x] **Step 2: Assign exit condition per wrapper**

For each wrapper, record whether it exits through hermetic Bazel rule migration, browser/service integration, or permanent local-only classification.

Observed: ADR-0041 records exit conditions for Biome lint, Vitest, bundle budget, and Playwright.

## Task 5: Final Verification

**Files:**
- Read: command outputs

- [x] **Step 1: Run Bazel fast graph**

Run:

```bash
bazelisk test //... --config=ci --verbose_failures
```

Expected: pass.

Observed: `bazelisk test //... --config=ci --verbose_failures --build_event_json_file=target/bazel/bep/local-final-ci-test.json --profile=target/bazel/profile/local-final-ci-test.gz` passed with 31 passing tests, 0 skipped, and 0 failing. BEP/profile outputs were produced.

- [x] **Step 2: Run frontend compatibility check**

Run:

```bash
pnpm --filter @gongzzang/web typecheck
```

Expected: pass.

Observed: `pnpm --filter @gongzzang/web typecheck` passed. `pnpm --filter @gongzzang/web build` also passed with deterministic production-like environment variables.

- [x] **Step 3: Run repository hygiene**

Run:

```bash
git diff --check
```

Expected: pass.

Observed: `git diff --check` passed.
