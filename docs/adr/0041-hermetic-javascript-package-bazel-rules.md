# ADR-0041: Hermetic JavaScript package Bazel rules

> ⛔ **[ADR-0044](./0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 **cargo(Rust) + pnpm/Turbo(프론트)가 영구 빌드 SSOT**다. 이 문서는 (취소된) Bazel 규칙 결정의 *역사적 기록*일 뿐 — 구현하지 말 것.

| | |
|---|---|
| 작성일 | 2026-06-07 |
| 상태 | Superseded by ADR-0044 |
| 결정자 | Platform engineering |

## 컨텍스트

ADR-0040으로 Rust workspace는 Bazel target graph에 올라갔지만, frontend/package
영역은 아직 pnpm/Turbo를 호출하는 transition wrapper 상태였다. 이 상태는 Bazel을
entrypoint로 보이게 만들 수는 있지만 Node, pnpm, TypeScript compiler, npm dependency
graph를 Bazel이 직접 통제하지 못한다.

대기업 수준의 build control plane으로 가려면 JavaScript도 ambient `node_modules`나
로컬 PATH에 의존하지 않고, lockfile 기반 dependency graph와 hermetic toolchain으로
분해되어야 한다.

## 결정

Gongzzang의 JavaScript/TypeScript package Bazel 전환은 `aspect_rules_js`와
`aspect_rules_ts`를 공식 경로로 채택한다.

- `rules_nodejs` module extension이 `.nvmrc`의 Node 버전을 Bazel toolchain으로 고정한다.
- `aspect_rules_js`의 `npm_translate_lock`이 `pnpm-lock.yaml`을 Bazel dependency graph로
  번역한다.
- `aspect_rules_ts`의 `ts_project`가 TypeScript package typecheck를 Bazel action으로
  실행한다.
- pnpm lifecycle hook 실행은 root `package.json`의 `pnpm.onlyBuiltDependencies`
  allowlist로 명시한다. 임의 npm lifecycle script 실행은 허용하지 않는다.
- Bazel 8+ node_modules ignore는 `REPO.bazel`의 `ignore_directories(["**/node_modules"])`
  로 관리한다.

첫 적용 범위는 package layer로 제한한다.

- `packages/api-types`
- `packages/ui`
- root entrypoint: `//:frontend_hermetic_typechecks`

`apps/web`의 Next.js build, Vitest, Playwright, bundle budget은 아직 transition wrapper
상태로 남긴다. Next.js hermetic build는 별도 단계에서 다룬다.

## 대안

- pnpm/Turbo wrapper 유지: 이행은 쉽지만 Node/pnpm/dependency graph가 ambient 상태로
  남아 hermetic이라고 말할 수 없다.
- 자체 TypeScript runner 작성: 통제는 가능하지만 build system을 직접 만들게 되어
  Bazel 채택 이유와 충돌한다.
- Bazel 없이 CI matrix만 분리: 병렬성은 개선되지만 action cache, affected graph,
  dependency traceability는 얻기 어렵다.

## 결과

- 긍정: JavaScript package typecheck가 Bazel sandbox와 npm lockfile graph 안에서 돈다.
- 긍정: `bazel test //...`가 Rust 26개 테스트와 TypeScript typecheck 2개를 함께 검증한다.
- 긍정: npm lifecycle hook allowlist가 생겨 공급망 스크립트 실행 표면이 명시화됐다.
- 비용: `MODULE.bazel`에 JS rule dependency surface가 추가됐다.
- 비용: `apps/web` 전체 hermetic build는 아직 남아 있다.
- 영향 영역: `MODULE.bazel`, `REPO.bazel`, root/package `BUILD.bazel`, `package.json`,
  `pnpm-lock.yaml`, `.nvmrc`.

## 재평가 트리거

- `apps/web` Next.js build를 Bazel action으로 올릴 때.
- `aspect_rules_js` 또는 `aspect_rules_ts` 주요 버전 변경 시.
- pnpm major upgrade 시 lifecycle hook 설정 위치가 바뀔 때.
- managed remote cache 도입 전 cache hit/miss policy를 확정할 때.

## 참조

- Bazel Central Registry `aspect_rules_js`: https://registry.bazel.build/modules/aspect_rules_js
- Bazel Central Registry `aspect_rules_ts`: https://registry.bazel.build/modules/aspect_rules_ts
- Bazel Central Registry `rules_nodejs`: https://registry.bazel.build/modules/rules_nodejs
- rules_js npm extension docs: https://registry.bazel.build/docs/aspect_rules_js
- rules_ts API docs: https://registry.bazel.build/docs/aspect_rules_ts/3.8.5

## 2026-06-07 apps/web hermetic slice

The first `apps/web` hermetic slice is now represented by Bazel targets:

- `//:web_typecheck`
- `//:web_typecheck_typecheck_test`
- `//apps/web:next_js_binary`
- `//apps/web:next_production_build`
- `//:web_typecheck_bazel`
- `//:web_next_production_build_bazel`
- `//:frontend_hermetic_full_builds`

`next_production_build` deliberately means the full official Next.js production build under
Bazel. The earlier compile-only target was retired because both targets generated the same
`.next` output directory.

Current rationale:

- The root `web_typecheck` target owns the cross-package TypeScript graph for `apps/web`,
  `packages/ui`, and `packages/api-types`.
- `apps/web` source copying, workspace package source copying, package declaration targets,
  and npm dependencies are explicit Bazel inputs.
- The target does not read ambient `apps/web/.next` or local source `node_modules`.
- Next 16 Turbopack currently fails in the Bazel sandbox on a symlinked `package.json`; the
  wrapper therefore uses the official `next build --webpack` option for the Bazel production
  build.
- The former full webpack `next build` `/_global-error` workStore failure was resolved by
  restoring Aspect's default execroot entrypoint behavior.

Verified:

- `bazelisk test //:web_typecheck_typecheck_test` passed on WSL2/Linux.
- The earlier typecheck and compile-only Bazel targets passed before the Next output was
  consolidated into `//apps/web:next_production_build`.
- `bazelisk test //...` passed with 31 passing tests, 0 skipped, and 0 failing.
- `bazelisk build //apps/web:next_production_build` passed on WSL2/Linux.
- `pnpm --filter @gongzzang/web typecheck` passed.

## 2026-06-07 full Next build hardening

The full `apps/web` production build is now represented by Bazel:

- `//apps/web:next_production_build`
- `//:web_next_production_build_bazel`
- `//:frontend_hermetic_full_builds`

The earlier `/_global-error` `workStore` failure was resolved by removing the
`use_execroot_entry_point = False` override from Next Bazel targets. That override split
Next's tool/runtime resolution between exec and target configurations. With the default
Aspect `nextjs_build` entrypoint behavior restored, Next's async-storage singleton remains
consistent and full `next build --webpack` passes in Bazel.

The production build remains on webpack for the Bazel path. Next 16's default Turbopack
path is still not claimed as hermetic here because the current sandboxed symlink layout is
not equivalent to the pnpm workspace execution path.

Current JavaScript ownership boundary:

- Hermetic under Bazel: package typechecks, root web typecheck, and Next full production build.
- Transitional by design: Biome lint, Vitest, bundle budget, and Playwright. Vitest currently
  includes Redis-backed tests; Playwright needs browser installation and managed local
  services; bundle budget still uses size-limit's pnpm-facing CLI.

Exit conditions for remaining transition wrappers:

- Biome lint exits when represented by a direct Bazel JS binary/test target with declared
  config and source inputs.
- Vitest exits when Redis-backed tests are split into explicit service-backed CI lanes or
  moved behind a Bazel-compatible test environment contract.
- Bundle budget exits when size-limit inputs and browser/runtime requirements are declared
  as Bazel data/toolchain inputs.
- Playwright exits when browser binaries, local app startup, Redis, and auth provider
  dependencies are represented by a Bazel-compatible integration-test harness.

Verified on WSL2/Linux:

- `bazelisk build //:frontend_hermetic_full_builds --config=ci --verbose_failures` passed.
- `pnpm --filter @gongzzang/web typecheck` passed.
