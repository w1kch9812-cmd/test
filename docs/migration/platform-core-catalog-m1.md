# Platform-Core Catalog Extraction M1

Updated: 2026-05-12

## Decision

Start the extraction inside `gongzzang` by enforcing the M1 boundary locally.
Claude Code sibling-repo trust is a tooling boundary, not an architecture boundary; the
architecture remains the three-service split from ADR 0030, ADR 0031, and ADR 0034.

## Why This Path

M1 says `gongzzang` is still the temporary catalog owner while `platform-core` prepares
shadow reads. The sound move is therefore to prevent new catalog ownership drift in this
repo: no new catalog write surfaces, no repository/writer ports in catalog domain crates,
and no catalog mutation HTTP routes.

Trusting a parent directory or making a parent monorepo session can be useful for later
cross-repo review, but it is not the first extraction step. A cargo workspace spanning
both sibling repos would weaken the chosen deployment boundary and would conflict with
the independent-service ownership model.

## Local M1 Guardrail

The guardrail is `scripts/lefthook/catalog-m1-boundary.sh`. It blocks:

- repository or writer modules in:
  - `crates/domain/core/industrial-complex`
  - `crates/domain/core/parcel`
  - `crates/domain/core/building`
  - `crates/domain/core/manufacturer`
- direct SQL writes to canonical catalog tables from `crates/db/src` or `services/api/src`
- catalog mutation HTTP routes under `/api/parcels`, `/api/buildings`,
  `/api/industrial-complexes`, or `/api/manufacturers`

The guardrail is wired into `lefthook.yml` for pre-commit and pre-push, and into
`.github/workflows/ci.yml` as `Catalog M1 boundary`.

## Event Schema

`crates/domain/core/shared-kernel/src/catalog_event.rs` defines `CatalogEventV1` and
`CatalogEventKind`. The schema gives the M3 dual-write/outbox phase stable event types
such as `catalog.parcel.changed.v1` without adding any new writer in `gongzzang`.

## Next Cross-Repo Step

Run a separate trusted session rooted at `C:/Users/admin/Desktop/platform-core` for the
platform-core-side M1 work. That repo remains the sequencing SSOT for the migration plan:

```text
C:/Users/admin/Desktop/platform-core/docs/migration/2026-05-11-platform-core-extraction.md
```

Do not create a combined cargo workspace or submodule only to bypass an agent trust
boundary. If a multi-repo agent session is needed, open it at `C:/Users/admin/Desktop`
with both repositories trusted, while preserving each repo's independent CI and release
pipeline.
