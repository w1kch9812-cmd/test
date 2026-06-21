# ADR-0045: ADR Placement & Cross-Repo Governance Home

| | |
|---|---|
| Date | 2026-06-20 |
| Status | Accepted |
| Decision owner | perfectoryinc (platform owner) |
| Related | ADR-0030 (three-service), ADR-0044 (first explicit cross-repo keystone under this rule), AGENTS.md §0.5 |

## Context

The platform has three sibling repositories — `gongzzang`, `platform-core`, `dawneer` — each with
its own `docs/adr/` series. Without an explicit rule, it is unclear where a given decision should be
recorded (e.g., "why is ADR-0044 in `gongzzang` when its content is mostly about `platform-core`?").
A de-facto convention already exists: cross-repo decisions live in `gongzzang` (three-service
architecture ADR-0030–0034; cross-repo Bazel ADR-0040, ADR-0042), and `gongzzang` AGENTS.md §0.5
designates "의사결정 SSOT = 이 repo의 ADR." This ADR makes that rule explicit.

## Decision

ADR placement is determined by the **scope of the decision**, not by which repo holds the most
implementation work:

- **Repo-scoped decision** (affects exactly one repo) → that repo's `docs/adr/`.
  - e.g., `platform-core` build internals → `platform-core/docs/adr/` (ADR-0010, 0011).
  - e.g., `dawneer`-only choices → `dawneer/docs/adr/`.
- **Cross-repo decision** (affects two or more repos) → **`gongzzang/docs/adr/`**, the designated
  cross-repo governance home. Each consuming repo receives a thin **pointer ADR** that says "adopts
  gongzzang ADR-NNNN" (e.g., `platform-core` ADR-0012 points to ADR-0044).

`gongzzang` is the cross-repo governance home **for now**. A dedicated governance/RFC repository is
intentionally NOT created at this scale.

## Alternatives

- **Consolidate all ADRs into `gongzzang`**: rejected — it destroys repo-local ownership and
  traceability for repo-scoped decisions.
- **Dedicated governance/`architecture`/`rfcs` repo for all cross-repo ADRs**: deferred — cleaner at
  large scale, but over-engineering for three repos and adds ceremony. Captured as a reassessment
  trigger below.

## Consequences

- Positive: unambiguous placement rule; no recurring "why is this ADR here?" confusion.
- Positive: cross-repo decisions have a single discoverable home; consuming repos stay first-class via
  pointer ADRs.
- Cost: `gongzzang` (a product repo) carries the dual role of product + cross-repo governance home —
  a mild conceptual awkwardness, accepted at the current scale.

## Reassessment Triggers

- Repo count grows beyond the current three, OR an org-governance need emerges that should not live in
  a product repo → create a dedicated governance/`architecture` repo and migrate cross-repo ADRs
  there (preserving numbers via a supersession/redirect note).

## References

- ADR-0030 (γ' three-service architecture).
- ADR-0044 (Bazel transition reconciliation — first explicit cross-repo keystone under this rule).
- `gongzzang` AGENTS.md §0.5 (cross-repo architecture / decision SSOT).
