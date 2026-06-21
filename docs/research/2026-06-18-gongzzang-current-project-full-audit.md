# Gongzzang Current Project Full Audit

Date: 2026-06-18
Repo: `C:\Users\admin\Desktop\gongzzang`

> Historical snapshot. This audit predates the PowerShell elimination (ADR-0044,
> 2026-06-21). The PowerShell guard/registry machinery named below has since been
> removed; the surviving guards are `scripts/lefthook/*.sh` and the `repo-guard`
> Rust binary, and the traffic-auth policy is regenerated with
> `cargo run -p api --bin generate-traffic-auth-policy`. Guard names below are kept
> for the historical record.

## 0. Current Rescan Summary

Current rescan status:

- After the guardrail follow-up commits, the tracked code worktree was clean before this audit
  note update.
- The local branch may be ahead of `origin/main` when this note is edited; that is commit state,
  not an uncommitted-code risk.
- The verification-task-registry draft has been promoted into a committed guardrail.
- No public-data collection was started.
- The main product/Catalog ownership guardrails pass.
- The 1500-line hard gate passes.
- Frontend typecheck/unit, Rust format/check/lint, Biome, markdown links, and the full guardrail
  suite pass.
- Runtime code search did not find direct V-World/data.go.kr Catalog endpoint usage in `apps/`,
  `services/`, `crates/`, or `packages/`.
- The prior red item was intentional maintainability ratcheting:
  `scripts/ci/check-platform-core-boundary tests` now self-checks that the boundary
  checker and its modules stay <=600 lines.
- That red item is now resolved: the test runner is 315 lines, the checker
  orchestrator is 12 lines, and every boundary checker module is <=201 lines.
- The generated/compatibility aggregate large-file gap is now governed by an explicit registry
  and wired into Bazel, lefthook, and CI.
- Coverage transition configuration is now governed by `tarpaulin.toml`; the transition runner
  may only invoke `cargo tarpaulin --workspace`.
- Verification guardrail execution metadata is now governed by
  `docs/architecture/verification-task-registry.v1.json`; the checker rejects unregistered
  Bazel guardrail targets, root guardrail suite labels, and `run_guardrail_task.sh` cases.
- Korean implementation marker and borrowed-brand enforcement is now covered by the forbidden
  marker guardrail: runtime `.css` files are scanned, `임시` is rejected, and implementation
  comments cannot reference borrowed `Claude.com` or `anthropic.com` design specs.

쉽게 말하면, 현재 프로젝트가 망가진 상태는 아닙니다. 이번 전수조사에서 발견된
검증/생성물 관리 빈틈은 guardrail로 막았고, 남은 일은 planned transition을 실제
Bazel evidence target으로 하나씩 끝내는 것입니다.

## 1. Scope

This audit checks the current Gongzzang repo before writing more architecture or implementation work.

Included:

- repo cleanliness and top-level structure
- Platform Core ownership boundary
- PNU-anchor / listing marker contract
- lakehouse registry integration
- traffic/auth policy registry
- Bazel transition ratchet state
- obvious legacy, marker, line-count, and secret-pattern risks

Not included:

- public data collection
- production deployment
- AWS provisioning
- full frontend/browser runtime verification
- full cargo workspace test run

No public-data collection was started.

## 2. Current Shape

Gongzzang is currently structured as a product repo, not a Catalog/raw-public-data repo.

Runtime/product ownership:

- `apps/web` owns the Next.js user-facing product surface.
- `services/api` owns the Rust API runtime.
- `services/outbox-publisher` owns Gongzzang-side lakehouse/media publishing integration
  and Platform Core lakehouse registry notifications.
- `crates/domain/core/{listing,listing-photo,user}` own product domain rules.
- `crates/domain/market/*` and `crates/domain/insights/*` own Gongzzang product-side market/insight semantics.
- `crates/db` owns Gongzzang persistence adapters.
- `crates/auth` owns Gongzzang auth integration and Platform Core service-token application.
Platform Core-owned responsibilities remain outside Gongzzang:

- canonical Catalog facts
- parcel/building/reference spatial layers
- PNU marker anchors
- public data raw lineage
- V-World/data.go.kr Catalog ingestion

Gongzzang consumes those through published contracts, events, and approved read-model artifacts.

Current collection status:

- Gongzzang has governed lakehouse asset registrations for `onbid_sale`, `court_auction`,
  listing marker tiles, listing marker serving indexes, and listing photo media.
- Gongzzang does not currently contain a live public-data nationwide collector.
- Catalog collection, including V-World/data.go.kr parcel/building/reference layers, remains
  Platform Core-owned.

## 3. Verified Gates

The following checks passed during this audit:

| Check | Result |
|---|---|
| `git status --short --branch --ahead-behind` | tracked code clean before this audit note update |
| `check-platform-core-boundary` | `platform-core-boundary-ok entries=46 contracts=5 gates=6 legacy_schema_allowances=11` |
| `check-platform-core-dependency-boundary` | `platform-core-dependency-boundary-ok manifests=26 allowances=0 source_allowances=0` |
| `check-pnu-anchor-pbf-marker-contract` | `pnu-anchor-pbf-marker-contract-ok files=60` |
| `check-traffic-auth-policy-registry` | `traffic-auth-policy-registry-ok routes=6 service_policies=2` |
| `check-platform-integration-policy` | `platform-integration-policy-ok components=10 route_surfaces=8` |
| `check-lakehouse-registry-integration` | `lakehouse-registry-integration-ok namespaces=1 assets=5 media_sets=1` |
| `check-bazel-transition-ratchet` | `bazel-transition-ratchet-ok targets=6 ci_refs=6` |
| `check-verification-control-plane` | `verification-control-plane-ok files=8 allowlisted=9` |
| `check-migration-version-prefixes` | `migration-version-prefixes-ok files=25` |
| `check-platform-core-event-receiver-contract` | `platform-core-event-receiver-contract-ok events=2 source_checked=True` |
| `check-forbidden-implementation-markers.sh` | passed |
| `file-line-limit.sh` | passed |
| `check-markdown-links.sh` | passed |
| `check-generated-artifact-registry` | `generated-artifact-registry-ok artifacts=2 sources=11` |
| `check-generated-artifact-registry tests` | `generated-artifact-registry-tests-ok` |
| `check-coverage-transition-ssot` | `coverage-transition-ssot-ok` |
| `check-coverage-transition-ssot tests` | `coverage-transition-ssot-tests-ok` |
| `check-verification-task-registry` | `verification-task-registry-ok tasks=32` |
| `check-verification-task-registry tests` | `verification-task-registry-tests-ok` |
| `check-forbidden-implementation-markers.tests.sh` | rejects English markers, mojibake, workflow markers, Korean `임시` markers in CSS, and borrowed external brand markers |
| `bash scripts/ci/run-bazel.sh test //:guardrails_all --config=ci --verbose_failures` | 32/32 passed |
| `bash scripts/ci/run-bazel.sh test //:workspace_typecheck //:workspace_hermetic_typechecks //:frontend_unit_test --config=ci --verbose_failures` | 5/5 passed |
| `bash scripts/ci/run-bazel.sh test //:rust_format_verification //:rust_check_verification //:rust_lint_verification --config=ci --verbose_failures` | 27/27 passed |
| `pnpm exec biome check .` | `Checked 260 files. No fixes applied.` |
| `git diff --check` | passed |

Focused test runners also pass after the prior guardrail decomposition work:

- `check-traffic-auth-policy-registry tests`
- `check-bazel-transition-ratchet tests`
- `check-platform-integration-policy tests`
- `check-pnu-anchor-pbf-marker-contract tests`
- `check-platform-core-boundary tests`

Previously expected failure, now resolved:

- `check-platform-core-boundary tests` previously failed with:
  `line count 812 exceeds 600`
- After the split it passes:
  `check-platform-core-boundary-tests-ok`

## 4. Important Non-Issues

### `crates/data-clients`

This directory exists, but currently contains only `README.md`.

It is not an active V-World/data.go.kr client layer. The README states that only Gongzzang-owned, non-Catalog external API anti-corruption adapters may live there after an ADR and boundary update.

Current status: acceptable placeholder.

### `services/etl-base-layer`

This service exists, but it is intentionally retained as a fail-closed handover stub.

It does not represent active Gongzzang ownership of static vector tile ETL. Its README and code state that legacy commands should exit with an ownership notice because Platform Core owns the lifecycle.

Current status: acceptable guard/stub, not an active ETL pipeline.

### Korean Text Encoding

Some files look broken when printed through Windows PowerShell `Get-Content`, but Node UTF-8 reads show the actual file contents are valid Korean. This is a console rendering issue, not file corruption.

### Runtime Catalog Endpoint Search

Focused runtime searches across `apps/`, `services/`, `crates/`, and `packages/` did not find
direct `api.vworld.kr` or `apis.data.go.kr` endpoint usage.

The remaining V-World/data.go.kr mentions are documentation, historical ADR/spec material,
Platform Core boundary policy tokens, or user-facing source links. That matches the current
ownership rule: Gongzzang consumes Platform Core contracts and must not own Catalog ingestion.

### Tracked File Size Shape

The largest tracked maintainability-relevant source files remain below the 1500-line hard gate.
The largest tracked text files are lockfiles or governed generated/compatibility artifacts:

- `pnpm-lock.yaml`
- `Cargo.lock`
- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `docs/architecture/platform-core-boundary.v1.json`
- `docs/architecture/verification-transition-ratchet.v1.json`
- `infrastructure/security/traffic-auth-edge-policy.generated.json`

Generated and aggregate artifacts are now required to be registered in
`docs/architecture/generated-artifacts.v1.json` and verified by guardrails.

## 5. Actual Gaps

### Gap 1: Architecture Docs Were Not Fully Filled

`docs/architecture/README.md` listed these documents as missing at the start of this audit:

- `data-flow.md`
- `layers.md`
- `mcp-vs-api.md`
- `geo-pipeline.md`
- `caching.md`
- `observability.md`

This is not a runtime bug, but it is a real SSS clarity gap. The architecture is partly enforced by JSON policies and guardrails, yet the human-readable architecture map is incomplete.

Follow-up status: resolved in this audit session by creating the six architecture documents and changing their index status to `Active`.

### Gap 2: `docs/backend/circuit-breaker.md` Was Referenced But Missing

`crates/data-clients/README.md` links to `docs/backend/circuit-breaker.md`, but that file does not exist.

This should be fixed before adding any new Gongzzang-owned external API adapter, because external calls require circuit breaker, retry, timeout, and audit rules.

Follow-up status: resolved in this audit session by creating `docs/backend/circuit-breaker.md`.

### Gap 3: Some Guardrail Files Are Still Large

Largest tracked text files after the audit follow-up split:

- `docs/architecture/traffic-auth-policy-registry.v1.json`: 1000 lines
- `infrastructure/security/traffic-auth-edge-policy.generated.json`: 510 lines
- `docs/architecture/platform-core-boundary.v1.json`: 527 lines

`traffic-auth-policy-registry.v1.json` is the single hand-edited SSOT for the
traffic/auth policy. (Update 2026-06-22: the former `00-*.json`..`80-*.json` split
fragments under `docs/architecture/traffic-auth-policy-registry/` were dead
duplicate copies that no code read; they were removed and the aggregate is now the
sole source. The earlier "aggregate generated from fragments" model and its
fragment-vs-aggregate checker no longer exist.)

The generated edge policy is produced by the traffic-auth policy generator (now
`cargo run -p api --bin generate-traffic-auth-policy`), so it is a generated
artifact rather than a source-maintainability priority.

Follow-up status: partially hardened after the audit. Generated and compatibility
aggregate artifacts are now registered in `docs/architecture/generated-artifacts.v1.json`.
The new `check-generated-artifact-registry` guardrail verifies generator, verifier,
source paths, artifact line budgets, source-fragment line budgets, and registration of
large generated JSON artifacts. The guardrail is wired into Bazel, lefthook, and CI.

The previous largest test runner, `scripts/ci/check-traffic-auth-policy-registry tests`, has been split into:

- a 189-line scenario runner;
- a 359-line shared helper;
- five fixture writers, each below 500 lines.

The runner now self-checks that it stays below 600 lines before executing the scenario matrix. No maintainability-relevant split file exceeds 500 lines in that test suite.

The traffic/auth checker itself has also been split into:

- a 27-line orchestrator;
- a 223-line shared helper module;
- a 185-line coverage helper;
- nine phase files, each below 200 lines.

The traffic/auth test runner now self-checks that the checker orchestrator and every checker module file stay below 600 lines.

The Bazel transition ratchet test runner has also been split into:

- a 320-line scenario runner;
- a 101-line shared helper;
- seven fixture files, each below 250 lines.

The Bazel transition ratchet test runner now self-checks that its helper and fixture files stay below 600 lines.

The platform integration policy test runner has also been split into:

- a 137-line scenario runner;
- a 67-line shared helper;
- six fixture files, each below 250 lines.

The platform integration policy test runner now self-checks that its helper and fixture files stay below 600 lines.

The platform integration policy checker itself has also been split into:

- a 22-line orchestrator;
- a 74-line shared helper;
- five phase files, each below 350 lines.

The platform integration policy test runner now self-checks that the checker orchestrator and every checker module file stay below 600 lines.

The Bazel transition ratchet checker itself has also been split into:

- a 13-line orchestrator;
- a 370-line shared helper;
- five phase files, each below 250 lines.

The Bazel transition ratchet test runner now self-checks that the checker orchestrator and every checker module file stay below 600 lines.

The PNU anchor PBF marker contract checker and test runner have also been split into:

- a 26-line checker orchestrator;
- five contract data files plus one validation phase, each below 250 lines;
- a 147-line test scenario runner;
- a 51-line test helper;
- six fixture files, each below 200 lines.

The PNU anchor PBF marker test runner now self-checks that the checker, checker modules, test helper, and test fixtures stay below 600 lines.

The Platform Core boundary checker and test runner have also been split into:

- a 12-line checker orchestrator;
- seven checker phase/shared files, each below 250 lines;
- a 315-line test scenario runner;
- a 41-line test helper;
- two fixture writers, each below 350 lines.

The Platform Core boundary test runner now self-checks that the checker, checker modules, test helper, and test fixtures stay below 600 lines.

The traffic/auth policy generator has also been split into:

- a 16-line generator orchestrator;
- eight generator phase/shared files, each below 200 lines.

The traffic/auth policy registry test runner now self-checks that the generator
orchestrator and every generator module file stay below 600 lines.

The traffic/auth policy registry SSOT has also been split into:

- a 54-line aggregate generator;
- nine source JSON fragments, each below 400 lines;
- a generated compatibility aggregate at `docs/architecture/traffic-auth-policy-registry.v1.json`.

The traffic/auth checker now compares the aggregate against the fragments, and
the traffic/auth test runner requires the fragment directory and enforces
<=600-line source fragments.

### Gap 4: Bazel Transition Ratchet Still Has Planned Exit Targets

The ratchet passes, but several transitions are intentionally still blocked because equivalent native Bazel evidence targets are planned, not implemented:

- dependency SCA evidence
- coverage evidence
- migration verification
- service e2e verification

This is structurally better than ad-hoc scripts, because the temporary state is explicit and guarded. It is not yet the final enterprise form.

Follow-up status: partially hardened after the audit. The ratchet now rejects an `exit_targets`
entry whose `state` is `available` unless that exact Bazel label exists in tracked `BUILD.bazel`
files. This does not retire the remaining transitions, but it prevents a planned exit target from
being reclassified as ready without a real Bazel target behind it.

Second follow-up status: further hardened after the audit. Each exit target now declares
`evidence_status` per required evidence item. This separates partial progress from completion:
for example, `dependency_sca_evidence` records Bazel-owned SBOM/evidence manifest coverage as
available through `//:verify_supply_chain`, while keeping pinned external advisory evidence
planned until explicit advisory collection approval exists.

Third follow-up status: guardrail transition targets are now tagged `no-cache` and `external`.
This prevents repo-scanning guardrails from reporting cached success after checker or policy
files change. The transition ratchet checker now also enforces those tags in
`GUARDRAIL_TRANSITION_TAGS`, so the cache-bypass contract is protected by tests instead of
remaining a convention.

Fourth follow-up status: coverage transition configuration is now guarded as SSOT. The
`coverage-tarpaulin` transition must read threshold, output formats, skip-clean behavior, and
exclude patterns from `tarpaulin.toml`; the runner may only invoke `cargo tarpaulin --workspace`.
The new coverage SSOT guardrail is wired into Bazel, lefthook, and CI.

Fifth follow-up status: guardrail execution metadata is now registry-driven. The
`verification-task-registry` lists 32 guardrail tasks and validates Bazel target definitions,
root guardrail suites, `run_guardrail_task.sh`, lefthook, and CI projections. The checker is
bidirectional for Bazel and runner projections: unregistered Bazel guardrail targets,
root guardrail suite labels, and runner cases are rejected.

Sixth follow-up status: planned Bazel exit evidence blockers are now structured. The transition
ratchet policy has a `planned_evidence_blocker_registry`, and every planned `evidence_status`
must declare non-empty registered `blocked_by` entries. Approval blockers must point at registered
approval gates covered by the exit target's `blocking_approval_gates`; implementation blockers must
point at the exact evidence requirement they block. This keeps the remaining planned transitions
from degrading into free-text promises.

Seventh follow-up status: transition approval gate decisions are now document-backed. ADR 0043
records the provisioning decisions for external advisory collection, browser runtime provisioning,
toolchain provisioning, database service provisioning, and service orchestration. The transition
ratchet checker now rejects approval gate `decision_reference` values unless they point to an
existing tracked file under `docs/`, preventing free-text decision drift.

Eighth follow-up status: exit target blocking gates are now evidence-backed. Every
`blocking_approval_gates` entry must be covered by at least one planned `evidence_status.blocked_by`
approval blocker. This forced migration and service e2e exit targets to declare the missing
toolchain/database provisioning evidence requirements explicitly instead of carrying ungrounded
blocking gates.

Ninth follow-up status: planned exit evidence targets are now registry-backed. The transition
ratchet policy has an `exit_evidence_target_registry` entry for every exit target and evidence
requirement pair. Each entry declares the owner, reason, and non-transition `planned_bazel_target`;
available evidence must match the registered target label. This turns the remaining transition
retirement work into explicit target-level evidence rather than implicit future intent.

### Gap 5: Internal Market Spatial Scope Naming Was Still BBox-Centric

Public listing marker routes are protected from `bbox`/`bounds` launch shapes by guardrails.

The audit initially found internal market-domain reader traits with `fetch_in_bbox` methods:

- `crates/domain/market/real-transaction/src/reader.rs`: `fetch_in_bbox`
- `crates/domain/market/court-auction/src/reader.rs`: `fetch_in_bbox`

Follow-up status: resolved after the audit by introducing `shared_kernel::spatial_scope`
and changing both market reader ports to `fetch_in_scope`. The new scope contract supports
`PNU`, administrative scopes, and validated tile coordinates without making `bbox` the product
query language.

### Gap 6: Korean Temporary and Borrowed-Brand Markers Were Not Fully Guarded

The audit found implementation-root comments that used Korean temporary wording, including
runtime design tokens and frontend boundary code. The existing forbidden marker guardrail blocked
English markers such as `TEMP`, `HACK`, and `XXX`, but it did not scan CSS files and did not reject
Korean `임시`. The audit also found UI token and primitive comments that referenced borrowed
external brand specs, which made Gongzzang's design system look derivative rather than owned.

Follow-up status: resolved in this audit session. The forbidden implementation marker guardrail now
scans `.css` files, rejects `임시` in implementation roots, and rejects borrowed `Claude.com` or
`anthropic.com` brand references in implementation comments. The guardrail test suite includes CSS
and TSX fixtures proving that those markers fail. Existing implementation comments were rewritten
to describe current intent instead of future cleanup or borrowed design provenance.

Related cleanup: typography token tracking values were set to `0`; unused legacy text-size aliases
were removed from `packages/ui/tokens/typography.css`; unused legacy shadow aliases were removed
from `packages/ui/tokens/spacing.css`.

## 6. Quality Assessment

Current Gongzzang quality is high in the areas that matter most for boundaries:

- Platform Core Catalog ownership is guarded.
- Direct Catalog client reintroduction is guarded.
- PNU-anchor listing marker regression is guarded.
- Traffic/auth route policy is registry-driven.
- Lakehouse integration is registry-driven.
- Bazel transition state is explicit rather than hidden.
- Planned Bazel exit targets now expose requirement-by-requirement evidence state, so partial
  evidence cannot be mistaken for transition retirement.
- Guardrail transition targets are uncached/external, so policy/checker edits rerun the
  guardrails instead of relying on stale Bazel test results.
- The uncached/external guardrail-tag contract is itself enforced by the Bazel transition ratchet.
- Coverage transition configuration is centralized in `tarpaulin.toml` instead of duplicated in
  shell runner flags.
- Guardrail execution metadata is centralized in `verification-task-registry.v1.json`, with
  bidirectional checks preventing unregistered Bazel/runner guardrail projections.
- English/Korean temporary implementation markers and borrowed external brand markers are blocked
  in runtime code and CSS token files.
- Typography token tracking is `0`, avoiding negative letter-spacing in Gongzzang UI surfaces.

It is not yet a complete SSS final form because:

- some Bazel transitions still rely on explicit transitional runners;
- database-backed migration verification, coverage evidence, dependency SCA evidence, and
  service e2e verification still need native Bazel evidence targets;
- production deployment, AWS provisioning, and public-data collection were intentionally not run.

## 7. Recommended Next Work

Recommended order:

1. Turn planned Bazel exit targets into native evidence targets one by one.
2. Keep generated and compatibility aggregate artifacts registered through
   `docs/architecture/generated-artifacts.v1.json` whenever a large generated JSON artifact is added.
3. Only after the verification-control plane is tightened, resume collection planning in the owning repo:
   Platform Core for Catalog/raw public data, Gongzzang for Gongzzang-owned market/listing assets.

## 8. Bottom Line

Gongzzang is not in a broken state.

The main product/Catalog boundary is structurally enforced and currently passes.
The next SSS-grade work should not be more public-data collection from this repo.
It should be Bazel exit-target retirement and explicit collection planning by ownership boundary,
while keeping Catalog ingestion in Platform Core.
