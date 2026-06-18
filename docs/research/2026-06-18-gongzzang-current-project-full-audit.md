# Gongzzang Current Project Full Audit

Date: 2026-06-18
Repo: `C:\Users\admin\Desktop\gongzzang`

## 0. Current Rescan Summary

Current rescan status:

- The repo is not clean: the current session has staged architecture and guardrail refactor work.
- There is no unstaged diff after the latest rescan.
- No public-data collection was started.
- The main product/Catalog ownership guardrails pass.
- The 1500-line hard gate passes.
- The prior red item was intentional maintainability ratcheting:
  `scripts/ci/check-platform-core-boundary.tests.ps1` now self-checks that the boundary
  checker and its modules stay <=600 lines.
- That red item is now resolved: the test runner is 315 lines, the checker
  orchestrator is 12 lines, and every boundary checker module is <=201 lines.

쉽게 말하면, 현재 프로젝트가 “망가진 상태”는 아닙니다. 실제 boundary checker들은 통과합니다.
방금까지 걸려 있던 빨간불은 기능 장애가 아니라 큰 guardrail 파일을 더 작게 쪼개라는 품질 기준이었고,
현재는 그 기준까지 통과합니다.

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
| `git status --short --branch` | staged architecture and guardrail refactor changes present |
| `check-platform-core-boundary.ps1` | `platform-core-boundary-ok entries=46 contracts=5 gates=6 legacy_schema_allowances=11` |
| `check-platform-core-dependency-boundary.ps1` | `platform-core-dependency-boundary-ok manifests=26 allowances=0 source_allowances=0` |
| `check-pnu-anchor-pbf-marker-contract.ps1` | `pnu-anchor-pbf-marker-contract-ok files=60` |
| `check-traffic-auth-policy-registry.ps1` | `traffic-auth-policy-registry-ok routes=6 service_policies=2` |
| `check-platform-integration-policy.ps1` | `platform-integration-policy-ok components=10 route_surfaces=8` |
| `check-lakehouse-registry-integration.ps1` | `lakehouse-registry-integration-ok namespaces=1 assets=5 media_sets=1` |
| `check-bazel-transition-ratchet.ps1` | `bazel-transition-ratchet-ok targets=6 ci_refs=6` |
| `check-verification-control-plane.ps1` | `verification-control-plane-ok files=8 allowlisted=9` |
| `check-migration-version-prefixes.ps1` | `migration-version-prefixes-ok files=25` |
| `check-platform-core-event-receiver-contract.ps1` | `platform-core-event-receiver-contract-ok events=2 source_checked=True` |
| `check-forbidden-implementation-markers.sh` | passed |
| `file-line-limit.sh` | passed |
| `check-markdown-links.sh` | passed |

Focused test runners also pass after the prior guardrail decomposition work:

- `check-traffic-auth-policy-registry.tests.ps1`
- `check-bazel-transition-ratchet.tests.ps1`
- `check-platform-integration-policy.tests.ps1`
- `check-pnu-anchor-pbf-marker-contract.tests.ps1`
- `check-platform-core-boundary.tests.ps1`

Previously expected failure, now resolved:

- `check-platform-core-boundary.tests.ps1` previously failed with:
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

`traffic-auth-policy-registry.v1.json` is now a compatibility aggregate generated
from smaller source fragments in `docs/architecture/traffic-auth-policy-registry/`.
Every source fragment is below 400 lines. The checker compares the aggregate
against the fragments to block drift.

The generated edge policy is produced by `generate-traffic-auth-policy.ps1`, so it
is a generated artifact rather than a source-maintainability priority.

The previous largest test runner, `scripts/ci/check-traffic-auth-policy-registry.tests.ps1`, has been split into:

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

### Gap 5: Internal Market Spatial Scope Naming Was Still BBox-Centric

Public listing marker routes are protected from `bbox`/`bounds` launch shapes by guardrails.

The audit initially found internal market-domain reader traits with `fetch_in_bbox` methods:

- `crates/domain/market/real-transaction/src/reader.rs`: `fetch_in_bbox`
- `crates/domain/market/court-auction/src/reader.rs`: `fetch_in_bbox`

Follow-up status: resolved after the audit by introducing `shared_kernel::spatial_scope`
and changing both market reader ports to `fetch_in_scope`. The new scope contract supports
`PNU`, administrative scopes, and validated tile coordinates without making `bbox` the product
query language.

## 6. Quality Assessment

Current Gongzzang quality is high in the areas that matter most for boundaries:

- Platform Core Catalog ownership is guarded.
- Direct Catalog client reintroduction is guarded.
- PNU-anchor listing marker regression is guarded.
- Traffic/auth route policy is registry-driven.
- Lakehouse integration is registry-driven.
- Bazel transition state is explicit rather than hidden.

It is not yet a complete SSS final form because:

- several generated or compatibility aggregate files are still large, though their source
  inputs are split and drift-checked;
- some Bazel transitions still rely on explicit transitional runners;
- full runtime/build verification was not rerun in this audit.

## 7. Recommended Next Work

Recommended order:

1. Turn planned Bazel exit targets into native evidence targets one by one.
2. Decide whether the remaining generated JSON artifacts need separate generated-file handling in line-count reporting.
3. Run a broader verification pass after docs and guardrail refactors.

## 8. Bottom Line

Gongzzang is not in a broken state.

The main product/Catalog boundary is structurally enforced and currently passes.
The next SSS-grade work should not be more public-data collection from this repo.
It should be Bazel exit-target retirement and broader runtime/build verification, while keeping
Catalog ingestion in Platform Core.
