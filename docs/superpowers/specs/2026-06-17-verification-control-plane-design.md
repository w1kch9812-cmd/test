# Verification Control Plane Design

Date: 2026-06-17

## Goal

Make Bazel the single source of truth for repository verification. CI workflows,
Git hooks, local package scripts, and release evidence should call declared
Bazel targets instead of re-encoding verification behavior in YAML or package
manager scripts.

The target state is not "more Bazel commands." The target state is a verification
control plane:

- Package capabilities are declared as package-local Bazel targets.
- Root verification suites are the public API for humans, hooks, and CI.
- Direct verification commands in workflows are rejected unless explicitly
  allowlisted as transition or bootstrap actions.
- Transition wrappers have owners, reasons, and exit conditions.

## Enterprise Evidence

### Google: Monorepo plus build graph as engineering substrate

Google's public monorepo write-up describes the repository as a common source of
truth for a very large engineering organization:

<https://research.google/pubs/why-google-stores-billions-of-lines-of-code-in-a-single-repository/>

The public Bazel site states that Bazel is used for large multi-language,
multi-platform builds and that it scales codebase and CI systems through
dependency analysis, distributed caching, and parallel execution:

<https://bazel.build/>

Relevance for Gongzzang: a large-scale repo needs one declared graph for build
and test behavior. CI should execute graph entrypoints, not own duplicated
verification logic.

### Meta: Build core separated from language rules

Meta's Buck2 announcement emphasizes a large-scale build system whose core is
separated from language-specific rules, with remote execution and a single
incremental dependency graph:

<https://engineering.fb.com/2023/04/06/open-source/buck2-open-source-large-scale-build-system/>

The Buck2 docs explicitly describe targets as queryable graph nodes and call out
multi-language composability and remote execution as common large-repo
properties:

<https://buck2.build/docs/about/why/>

Relevance for Gongzzang: the pattern is not "put commands in CI." The pattern is
to make verification capabilities queryable, composable target graph nodes.

### Uber: Bazel graph used to optimize CI selection

Uber's Go monorepo article describes moving from Make and `go build` to Bazel
when the monorepo model outgrew ad hoc build tooling:

<https://www.uber.com/us/en/blog/go-monorepo-bazel/>

Buildkite's write-up of Uber's monorepo CI explains that Bazel's dependency
graph lets CI avoid validating the entire monorepo for small changes while still
knowing what to check for foundational changes:

<https://buildkite.com/resources/blog/how-uber-halved-monorepo-build-times-with-buildkite/>

Relevance for Gongzzang: once all verification is graph-backed, the repo can
later add affected-target CI without changing the verification contract.

## Design Principles

1. Bazel labels are the verification API.
2. Package-local targets are convention, not preference.
3. Root suites are the only public CI/hook entrypoints.
4. CI YAML owns orchestration only, not verification semantics.
5. Transition wrappers are debt with explicit metadata.
6. Guardrails enforce the policy automatically.
7. Bootstrap/install/deploy commands are allowed only when they are not
   verification semantics.

## Target Taxonomy

Package-local convention targets:

- `:lint`
- `:typecheck`
- `:test`
- `:build`
- `:audit`
- `:release`

Root public suites:

- `//:verify_fast`
- `//:verify_pr`
- `//:verify_release`
- `//:verify_supply_chain`
- `//:verify_all`

Existing transitional suites remain valid only while being migrated:

- `//:frontend_lint`
- `//:frontend_typecheck`
- `//:frontend_unit_test`
- `//:frontend_build`
- `//:frontend_bundle`
- `//:frontend_e2e`
- `//:workspace_typecheck`
- `//:guardrails_all`

## Workflow Policy

GitHub Actions and lefthook may invoke:

- `bazelisk test ...`
- `bazelisk build ...`
- package-manager install/bootstrap commands
- explicit runtime setup commands, for example Playwright browser install
- deployment or cloud CLIs where Bazel is not yet the deployment runner

GitHub Actions and lefthook must not invoke verification semantics directly:

- `pnpm lint`
- `pnpm test`
- `pnpm build`
- `pnpm typecheck`
- `pnpm biome check`
- `pnpm markdownlint-cli2`
- `cargo check`
- `cargo clippy`
- `cargo test`
- `cargo build`

Exceptions require a checked-in transition allowlist entry with:

- command pattern
- owner
- reason
- exit target
- sunset date

## Initial Implementation Scope

The first implementation batch should add policy enforcement without rewriting
every remaining command immediately.

1. Add `docs/architecture/verification-control-plane.v1.json`.
2. Add `scripts/ci/check-verification-control-plane.ps1`.
3. Add tests for the guardrail.
4. Add Bazel target:
   `//tools/bazel:guardrail_verification_control_plane`.
5. Add the guardrail to `//:guardrails_policy_tests` or a dedicated policy
   suite.
6. Allowlist only commands that are still justified:
   - package install/bootstrap
   - Playwright browser install
   - `cargo-deny` until represented as Bazel
   - `cargo-tarpaulin` until represented as Bazel
   - sqlx migration/prepare commands until represented as Bazel
   - supply-chain release packaging until Bazel release outputs become canonical

## Later Migration Batches

Batch 2:

- Add package-local `:lint` convention targets.
- Move Biome and markdownlint to Bazel-native or Bazel-owned execution.
- Replace CI lint direct commands with `//:verify_fast` or `//:verify_pr`.

Batch 3:

- Add package-local `:test` convention targets.
- Move remaining unit tests into Bazel aggregate suites.
- Reduce `frontend_unit_test_transition`.

Batch 4:

- Add package-local `:build` and root `//:verify_release`.
- Make Next and Rust release artifacts Bazel outputs.
- Stop release jobs from packaging ad hoc build directories.

Batch 5:

- Add `:audit` and `//:verify_supply_chain`.
- Move dependency audit, SBOM, provenance, and admission checks into Bazel-owned
  targets.

## Non-Goals

- Do not merge `gongzzang`, `platform-core`, and `dawneer` into one Git repo.
- Do not delete transition wrappers before replacement targets exist.
- Do not force Windows native Bazel execution for JS lifecycle paths; use the
  WSL wrapper until Windows-native Bazel is proven clean.
- Do not hide policy exceptions in comments. Exceptions must be data.

## Acceptance Criteria

- CI/hook verification commands are enforceable by a checked-in guardrail.
- The guardrail has tests that reject direct verification commands.
- All allowed direct commands are explained in a structured allowlist.
- Public docs name Bazel root suites as the verification API.
- `bazelisk test //... --config=ci --verbose_failures` remains green.
