# Bazel Commit Boundary Handoff

> ⛔ **[ADR-0044](../../adr/0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 cargo+pnpm/Turbo가 영구 빌드 SSOT다. 이 문서는 (취소된) 결정의 역사적 기록일 뿐 — 구현하지 말 것.

Date: 2026-06-07
Scope: Gongzzang Bazel-first build and verification control plane

## Purpose

This handoff records the clean commit boundary for the Bazel work. The repository currently contains
other in-progress changes, so the Bazel commit must be staged deliberately instead of using a broad
`git add .`.

## Include In The Bazel Commit

Core Bazel control plane:

- `.bazelignore`
- `.bazelrc`
- `.bazelrc.remote.example`
- `.bazelversion`
- `BUILD.bazel`
- `MODULE.bazel`
- `REPO.bazel`
- `tools/bazel/**`

CI and build metadata:

- `.github/workflows/ci.yml`
- `.github/workflows/frontend.yml`
- `.gitignore`
- `.nvmrc`
- `package.json`
- `pnpm-lock.yaml`

Frontend Bazel package graph:

- `apps/web/BUILD.bazel`
- `apps/web/bazel/next-cli.mjs`
- `apps/web/tsconfig.bazel.json`
- `packages/api-types/BUILD.bazel`
- `packages/api-types/tsconfig.bazel.json`
- `packages/ui/BUILD.bazel`
- `packages/ui/tsconfig.bazel.json`

Next production build compatibility required by the Bazel target:

- `apps/web/app/global-error.tsx`
- `apps/web/app/(authenticated)/layout.tsx`
- `apps/web/next.config.ts`

TypeScript strict compatibility exposed by the Bazel target:

- `apps/web/components/listings/listing-card.tsx`

Rust/service Bazel package declarations:

- `crates/**/BUILD.bazel`
- `services/**/BUILD.bazel`

Bazel decision records:

- `docs/adr/0040-bazel-first-build-verification-control-plane.md`
- `docs/adr/0041-hermetic-javascript-package-bazel-rules.md`
- `docs/superpowers/plans/2026-06-07-bazel-enterprise-hardening.md`
- `docs/superpowers/plans/2026-06-07-bazel-first-build-verification-control-plane.md`
- `docs/superpowers/specs/2026-06-07-bazel-first-build-verification-control-plane-design.md`
- `docs/superpowers/handoff/2026-06-07-bazel-commit-boundary.md`

ADR index update:

- `docs/adr/README.md`

Stage only the ADR 0040 and ADR 0041 lines for the Bazel commit. The ADR 0039 index line belongs
with the lakehouse commit because ADR 0039 itself is intentionally outside this commit boundary.

## Keep Out Of The Bazel Commit

Lakehouse/R2 work:

- `.env.example`
- `docs/adr/0039-service-owned-lakehouse-registry-integration.md`
- `docs/superpowers/specs/2026-06-05-gongzzang-service-owned-lakehouse-integration-design.md`

Marker visual runtime work:

- `apps/web/components/listings/listing-map.tsx`
- `apps/web/lib/map/marker-tile-style.ts`
- `apps/web/tests/unit/map/marker-tile-style.test.ts`
- `docs/superpowers/README.md`
- `docs/superpowers/specs/2026-06-01-gongzzang-marker-visual-runtime-design.md`

Review artifact:

- `docs/review/2026-06-06-adversarial-review.md`

## Notes

- Historical mentions of `next_compile` and `use_execroot_entry_point = False` are intentional in
  ADR/spec text because they explain the removed failure mode.
- Live Bazel labels now use `//apps/web:next_production_build`,
  `//:frontend_hermetic_typechecks`, and `//:frontend_hermetic_full_builds`.
- Transition wrapper targets remain explicit and `manual` because Biome, Vitest, bundle budget, and
  Playwright still depend on repo-local service/runtime setup outside the hermetic Bazel JS targets.

## Required Verification Before Claiming Ready

Run these from the repository root:

```bash
rg -n "frontend_hermetic_compiles|next_full_build_probe|web_next_compile_bazel|web_next_full_build_bazel|GONGZZANG_BAZEL_NEXT_COMPILE_ONLY" .github BUILD.bazel apps/web tools -S
bazelisk query 'set(//:frontend_hermetic_typechecks //:frontend_hermetic_full_builds //:guardrails_all)'
bazelisk build //:frontend_hermetic_typechecks //:frontend_hermetic_full_builds --config=ci --verbose_failures
git diff --check
```
