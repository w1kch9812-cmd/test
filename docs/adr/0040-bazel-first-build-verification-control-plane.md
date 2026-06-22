# ADR-0040: Bazel-first build and verification control plane

> ⛔ **[ADR-0044](./0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 **cargo(Rust) + pnpm/Turbo(프론트)가 영구 빌드 SSOT**다. 이 문서는 (취소된) Bazel-first 결정의 *역사적 기록*일 뿐 — 구현하지 말 것.

| | |
|---|---|
| 작성일 | 2026-06-07 |
| 상태 | Superseded by ADR-0044 |
| 결정자 | Platform engineering |

## 컨텍스트

ADR-0002는 `Cargo + pnpm + Turborepo`를 monorepo 기본값으로 정하고 Bazel은 Phase 3+
검토 대상으로 남겼다. 이후 Gongzzang, Platform Core, Dawneer가 sibling repo로 분리되면서
검증 대상이 늘었고, Rust/TypeScript/SQL/문서/경계 guardrail을 매번 긴 직렬 스크립트로
검사하는 방식은 시간이 너무 오래 걸린다.

사용자는 자체 build planner 구현이 아니라 실제 대기업이 쓰는 방식에 맞춘 표준 도구 채택을
요구했다. Bazel 공식 사용자 목록에는 Google, Canva, Databricks, Dropbox, Redfin, Stripe,
Uber, LinkedIn, Nvidia 등이 공개되어 있고, Bazel 공식 remote caching 문서는 build를 명시적
input/output/action graph로 쪼개 action cache와 content-addressable store를 공유하는 모델을
설명한다.

## 결정

Gongzzang의 build/verification control plane은 **Bazel-first**로 전환한다.

- Bazel은 보조 실험 도구가 아니라 repo 표준 검증/빌드 진입점이 된다.
- `.bazelversion`으로 Bazel 버전을 고정하고 Bazelisk로 실행한다.
- 외부 의존성은 legacy `WORKSPACE`가 아니라 Bzlmod `MODULE.bazel`로 관리한다.
- `MODULE.bazel.lock`은 현재 생성 결과가 1500줄을 초과하므로 이 repo의 file-size 헌법과
  충돌한다. 별도 ADR-approved lockfile partition 또는 예외 전략 전까지는
  `--lockfile_mode=off`로 두고 커밋하지 않는다.
- Rust는 `rules_rust`와 `crate_universe`를 사용한다.
- Cargo workspace와 `Cargo.lock`은 전환기 동안 계속 유지하되, Bazel target이 점진적으로
  공식 검증 단위가 된다.
- pnpm/Turborepo는 TypeScript/Next.js 영역이 Bazel target으로 옮겨질 때까지 전환기 실행기로
  둔다.
- Windows 개발 머신은 Bazelisk 실행을 허용하지만, `rules_rust` 공식 문서가 Linux/macOS를
  주 지원 대상으로 설명하므로 Rust Bazel release gate의 canonical 실행 환경은 WSL2/Linux
  또는 Linux CI runner로 둔다.
- managed remote cache/execution은 credentials, cache-write policy, ownership, observability가
  준비된 뒤 BuildBuddy, EngFlow, Depot, GCS-compatible cache 등 관리형 선택지로 붙인다.
  자체 remote execution 시스템은 만들지 않는다.

## 대안

- Cargo + pnpm + Turborepo 유지: 현재 구조와 호환은 좋지만, cross-repo/affected verification과
  remote cache 기반 대기업식 운영으로 가기 어렵다.
- Buck2 도입: Meta식이고 Rust core라 매력적이지만 외부 생태계와 공개 운영 사례가 Bazel보다
  작다. 자체 연결 작업이 늘 가능성이 있다.
- 자체 Rust guardrail planner 구현: 프로젝트 취향에는 맞지만 사용자가 원하는 "대기업 표준 도구"
  요구와 충돌한다.

## 결과

- 긍정: 변경 영향 기반 검증, 로컬/원격 캐시, 명시적 dependency graph, hermetic toolchain으로
  긴 guardrail 시간을 구조적으로 줄일 수 있다.
- 긍정: Google/Canva/Databricks/Dropbox/Redfin 등 공개 사례와 같은 방향의 build platform을
  채택한다.
- 비용: Bazel target 작성, rules 학습, Windows 직접 실행 리스크, Cargo/Turbo와의 전환기 중복이
  생긴다. Bzlmod lockfile은 현재 line-limit와 충돌해 별도 해결이 필요하다.
- 영향 영역: root build files, Rust crates, CI/lefthook guardrail, TypeScript apps/packages,
  cross-repo verification docs.

## 재검토 트리거

- Bazel migration 때문에 핵심 개발 속도가 2주 이상 지속적으로 악화된다.
- `rules_rust` 또는 Bzlmod가 현재 Rust toolchain과 호환되지 않아 Linux canonical runner에서도
  안정화되지 않는다.
- Buck2의 외부 생태계와 Rust monorepo 운영 사례가 Bazel 대비 명확히 우위가 된다.
- managed remote cache 비용이 기대 성능 개선 대비 과도하다.

## 참조

- Bazel users: https://bazel.build/community/users
- Bazel remote caching: https://bazel.build/remote/caching
- Bazel Bzlmod overview: https://bazel.build/external/overview
- rules_rust introduction: https://bazelbuild.github.io/rules_rust/
- rules_rust crate_universe: https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html

## Implementation status

2026-06-07 first-wave implementation completed the Rust Cargo workspace migration into
Bazel targets:

- Root Bazel control plane: `.bazelversion`, `.bazelrc`, `.bazelignore`, `MODULE.bazel`,
  root `BUILD.bazel`.
- Rust target SSOT: `tools/bazel/rust_workspace.bzl`.
- All workspace packages under `crates/` have Bazel library and unit-test targets.
- Rust service binaries are represented for `services/api`, `services/outbox-publisher`,
  and `services/etl-base-layer`.
- `crates/db` compiles under Bazel with SQLx offline proc macros by declaring `.sqlx`,
  Cargo workspace metadata, Bazel control-plane metadata, and a local `cargo metadata`
  shim as hermetic compile inputs.

Verified on WSL2/Linux:

- `bazelisk query //...` passed and listed 56 targets.
- `bazelisk test //...` passed with 26 passing tests, 0 skipped, and 0 failing.

Remaining out of scope for this ADR's first wave:

- Frontend/package Bazel targets.
- CI/lefthook replacement with Bazel entrypoints.
- Managed remote cache/execution credentials and policy.
- Bzlmod lockfile strategy that satisfies the repository line-limit rule.

## Transition wrapper status

2026-06-07 follow-up added manual local Bazel wrappers for existing frontend and guardrail
commands without introducing new npm, Cargo, or Bazel module dependencies.

Added frontend entrypoints:

- `//:frontend_lint`
- `//:frontend_typecheck`
- `//:frontend_unit_test`
- `//:frontend_build`
- `//:frontend_bundle`
- `//:frontend_e2e`
- `//:frontend_verification`
- `//:frontend_release_verification`

Added guardrail suites:

- `//:guardrails_fast`
- `//:guardrails_policy`
- `//:guardrails_policy_tests`
- `//:guardrails_all`

These targets are intentionally tagged `manual`, `local`, and `no-sandbox`. They make Bazel
the visible entrypoint for the existing pnpm/Turbo/bash/PowerShell surface, while avoiding a
false claim that the JavaScript workspace is already hermetic under Bazel. Full JS Bazel
hermeticity remains a separate ADR-level migration.

## Hermetic JavaScript package status

2026-06-07 follow-up adopted ADR-0041 and added the first hermetic JavaScript package
targets using `rules_nodejs`, `aspect_rules_js`, and `aspect_rules_ts`.

Added package targets:

- `//packages/api-types:api_types_typecheck`
- `//packages/ui:ui_typecheck`
- `//:frontend_hermetic_typechecks`

Root package changes:

- `.nvmrc` now matches the Node engine floor used by CI: `20.19.0`.
- `package.json` pins the root TypeScript compiler at `5.9.3`, matching `pnpm-lock.yaml`.
- `package.json#pnpm.onlyBuiltDependencies` explicitly allowlists npm lifecycle hook packages.
- `REPO.bazel` ignores nested `node_modules` directories for Bazel 8+.

Verified on WSL2/Linux:

- `bazelisk build //:frontend_hermetic_typechecks` passed.
- `bazelisk query //...` passed.
- `bazelisk test //...` passed with 28 passing tests, 0 skipped, and 0 failing.

Remaining JavaScript work before the enterprise hardening update:

- `apps/web` full Next.js prerender production build is not yet a completion target.
- Vitest, Playwright, and bundle-budget checks are still transition wrappers.
- CI still needs to call the Bazel entrypoints directly.

## Hermetic apps/web status

2026-06-07 follow-up added the first hermetic `apps/web` Bazel targets.

Added web targets:

- `//:web_typecheck`
- `//:web_typecheck_typecheck_test`
- `//apps/web:next_js_binary`
- `//apps/web:next_production_build`
- `//:web_typecheck_bazel`
- `//:web_next_production_build_bazel`
- `//:frontend_hermetic_full_builds`

The root-owned `web_typecheck` target owns the cross-package TypeScript graph for
`apps/web`, `packages/ui`, and `packages/api-types`. This avoids making `apps/web`
pretend it can own parent workspace packages, while still keeping source inputs and
npm dependencies explicit through Bazel.

The original `next_compile` target used Aspect's Next.js Bazel macro with the
repository-pinned Next.js package and Bazel-provided Node toolchain. That compile-only target
was retired by the enterprise hardening update because it generated the same `.next` output as
the full production target:

- Next 16 defaults to Turbopack, but Turbopack currently fails inside the Bazel sandbox
  on the symlinked `package.json` path. The Bazel wrapper therefore uses the official
  `next build --webpack` CLI path for this target.
- The final production target validates the app bundle, TypeScript path, and prerender path
  with declared Bazel inputs, without reading ambient `apps/web/.next` or local `node_modules`.

Verified on WSL2/Linux before the enterprise hardening update:

- `bazelisk test //:web_typecheck_typecheck_test` passed.
- The earlier typecheck and compile-only Bazel targets passed before the Next output was
  consolidated into `//apps/web:next_production_build`.
- `bazelisk test //...` passed with 31 passing tests, 0 skipped, and 0 failing.
- `pnpm --filter @gongzzang/web typecheck` passed.

Observed hardening from this slice:

- `apps/web/.next`, `.turbo`, local `node_modules`, Playwright reports, test results, and
  `var` outputs are explicitly ignored from Bazel source discovery.
- Production app sources are separated from test/tooling sources for Next build inputs.
- `@gongzzang/api-types` and `@gongzzang/ui` are marked as Next transpile packages.
- The authenticated route group explicitly declares `dynamic = "force-dynamic"` because it
  reads cookies/session state and must not be statically prerendered.

## Enterprise hardening update: CI and Next full build

2026-06-07 follow-up raised the Bazel surface from local migration proof to CI-ready
verification entrypoints.

Added CI and remote-cache-ready controls:

- `.github/workflows/ci.yml` now has a `bazel-fast-graph` job that installs the
  repo-owned Bazelisk binary and runs `bazelisk test //... --config=ci`.
- The Bazel CI job writes Build Event Protocol JSON and a Bazel profile under
  `target/bazel/` and uploads them as evidence artifacts.
- `.bazelrc` imports optional `.bazelrc.remote`, and `.bazelrc.remote.example` documents
  managed remote cache wiring without committing credentials.
- `.bazelrc` keeps `--remote_download_minimal` in `--config=ci` and leaves
  `--remote_download_toplevel` as an explicit developer config.

Added full frontend Bazel production build coverage:

- `//apps/web:next_production_build`
- `//:web_next_production_build_bazel`
- `//:frontend_hermetic_full_builds`

The prior `/_global-error` full-build blocker was traced to a Bazel execution-path issue,
not an application route bug. Forcing `use_execroot_entry_point = False` caused the Next.js
CLI/tool path and application runtime path to diverge between exec and target configurations.
Next's request async-storage singleton then appeared uninitialized during prerender. Removing
that override returned Aspect's `nextjs_build` macro to its default execroot entrypoint
behavior and made the full `next build --webpack` target pass under Bazel.

`apps/web/app/global-error.tsx` was also added as a root-layout error boundary. It is
self-contained, avoids depending on the normal root layout or UI package, and reads existing
`errorPage.*` i18n keys through the static i18n helper.

Verified on WSL2/Linux:

- `bazelisk build //apps/web:next_production_build --verbose_failures` passed.
- `bazelisk build //:frontend_hermetic_full_builds --config=ci --verbose_failures` passed.
- `pnpm --filter @gongzzang/web typecheck` passed.
