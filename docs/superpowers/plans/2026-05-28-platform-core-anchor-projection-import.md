# Platform Core Anchor Projection Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import Platform Core PNU anchor snapshot artifacts into Gongzzang's local read model with durable event idempotency and listing marker projection refresh.

**Architecture:** Platform Core still owns parcel geometry and anchor coordinates. Gongzzang stores only a read-model copy in `parcel_marker_anchor`, records inbound Platform Core events in a durable inbox, and refreshes Gongzzang-owned `listing_marker_projection` rows from listing semantics joined to the copied anchors. The public Next.js receiver validates Platform Core headers, then forwards accepted events to the Rust API internal route for durable storage; the artifact importer runs from the Rust API package against the same database.

**Tech Stack:** Next.js route handler, Rust Axum, SQLx/PostGIS, existing `reqwest`, existing workspace `sha2`, Vitest, Rust integration tests, repo guardrails (the `repo-guard` Rust binary and `scripts/lefthook/*.sh`; the former PowerShell guards were removed per ADR-0044).

---

## Approval Gate

Do not create the migration file in this plan until the user explicitly approves DB schema changes.

The 2026-05-28 DB approval handoff only approved
`migrations/30015_drop_platform_core_legacy_schema.sql`. It does not approve the
durable inbox/read-model import migration in this plan. Because `30015` is
already used, this plan reserves the next forward migration number,
`30016_platform_core_event_inbox_anchor_import.sql`, after explicit approval.
`scripts/ci/check-migration-version-prefixes` now guards the actual
`migrations/` directory against duplicate numeric prefixes before this plan can
land a new migration file.

The required DB schema changes are:

- widen `parcel_marker_anchor.algorithm_version` from `varchar(32)` to `varchar(128)`;
- create `platform_core_event_inbox` for durable event idempotency, traceability, retry state, and failure state.

No new external package version is required. If implementation uses `sha2` from the workspace in `services/api`, add it as a package dependency only because the workspace already pins the version.

## File Structure

- Create after DB approval: `migrations/30016_platform_core_event_inbox_anchor_import.sql`
  - Widens the existing anchor projection column.
  - Creates the event inbox table and indexes.
- Create: `crates/db/src/platform_core_anchor.rs`
  - Owns SQLx persistence for the inbox, anchor artifact row upsert, and affected listing projection refresh.
- Modify: `crates/db/src/lib.rs`
  - Exposes the new `platform_core_anchor` module.
- Test: `crates/db/tests/platform_core_anchor_import_integration.rs`
  - Proves inbox idempotency, long algorithm versions, anchor row upsert, and listing projection refresh.
- Create: `services/api/src/routes/platform_core_events.rs`
  - Owns `/internal/platform-core/events` and shared-secret validation.
- Modify: `services/api/src/main.rs`
  - Wires the internal Platform Core event route with DB state.
- Create: `services/api/src/bin/platform_core_anchor_import.rs`
  - Processes pending anchor snapshot events by fetching the immutable manifest and JSONL objects.
- Create: `services/api/src/platform_core_anchor_import.rs`
  - Parses manifests/entries, validates checksum and row counts, and calls `db::platform_core_anchor`.
- Modify: `services/api/Cargo.toml`
  - Adds existing workspace `sha2` if checksum code lives in `services/api`.
- Modify: `apps/web/app/platform-core/events/route.ts`
  - Keeps public validation and cache invalidation, forwards supported events to Rust internal API.
- Test: `apps/web/tests/unit/platform-core-events.test.ts`
  - Proves forwarding success, upstream failure retry behavior, and duplicate ack pass-through.
- Modify: `scripts/ci/check-platform-core-boundary`
  - Requires the durable inbox migration and Rust internal route.
- Modify: `scripts/ci/check-platform-core-boundary.tests`
  - Adds fixtures for the required inbox/importer paths.
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract`
  - Requires `algorithm_version varchar(128)` and the anchor import integration test.
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests`
  - Updates fixture expectations.

## Plan Parts

Detailed task bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - DB Migration Contract](./2026-05-28-platform-core-anchor-projection-import.part-01-db-migration-contract.md)
- [Part 02 - DB Repository Tests](./2026-05-28-platform-core-anchor-projection-import.part-02-db-repository-tests.md)
- [Part 03 - DB Repository Implementation](./2026-05-28-platform-core-anchor-projection-import.part-03-db-repository-implementation.md)
- [Part 04 - Event Route And Importer](./2026-05-28-platform-core-anchor-projection-import.part-04-event-route-and-importer.md)
- [Part 05 - Forwarding, Guardrails, And Verification](./2026-05-28-platform-core-anchor-projection-import.part-05-forwarding-guardrails-verification.md)
