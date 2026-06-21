# ADR-0044: Bazel Transition Reconciliation — PowerShell Elimination, Two-Bucket Cleanup, Enablers-First Sequencing

| | |
|---|---|
| Date | 2026-06-20 |
| Status | **REVERSED 2026-06-21 — Bazel 전환 폐기. Cargo가 영구 빌드 SSOT.** (아래 배너 참조) |

> **⛔ 역전 (2026-06-21):** 실제 대기업 사례 조사 + 소유자 Windows 머신 실측 결과, 풀 Bazel은 이
> 프로젝트(소규모 팀 · 3개 polyrepo · Rust 위주 · Windows)에 **부적합**으로 결론. 근거: (1) Bazel
> 빌드가 이 Windows 머신에서 `aws-lc-sys` 컴파일 실패로 **안 됨**; (2) 모든 풀-Bazel 성공 사례는
> 거대 monorepo + 전담 빌드팀 + 원격 실행 클러스터(소규모 Rust polyrepo 사례 0건); (3) Rust 커뮤니티
> 정설 = "작은 Rust 팀은 cargo, Bazel은 정말 필요할 때만". **결정: Bazel 전환 폐기, cargo가 빌드/테스트
> SSOT.** "부분만 실행"은 `cargo test -p <crate>`로 충족. 본 문서의 "go Bazel" 결정은 무효이며,
> 아래 내용은 역사적 기록으로만 남긴다. 신규 Bazel target/파일 추가 금지.
| Decision owner | perfectoryinc (platform owner) |
| Builds on | ADR-0040, ADR-0041, ADR-0042 |
| Supersedes on acceptance | ADR-0002 (in part — turbo/pnpm as *terminal* SSOT), ADR-0043 |
| Reframes | platform-core ADR-0010 (current-state diagnosis only; its rollback clause is void) |

## Context

The cross-repo Bazel effort produced a large, self-contradicting documentation surface. An
adversarial audit of ~39 Bazel-related documents across `gongzzang`, `platform-core`, and
`dawneer` found ~26 documents in direct P0 contradiction with each other. The clearest symptom:
two `platform-core` documents written on the **same day** point in opposite directions —
`docs/superpowers/plans/2026-06-20-true-bazel-build-ssot-transition.md` says *grow Bazel into the
SSOT*, while `docs/research/2026-06-20-bazel-cleanup-inventory.md` classifies nearly the entire
Bazel surface as *replace/delete*.

Root cause: a **federation of PowerShell "registry / projection / ratchet / control-plane"
meta-machines** spread across all three repos. These verify *themselves* (registry-of-registries,
projection writers, impact selectors) rather than producing build/test/release value. They are the
ceremony that makes the strategy look both "grown" and "deleted" at once.

Current reality (verified 2026-06-20):

- `cargo` builds, tests, lints, and ships everything. In `platform-core` CI, Bazel runs **only**
  PowerShell-wrapper guardrail suites; `//:rust_fast` was removed from CI.
- There is **no Dockerfile** in `platform-core`; the release artifact is two `cargo build --release`
  binaries.
- `cargo test --workspace --all-features` is genuinely green (700 passed / 0 failed), but the
  live-service integration suite (Postgres/R2) and the R2 client behavior change are not verified by
  the default gate.

A single, unambiguous direction is required so that humans and code agents stop oscillating between
"build it up" and "tear it down."

## Decision

One direction for `gongzzang`, `platform-core`, and `dawneer` build strategy:

1. **PowerShell is eliminated.** All build/verification logic becomes Rust or native Bazel rules.
   A PowerShell guardrail is never a permitted end-state.

2. **Two-bucket cleanup.** Every existing Bazel-related surface is exactly one of:
   - **DELETE (PowerShell-wrapper meta-machine):** the verification registry / projection /
     CI-fragments, the impact selector, transition-ratchet and verification-task / control-plane
     registries, `shell_test_compat.bzl`, `platform_core_guardrails.bzl`,
     `run_platform_core_powershell.sh` / `run_guardrail_task.sh`, the `//:guardrails_*` and
     `//:national_collection_*` PowerShell-wrapped suites, and the catalog JSON/YML SSOTs that drive
     them. (Full list in *Affected Surfaces* below.)
   - **KEEP + GROW (Rust-native):** the `rules_rust` graph (`crates/**/BUILD.bazel`,
     `services/**/BUILD.bazel`, `tools/bazel/rust_workspace.bzl`, root `BUILD.bazel`, `MODULE.bazel`,
     `.bazelrc`, `.bazelversion`) and `rules_js`/aspect JS rules (ADR-0041), with **dependencies
     generated from cargo/lockfile metadata via `crate_universe`, never hand-mirrored.**

3. **Bazel is the terminal long-term build / test / release SSOT. There is no rollback/abandon.**
   `cargo`, `pnpm`, and `Turborepo` are transitional executors only, not the terminal SSOT.

4. **Sequencing: enablers-first, release cutover last.** Release-artifact cutover must NOT precede
   the enablers in *Open Decisions* below.

5. **Progress is measured by user-visible capability** — remote-cache hit-rate, CI minutes saved,
   one-command cross-repo build — **not** by registry/projection/document volume. No new
   registry/projection/selector meta-machine may be created.

## Enabler Decisions (resolved 2026-06-21)

Priority: do **#4 and #3 now** (they deliver "PowerShell out, all Rust" at zero infra cost); **defer
enablers #1 and #2** (not urgent; #2 is the last step by definition).

1. **Remote cache backend — DEFERRED.** Local disk cache (`--disk_cache`, already configured) is
   sufficient for now. Pick a remote backend only when CI build time or cross-repo result sharing
   becomes the measured bottleneck. No spend now.
2. **Release / OCI artifact model — DEFERRED (last phase).** Not urgent pre-launch; release cutover is
   the final step. Target when chosen: `rules_oci` + distroless + `cosign` keyless + a registry,
   produced from Bazel outputs, replacing `cargo build --release`. Revisit near launch.
3. **Cross-repo build-graph mechanism — ADOPTED.** A slim shared `platform_contracts` Bazel module
   (housed in `platform-core`) that generates the Rust client + TS types as build targets; consumers
   `bazel_dep` on the generated target so the build enforces the contract and the PowerShell pin
   checks are retired. Final form = a private Bazel registry; interim bridge = `git_override` pinned
   to a SHA (NOT `local_path_override`). Prereqs: reconcile Rust toolchains (gongzzang 1.91.1 vs
   platform-core 1.95.0); keep the generated crate cargo-buildable while `dawneer` is not yet
   Bazel-ified.
4. **PowerShell → Rust port triage — ADOPTED (start here).** Value-triage with a DELETE default:
   delete the meta-machine (registry/projection/ratchet/control-plane) and all PowerShell test
   scripts; keep off-the-shelf tools (gitleaks, lefthook, cargo-deny); port only the few real
   repo-specific guards into one `repo-guard` Rust binary. End state: PowerShell scripts → 0.
   **Carve-out (verified 2026-06-21):** the `traffic-auth-policy-registry` / `traffic-auth-policy-generator`
   cluster (~16 PowerShell files + `docs/architecture/traffic-auth-policy-registry.v1.json`, 1000 lines)
   is NOT ceremony — it is the auth / authz / rate-limit + web↔API-proxy policy SSOT. Its 6-phase
   generator emits real runtime code that the app imports: `services/api/src/traffic_auth_policy.rs`
   (used by `app.rs` for `BackendAuthorizationState` + rate policies) and `listing_marker_policy.rs`,
   plus `apps/web/lib/policies/traffic-auth-policy.generated.ts` and `.../api/api-proxy-client.generated.ts`
   (imported across the web data layer: buildings/parcels/listings/notifications + the `/api/proxy` route).
   Therefore KEEP it; it is the LARGEST, most careful PORT — the PowerShell generator → a Rust
   codegen step, with regenerated-output parity verification. This generator was the last to be
   ported; it is now the Rust binary `cargo run -p api --bin generate-traffic-auth-policy`, which
   produces the committed `.generated.*` files, completing PowerShell scripts → 0.

## Alternatives

- **Cargo-per-repo, drop Bazel entirely** (follow the cleanup inventory to its end): rejected — the
  owner has chosen enterprise Bazel; the cross-repo graph value is wanted.
- **Keep both directions / keep PowerShell wrappers as a long-term state**: rejected — this *is* the
  current contradiction and the source of the ceremony.
- **Adopt Buck2 instead**: rejected, consistent with ADR-0042.

## Consequences

- Positive: one direction; the ~26 P0 contradictions resolve to a single rule; ceremony is deleted;
  the consistency / SSOT pillars are restored.
- Positive: code agents (including Codex) receive an unambiguous spec.
- Cost: large documentation + code cleanup.
- Cost: the four enablers require real infrastructure investment before release cutover.
- Honest limitation: **until the enablers land, Bazel is NOT yet the real build system** — `cargo`
  remains the shipping path *transitionally*. No "Bazel complete" claim is permitted in this window.

## Affected Surfaces

**DELETE bucket (after a Rust-native or direct-CI replacement exists, never before):**

- `platform-core`: `docs/catalog/verification-registry-bazel.v1.json` (+ `.projection.v1.json`,
  `.ci-fragments.v1.yml`), `verification-impact-map-bazel.v1.json`,
  `local-bazel-execution-policy.v1.json`, `guardrail-transition-suite-contract.v1.json`;
  `tools/bazel/platform_core_guardrails.bzl`, `shell_test_compat.bzl`,
  `run_platform_core_powershell.sh`; `services/outbox-publisher/src/bin/bazel_verification_selector.rs`;
  the `//:guardrails_*` / `//:national_collection_*` suites; the ceremony PowerShell scripts under
  `scripts/**`.
- `gongzzang`: `verification-control-plane.v1.json` + its `check-verification-control-plane` guard;
  `verification-transition-ratchet.v1.json` + its `check-bazel-transition-ratchet` guard;
  `verification-task-registry.v1.json` + its `check-verification-task-registry` guard;
  `shell_test_compat.bzl`, `run_guardrail_task.sh`, the `guardrails_*` wrapper targets.

**Documents superseded (banner pointing here, on acceptance):**

- `platform-core`: `docs/research/2026-06-20-bazel-cleanup-inventory.md`,
  `docs/research/2026-06-20-rust-fast-cargo-coverage-map.md`.
- `gongzzang`: ADR-0043; `docs/superpowers/plans/2026-06-17-verification-control-plane.md`,
  `2026-06-18-bazel-transition-ratchet.md`, `2026-06-18-verification-task-registry-ssot.md`;
  `docs/superpowers/handoff/2026-06-07-bazel-commit-boundary.md`.
- ADR-0002: turbo/pnpm downgraded from terminal SSOT to transitional executor.
- `platform-core` ADR-0010: kept as current-state diagnosis only; its "rollback criteria" clause is
  void (this ADR establishes no-rollback).

**Documents edited to align** (enablers-first, no-rollback, PowerShell-elimination, cargo-metadata
deps): `platform-core` ADR-0011 + the transition plan + remaining 2026-06-20 research notes;
`gongzzang` ADR-0040, ADR-0042, the 2026-06-07 plans + spec, the 2026-06-16 cross-repo plan, the
supply-chain-evidence plan, the verification-control-plane design spec, the concurrent-session
handoff, `architecture/observability.md`, `architecture/layers.md`,
`runbooks/supply-chain-provenance-and-deploy-gate.md`, `superpowers/next-actions.md`, `adr/README.md`;
`dawneer` cross-repo alignment plan + spec.

## Reassessment Triggers

> No-rollback (Decision 3) means these triggers cause a **re-plan, not an abandon** of Bazel.

- A `rules_rust` / `rules_js` rule cannot support the pinned toolchain in Linux CI → fix/replace the
  rule.
- The Bzlmod lockfile strategy cannot reconcile with the file-size policy → adjust policy or lockfile
  handling.
- A chosen remote-cache backend proves unaffordable/unsafe → choose another backend (Open Decision 1).

## Implementation Status

- 2026-06-20: this ADR proposed; ~39-document adversarial reconciliation audit complete.
- Checkbox reality of the `platform-core` transition plan: Tasks 1–2 done, Tasks 3–4 partial,
  Tasks 5–7 not started.
- Pending: owner fills the four Open Decisions → then supersession banners, Rust-native replacements,
  and DELETE-bucket removal (delegated to a code agent), enablers before release cutover.

## References

- ADR-0040, ADR-0041, ADR-0042, ADR-0043; ADR-0002.
- `platform-core` ADR-0010, ADR-0011;
  `docs/superpowers/plans/2026-06-20-true-bazel-build-ssot-transition.md`;
  `docs/research/2026-06-20-bazel-cleanup-inventory.md`.
- Cross-repo architecture: ADR-0030.
