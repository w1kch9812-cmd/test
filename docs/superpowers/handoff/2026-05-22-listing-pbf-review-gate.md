# Handoff - Listing PBF Review Gate

| Field | Value |
|---|---|
| Date | 2026-05-22 |
| Status | Implementation slice verified locally; full project completion not claimed |
| Current gate | Keep evidence current; do not treat this slice as whole-product launch completion |

## Current Decision

Gongzzang listing marker placement is PNU-first and PBF-based.

- platform-core owns parcel geometry, PNU marker anchors, and public/reference spatial layers.
- Gongzzang owns listing semantics and Gongzzang listing marker PBF tiles.
- Gongzzang listing rows must not own canonical marker coordinates.
- A successful marker tile must not silently drop eligible records.

## Current SSOT

- [ADR 0018 - Listing Identity Is PNU-First](../../adr/0018-pnu-first-identity-no-coordinates.md)
- [ADR 0037 - PNU Anchor PBF Marker Tiles](../../adr/0037-pnu-anchor-pbf-marker-tiles.md)
- [Gongzzang-owned listing PBF marker tiles design](../specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md)
- [platform-core ADR 0008](../../../../platform-core/docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md)

## Work Completed In This Gate

- Removed current-code references that could imply listing-owned marker coordinates.
- Added `docs/superpowers/README.md` to mark older Superpowers specs/plans as historical archive.
- Added current-gate warnings to `docs/superpowers/next-actions.md` and `docs/superpowers/roadmap.md`.
- Added Gongzzang ADR 0037 and platform-core ADR 0008 for the PNU-anchor PBF marker contract.
- Added a written design spec for Gongzzang-owned listing PBF marker tiles.
- Expanded Gongzzang guardrails so stale coordinate/bbox/listing-ownership direction is blocked.
- Expanded platform-core guardrails so platform-core cannot claim Gongzzang listing ownership.
- Verified Gongzzang listing PBF runtime, migration smoke, frontend source/layer tests, and
  platform-core local CORS behavior for the manifest/marker-contract endpoints.

## Approval Update

The user approved the written spec and the `parcel_marker_anchor` DB migration on 2026-05-22.
The former "do not implement yet" gate is closed. The implementation slice has local verification
evidence, but the broad active thread goal is still not a finite whole-product launch completion.

## Still Do Not Do

- Do not call platform-core databases directly from Gongzzang.
- Do not move listing price, status, exposure, search filters, or detail payloads into platform-core.

## Next Correct Step

If this slice is touched again, re-run the implementation verification checklist from
[`docs/superpowers/plans/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles.md`](../plans/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles.md):

- Rust DB integration tests for listing marker tiles;
- API compile and route tests;
- frontend map source/layer unit tests and panel codec tests;
- migration smoke checks;
- PNU-anchor PBF guardrail tests and repository guardrail;
- scoped `git diff --check`.

## Verification Evidence

Verification commands that passed for this gate (historical evidence). The
PNU-anchor PBF contract guard was a PowerShell `scripts/ci/check-pnu-anchor-pbf-marker-contract`
check that has since been removed (ADR-0044, PowerShell elimination); its
intent now lives in the Rust contract/repository tests run via `cargo test`:

```bash
pnpm markdownlint-cli2 docs/superpowers/roadmap.md docs/superpowers/next-actions.md docs/superpowers/README.md docs/superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md docs/adr/0037-pnu-anchor-pbf-marker-tiles.md
git diff --check -- docs/superpowers/roadmap.md docs/superpowers/next-actions.md docs/superpowers/README.md docs/superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md docs/adr/0037-pnu-anchor-pbf-marker-tiles.md crates/domain/core/shared-kernel/src/geometry.rs crates/db/tests/listing_integration.rs
cargo test -p platform-core-api
cargo fmt --check
cargo check -p platform-core-api
```

Live HTTP CORS smoke also passed on a temporary `platform-core-api` at `127.0.0.1:18082`: GET and
OPTIONS for `/catalog/v1/vector-tiles/manifest` and `/map/v1/marker-tiles/contract` returned
`access-control-allow-origin: http://localhost:3900`.

## Current Completion Status

This handoff is not the full project completion. It records the verified local implementation slice.

Completion audit:
[2026-05-22 active goal completion audit](./2026-05-22-active-goal-completion-audit.md).
