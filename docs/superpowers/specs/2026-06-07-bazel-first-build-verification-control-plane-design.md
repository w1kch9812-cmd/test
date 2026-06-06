# Bazel-first Build Verification Control Plane Design

## 목적

Gongzzang의 긴 로컬/CI 검증 시간을 구조적으로 줄이기 위해 Bazel을 repo 표준 build and
verification control plane으로 채택한다. 목표는 "스크립트를 더 빠르게 돌리기"가 아니라,
대기업식 action graph, 명시적 input/output, local/remote cache, affected verification을
가능하게 하는 것이다.

## 대기업식 기준

공개 사례 기준으로 Bazel은 Google, Canva, Databricks, Dropbox, Redfin, Stripe, Uber 등에서
대형 polyglot monorepo 또는 build/test pipeline에 사용된다. 우리가 가져올 핵심 패턴은 다음이다.

- 하나의 canonical entrypoint: 개발자와 CI가 같은 target graph를 호출한다.
- 명시적 dependency graph: 어떤 파일이 어떤 검증에 영향을 주는지 build system이 안다.
- cache-first execution: 동일 input의 action output은 재사용한다.
- remote cache/execution readiness: 처음부터 관리형 remote cache를 붙일 수 있게 설계한다.
- incremental migration: Cargo/pnpm/Turbo를 한 번에 폐기하지 않고 Bazel target으로 점진 이관한다.

## 아키텍처

Root에는 `.bazelversion`, `.bazelrc`, `.bazelignore`, `MODULE.bazel`, `BUILD.bazel`을 둔다.
Bazelisk가 `.bazelversion`을 읽어 Bazel 버전을 고정하고, `MODULE.bazel`은 Bzlmod 기준으로
rules_rust와 Rust toolchain을 선언한다. `crate_universe`는 `Cargo.lock`과 workspace manifest를
읽어 Rust third-party dependency graph를 생성한다.

Bzlmod lockfile은 현재 생성 결과가 1500줄을 넘는다. Gongzzang의 file-line-limit rule이 더
강하므로 이번 단계에서는 `.bazelrc`의 `--lockfile_mode=off`로 lockfile 생성을 끄고
`MODULE.bazel.lock`은 커밋하지 않는다. lockfile을 다시 켜려면 별도 ADR로 1500줄 규칙과의
충돌을 먼저 해결한다.

첫 target은 `crates/domain/core/shared-kernel`이다. 이 crate는 도메인 공통 value object를 담고
다른 crate보다 dependency surface가 작아서 Bazel Rust migration의 smoke target으로 적합하다.

## 경계

이번 단계에서 하지 않는다.

- 전체 Rust workspace 완전 Bazel 전환
- pnpm/Turborepo 완전 제거
- GitHub Actions 전체 재작성
- remote cache/execution 계정 연결
- Platform Core와 Dawneer repo까지 동시 변경

이번 단계에서 한다.

- Bazel-first ADR 작성
- Bazelisk/Bzlmod/rules_rust 기반 root bootstrap
- local disk cache baseline
- shared-kernel Bazel rust_library/rust_test target
- 다음 단계 구현 계획 문서화

## 실행 환경

Windows에서는 Bazelisk를 설치해 기본 query/build smoke를 시도할 수 있다. 다만 rules_rust 공식
문서가 Linux/macOS를 주 지원 대상으로 설명하므로, Rust Bazel release gate는 WSL2/Linux 또는
Linux CI runner에서 canonical로 실행한다. Windows는 개발 편의 환경이며 최종 판단 환경이 아니다.

## 검증

최소 검증은 다음 순서다.

1. `bazelisk version`
2. `bazelisk query //...`
3. `bazelisk test //crates/domain/core/shared-kernel:shared_kernel_unit_test`
4. `cargo fmt --check`
5. `git diff --check`

## First-wave implementation result

The 2026-06-07 implementation expanded the initial shared-kernel smoke target to the full
Rust Cargo workspace.

Current Bazel Rust coverage:

- All `crates/*` Cargo workspace packages.
- `services/api` as `api_service` and `platform_core_anchor_import`.
- `services/outbox-publisher` as `outbox_publisher_service`.
- `services/etl-base-layer` as `etl_base_layer_service`.

The Rust Bazel target shape is centralized in `tools/bazel/rust_workspace.bzl`. SQLx offline
proc macros in `crates/db` use declared compile inputs instead of relying on ambient Cargo
workspace state.

Verified on WSL2/Linux:

1. `bazelisk query //...` passed and listed 56 targets.
2. `bazelisk test //...` passed with 26 passing tests, 0 skipped, and 0 failing.

This does not yet replace frontend/package verification, CI/lefthook guardrails, managed
remote cache, or the Bzlmod lockfile strategy.

## Transition wrapper result

The follow-up transition added manual local Bazel entrypoints for existing frontend and
guardrail commands:

- Frontend: `//:frontend_lint`, `//:frontend_typecheck`, `//:frontend_unit_test`,
  `//:frontend_build`, `//:frontend_bundle`, `//:frontend_e2e`.
- Frontend suites: `//:frontend_verification`, `//:frontend_release_verification`.
- Guardrail suites: `//:guardrails_fast`, `//:guardrails_policy`,
  `//:guardrails_policy_tests`, `//:guardrails_all`.

These wrappers centralize command entrypoints under Bazel but remain transitional. They run
the existing pnpm/Turbo/bash/PowerShell tools locally and are tagged `manual`, `local`, and
`no-sandbox` so wildcard Rust verification remains stable. Full hermetic JavaScript Bazel
support requires a later `aspect_rules_js` / `rules_ts` / Next.js integration decision.

## Hermetic JavaScript package result

ADR-0041 adds the first real JavaScript hermetic slice. Bazel now owns Node 20.19.0,
pnpm lockfile translation, TypeScript compiler resolution, and package-level typecheck
actions for:

- `//packages/api-types:api_types_typecheck`
- `//packages/ui:ui_typecheck`
- `//:frontend_hermetic_typechecks`

This is deliberately package-level first. It proves the dependency graph and TypeScript
compiler path without pretending that the full Next.js app build is solved.

Verified on WSL2/Linux:

1. `bazelisk build //:frontend_hermetic_typechecks` passed.
2. `bazelisk query //...` passed.
3. `bazelisk test //...` passed with 28 passing tests, 0 skipped, and 0 failing.

## Hermetic apps/web result

The next slice extends Bazel ownership from shared JavaScript packages into the Next.js
application boundary:

- `//:web_typecheck`
- `//:web_typecheck_typecheck_test`
- `//apps/web:next_production_build`
- `//:frontend_hermetic_full_builds`

`//:web_typecheck` is intentionally root-owned because it spans `apps/web`, `packages/ui`, and
`packages/api-types`. The participating packages expose explicit `copy_to_bin` source inputs and
declaration targets, while the root target owns the cross-package TypeScript action. This keeps
package ownership visible instead of flattening everything into `apps/web`.

`//apps/web:next_production_build` runs the Next.js compile phase through the Aspect Next.js Bazel macro
and a repository-owned Next CLI wrapper. The wrapper pins webpack mode for Bazel because the
current Next 16 Turbopack path rejects Bazel sandbox symlinks that point outside the project root.

This is still intentionally narrower than claiming a full production deploy build. The full
Next.js webpack build path currently reaches `/_global-error` prerender and fails on a Next
internal `workStore` invariant. The SSS position is to expose that as a tracked gap instead of
turning it into a fake green build.

Verified on WSL2/Linux:

1. `bazelisk test //:web_typecheck_typecheck_test --verbose_failures` passed.
2. `bazelisk build //:frontend_hermetic_typechecks //:frontend_hermetic_full_builds --verbose_failures` passed.
3. `bazelisk test //... --verbose_failures` passed with 31 passing tests, 0 skipped, and 0 failing.
4. `pnpm --filter @gongzzang/web typecheck` passed.

Bazel Rust test가 Windows rules_rust 한계로 실패하면 실패 내용을 문서화하고 Linux canonical
runner에서 재검증한다. 실패를 숨기거나 Cargo 통과만으로 Bazel 전환 완료를 주장하지 않는다.
## CI-ready enterprise hardening update

2026-06-07 follow-up raised the Bazel design from local migration proof to CI-ready
verification.

Next.js full build ownership:

- `//apps/web:next_production_build`
- `//:web_next_production_build_bazel`
- `//:frontend_hermetic_full_builds`

The former `/_global-error` full-build blocker was caused by forcing
`use_execroot_entry_point = False` on Next Bazel targets. That split Next's CLI/tool path
from the application runtime path between exec and target configurations. Restoring Aspect's
default `nextjs_build` execroot entrypoint behavior keeps Next's async-storage singleton
consistent and makes full `next build --webpack` pass under Bazel.

CI ownership:

- `ci.yml` runs `bazelisk test //... --config=ci` and uploads BEP/profile evidence.
- `frontend.yml` runs Bazel hermetic typecheck, Next compile, and Next full production build
  before transition pnpm lanes.
- `.bazelrc` imports optional `.bazelrc.remote`; `.bazelrc.remote.example` documents managed
  remote cache wiring without committed credentials.

Remaining transition lanes:

- Biome lint remains pnpm-facing.
- Vitest includes Redis-backed tests and needs a service-backed Bazel test contract before a
  hermetic claim.
- Bundle budget remains size-limit CLI based.
- Playwright needs browser binaries plus local app/auth/Redis orchestration.

Verified on WSL2/Linux:

- `bazelisk build //apps/web:next_production_build --verbose_failures` passed.
- `bazelisk build //apps/web:next_production_build --verbose_failures` passed.
- `bazelisk build //:frontend_hermetic_full_builds --config=ci --verbose_failures` passed.
- `pnpm --filter @gongzzang/web typecheck` passed.
