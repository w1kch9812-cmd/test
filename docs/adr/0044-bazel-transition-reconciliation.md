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
| Reverses | ADR-0040, ADR-0041, ADR-0042, ADR-0043 (Bazel adoption — all superseded-bannered) |
| Reaffirms | ADR-0002 (turbo/pnpm as terminal SSOT) + platform-core ADR-0010 (cargo as build SSOT) |

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

State at the 2026-06-20 audit (Bazel has since been removed entirely — cargo is now the sole build system):

- `cargo` built, tested, linted, and shipped everything. At that time `platform-core` CI ran Bazel
  **only** for PowerShell-wrapper guardrail suites (`//:rust_fast` was already excluded from CI); all
  of that Bazel scaffolding was removed on 2026-06-21.
- There is **no Dockerfile** in `platform-core`; the release artifact is two `cargo build --release`
  binaries.
- `cargo test --workspace --all-features` is genuinely green (700 passed / 0 failed), but the
  live-service integration suite (Postgres/R2) and the R2 client behavior change are not verified by
  the default gate.

A single, unambiguous direction is required so that humans and code agents stop oscillating between
"build it up" and "tear it down."

## Decision (final — 2026-06-21 reversal)

One direction for `gongzzang`, `platform-core`, and `dawneer`:

1. **Bazel is abandoned. `cargo` (Rust) + `pnpm`/`Turborepo` (frontend) are the PERMANENT build,
   test, lint, and release SSOT.** There are no Bazel files, targets, rules, registries, or
   `MODULE.bazel`/`BUILD.bazel`/`.bazelrc` anywhere in the three repos. (Why: this owner's Windows
   machine cannot build under Bazel — `aws-lc-sys` fails; every full-Bazel success story is a giant
   monorepo with a dedicated build team and remote-execution cluster, with zero small-Rust-polyrepo
   precedent; the Rust-community norm is cargo for small teams. See the reversal banner above.)

2. **"Run only part, not everything"** — the original reason Bazel was considered — is native and
   already works: `cargo build|test|check -p <crate>` (Rust) and `pnpm turbo <task> --filter <pkg>`
   (frontend). No Bazel is required to get partial/incremental builds.

3. **PowerShell is eliminated.** All build/verification logic is Rust or standard tools (cargo-deny,
   gitleaks, lefthook). The PowerShell "registry / projection / ratchet / control-plane" meta-machines
   were deleted; the few real repo-specific guards were ported to one `repo-guard` Rust binary.
   PowerShell scripts → **0** across all three repos.

4. **No meta-machines.** No verification / projection / ratchet / registry that verifies *itself*.
   **Progress is measured by user-visible capability — not by registry/projection/document volume.**

## Cleanup outcome (2026-06-21)

The Bazel-specific "enablers" from the original plan — a remote-cache backend, an OCI/release-via-Bazel
artifact model, and a cross-repo Bazel `platform_contracts` module — are **void**: Bazel is abandoned,
so none are built. The one piece that was real, and is **done**:

- **PowerShell → Rust port (completed).** Value-triage with a DELETE default: the meta-machine
  (registry / projection / ratchet / control-plane) and all PowerShell test scripts were deleted;
  off-the-shelf tools (gitleaks, lefthook, cargo-deny) were kept; the few real repo-specific guards were
  ported into one `repo-guard` Rust binary. **PowerShell scripts → 0.**
  - **Carve-out (real capability, ported — not ceremony):** the `traffic-auth-policy` generator cluster
    is the auth / authz / rate-limit + web↔API-proxy policy SSOT. Its 6-phase PowerShell generator emitted
    real runtime code the app imports: `services/api/src/traffic_auth_policy.rs` (used by `app.rs` for
    `BackendAuthorizationState` + rate policies) and `listing_marker_policy.rs`, plus
    `apps/web/lib/policies/traffic-auth-policy.generated.ts` and `.../api/api-proxy-client.generated.ts`
    (imported across the web data layer + the `/api/proxy` route). It was ported to the Rust binary
    `cargo run -p api --bin generate-traffic-auth-policy`, which reproduces the committed `.generated.*`
    files byte-for-byte (a CI drift-guard enforces parity). SSOT = the single aggregate
    `docs/architecture/traffic-auth-policy-registry.v1.json`.

## Alternatives

- **cargo + pnpm/Turbo, no Bazel (native partial builds)** — **ADOPTED** (this decision). `cargo -p`
  and `turbo --filter` cover the partial / cross-repo build need without monorepo-scale Bazel overhead.
- **Full Bazel as the terminal long-term SSOT** — **REJECTED (2026-06-21).** This was the
  originally-proposed direction of this ADR; reversed because Bazel does not build on the team's actual
  platform (Windows `aws-lc-sys` failure) and has no small-Rust-polyrepo precedent. Re-adoption requires
  a new ADR (see *Re-adoption bar*).
- **Keep PowerShell wrappers as a long-term state** — rejected: this was the source of the ceremony/contradiction.
- **Adopt Buck2 instead** — rejected, consistent with ADR-0042.

## Consequences

- Positive: one direction — cargo/native; the ~26 P0 Bazel contradictions resolve to "cargo is the
  permanent SSOT, Bazel abandoned." Ceremony deleted; consistency / SSOT pillars restored.
- Positive: code agents (including Codex) receive an unambiguous spec.
- Positive: no remote-cache / OCI-via-Bazel / cross-repo-Bazel-graph infrastructure is needed — those
  were enablers for the abandoned Bazel plan.
- Cost: a large one-time documentation + code cleanup (Bazel files, PowerShell, ceremony) — completed.
- `cargo` is the permanent build system, not a transitional stopgap; there is no "terminal vs transitional" split.

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
- ADR-0002: turbo/pnpm **reaffirmed** as the terminal frontend SSOT (the earlier "downgrade to
  transitional" is reversed); cargo joins as the terminal Rust build/test/release SSOT.
- `platform-core` ADR-0010 (Cargo Build SSOT + Bazel Freeze): **reaffirmed** — cargo is the permanent
  SSOT and Bazel is abandoned; the diagnosis stands and there is no Bazel-freeze thaw.

**Documents edited to align** (PowerShell elimination, Bazel removal, cargo as the build SSOT):
`platform-core` ADR-0011 + the transition plan + remaining 2026-06-20 research notes;
`gongzzang` ADR-0040, ADR-0042, the 2026-06-07 plans + spec, the 2026-06-16 cross-repo plan, the
supply-chain-evidence plan, the verification-control-plane design spec, the concurrent-session
handoff, `architecture/observability.md`, `architecture/layers.md`,
`runbooks/supply-chain-provenance-and-deploy-gate.md`, `superpowers/next-actions.md`, `adr/README.md`;
`dawneer` cross-repo alignment plan + spec.

## Re-adoption bar

Bazel is abandoned, not paused. Re-adopting it requires a NEW ADR demonstrating BOTH: (a) Bazel builds
on the team's actual dev machines (including the owner's Windows host, which currently fails on
`aws-lc-sys`), and (b) a concrete, measured pain that `cargo build|test|check -p <crate>` /
`pnpm turbo --filter` cannot solve. Absent that, do not add Bazel files, targets, rules, or registries.

## Implementation Status

- 2026-06-20: proposed (originally "reconcile toward Bazel"); ~39-document adversarial audit.
- 2026-06-21: **reversed** — Bazel abandoned; cargo/native is the permanent build/test/release SSOT.
- Done: PowerShell → 0 (all three repos); all Bazel files/targets/registries removed; the traffic-auth
  generator ported to Rust with a CI drift-guard; pro-Bazel ADRs (0040–0043, platform-core 0011)
  superseded-bannered; Bazel + deleted-CI-machinery doc references reconciled across the three repos.
- No open enablers remain (the Bazel enablers are void).

## References

- ADR-0040, ADR-0041, ADR-0042, ADR-0043; ADR-0002.
- `platform-core` ADR-0010, ADR-0011;
  `docs/superpowers/plans/2026-06-20-true-bazel-build-ssot-transition.md`;
  `docs/research/2026-06-20-bazel-cleanup-inventory.md`.
- Cross-repo architecture: ADR-0030.
