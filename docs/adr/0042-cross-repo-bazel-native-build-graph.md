# ADR-0042: Cross-Repo Bazel-Native Build Graph

| | |
|---|---|
| Date | 2026-06-16 |
| Status | Accepted |
| Decision owner | Platform engineering |

## Context

ADR-0040 made Bazel the Bazel-first build and verification control plane for Gongzzang.
ADR-0041 moved the first JavaScript package and web production build slices into hermetic
Bazel targets. The three-service architecture now has three sibling repositories:

- `gongzzang`
- `platform-core`
- `dawneer`

The final product is still pre-release, so the build architecture should optimize for the
target operating model, not for preserving transitional convenience. A mixed final state
where each repository keeps a different primary build planner would violate the SSS pillars:
consistency, automated enforcement, traceability, safety, visibility, SSOT, and clarity.

The relevant enterprise pattern is a build-platform graph with hermetic toolchains,
declared inputs, content-addressed caching, and policy targets. Public references include
Google Bazel/Blaze, Uber's Go monorepo on Bazel, Airbnb's JVM monorepo migration to Bazel,
Dropbox's Android Bazel migration, and Snowflake's Bazel migration for fast reliable builds.

## Decision

The final build architecture for `gongzzang`, `platform-core`, and `dawneer` is a
**cross-repo Bazel-native build graph**.

- Each repository owns a `MODULE.bazel`, `.bazelversion`, `.bazelrc`, root `BUILD.bazel`,
  and package-local `BUILD.bazel` files.
- Bazel is the canonical build, test, lint, guardrail, generated-contract, and release
  verification entrypoint.
- Cargo, pnpm, Turborepo, PowerShell scripts, and shell wrappers may remain only as migration
  scaffolding or developer convenience, not as final verification SSOT.
- Wrapper targets must have an owner, reason, and exit condition. A wrapper without an exit
  condition is not allowed as the final state.
- Cross-service API contracts, generated clients, policy checks, and cutover guardrails must
  become Bazel targets so the build graph can prove the service boundary.
- The repositories remain physically separate. This ADR does not merge the three repos into a
  single Git repository.
- The first repo to receive the full Rust build graph is `platform-core`, because it is the
  Catalog and Workforce SSOT and currently has a clean worktree.
- `dawneer` Bazel changes must be staged only after its existing dirty worktree is isolated or
  committed, to avoid mixing user work with build-platform migration work.

## Alternatives

- Keep Gongzzang Bazel-first and leave `platform-core`/`dawneer` on Cargo/pnpm/Turbo:
  rejected because it creates permanent cross-repo build inconsistency.
- Use Bazel only as a wrapper around existing commands: rejected as the final state because it
  does not provide hermeticity, graph-level dependency tracing, or meaningful remote caching.
- Migrate all three repos in one large change: rejected because it creates avoidable review and
  rollback risk, especially while `dawneer` has unrelated uncommitted work.
- Adopt Buck2 instead of Bazel: rejected for now because the existing Gongzzang implementation,
  Rust/JS rules, and public operational references are already Bazel-aligned.

## Consequences

- Positive: one build vocabulary across the three-service platform.
- Positive: contract, guardrail, test, and release verification can share one declared graph.
- Positive: managed remote cache/execution becomes a straightforward platform concern instead
  of per-tool optimization.
- Cost: initial target authoring is larger than wrapper-only adoption.
- Cost: Windows direct execution remains a risk for some rules; WSL2/Linux CI is the canonical
  verification environment until the rule/toolchain behavior is proven on Windows.
- Cost: `dawneer` cannot be touched safely until its active worktree is protected.

## Reassessment Triggers

- Bazel rules for Rust or JavaScript become unable to support the pinned toolchains in Linux CI.
- The Bzlmod lockfile strategy cannot be reconciled with repository file-size policy.
- Managed remote cache/execution policy proves operationally unsafe or unaffordable.
- A future build platform provides clearly better Rust/TypeScript support and public operating
  evidence than Bazel for this three-repo architecture.

## Implementation Status

2026-06-16 first slice:

- Added the Platform Core root Bazel control plane.
- Added the first Platform Core Rust Bazel target for `crates/shared-kernel`.
- Verified on WSL2/Linux:
  - `~/.local/bin/bazelisk query //...`
  - `~/.local/bin/bazelisk test //:rust_fast --verbose_failures`
- Windows direct Bazel execution still reproduces the known `crate_universe` symlink privilege
  failure and is not the canonical verification path.

2026-06-16 Platform Core Rust graph expansion:

- Added Bazel targets for all current Platform Core Cargo workspace members.
- Added `//:rust_fast` as the current Platform Core Rust verification suite.
- Added declared runtime data for the service API pipeline graph tests.
- Made pipeline graph default artifact resolution aware of Bazel test runfiles.
- Verified on WSL2/Linux:
  - `~/.local/bin/bazelisk query //...`
  - `~/.local/bin/bazelisk test //:rust_fast --verbose_failures`

## References

- ADR-0040: `docs/adr/0040-bazel-first-build-verification-control-plane.md`
- ADR-0041: `docs/adr/0041-hermetic-javascript-package-bazel-rules.md`
- Bazel users: https://bazel.build/community/users
- Google Bazel open-source project: https://opensource.google/projects/bazel
- Uber Go monorepo with Bazel: https://www.uber.com/us/en/blog/go-monorepo-bazel/
- Airbnb JVM monorepo Bazel migration: https://airbnb.tech/infrastructure/migrating-airbnbs-jvm-monorepo-to-bazel/
- Dropbox Android Bazel migration: https://dropbox.tech/mobile/modernizing-our-android-build-system-part-i-the-planning
- Snowflake Bazel migration: https://www.snowflake.com/en/blog/engineering/fast-reliable-builds-snowflake-bazel/
