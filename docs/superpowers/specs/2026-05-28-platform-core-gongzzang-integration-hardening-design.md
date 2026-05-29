# Platform Core Gongzzang Integration Hardening Design

| Field | Value |
|---|---|
| Date | 2026-05-28 |
| Status | Design pending implementation planning |
| Scope | Gongzzang consumer boundary for Platform Core Catalog data |
| Related ADRs | ADR 0030, ADR 0031, ADR 0034, ADR 0036, ADR 0037, ADR 0038, platform-core ADR 0004, platform-core ADR 0008 |

## 1. Objective

Make the Gongzzang and Platform Core connection production-grade without blurring ownership.

The goal is not to move more code into either service. The goal is to make the service boundary
observable, contract-tested, replayable, and hard to misuse.

## 2. Current State

The high-level boundary is correct.

- Platform Core owns parcel geometry, PNU marker anchors, and public/reference spatial layers.
- Gongzzang owns listing semantics, listing search, listing filters, listing marker tiles, B2C
  users, bookmarks, notifications, and product-specific market behavior.
- Gongzzang already consumes the Platform Core vector tile manifest at runtime.
- Gongzzang already serves listing PBF marker tiles from its own same-origin API.
- Gongzzang already has a local `parcel_marker_anchor` read model and joins listings to it by PNU.
- Guardrails already reject launch marker regressions such as `bbox`/`bounds` marker APIs and
  listing-owned marker coordinates.

The weak point is not conceptual ownership. The weak point is cutover-grade integration evidence.

- The local anchor read model exists, but the Platform Core to Gongzzang anchor sync path is not
  explicit enough.
- The existing `/platform-core/events` receiver handles the industrial-complex gold pointer event,
  but not anchor snapshot/materialized anchor projection events.
- Live dual-service smoke evidence is not part of Gongzzang CI.
- Platform Core has local prelaunch evidence, but not deployed consumer receiver evidence.
- Some Catalog crates and ETL paths remain in Gongzzang as M3 migration assets.

## 3. Ownership Boundary

This hardening keeps a one-way product dependency:

```text
Gongzzang product runtime -> Platform Core published contracts
```

Allowed contracts:

- Platform Core HTTP APIs.
- Platform Core runtime vector tile manifest.
- Platform Core outbox webhook events.
- Platform Core immutable anchor snapshot artifacts addressed by a published manifest.
- Generated or checked DTO/schema fixtures.

Forbidden contracts:

- Gongzzang reading or writing the Platform Core database.
- Platform Core storing Gongzzang listing price, status, exposure, filter, bookmark, or detail
  semantics.
- Gongzzang treating anchor coordinates as listing-owned canonical coordinates.
- Gongzzang deriving marker coordinates from viewport bounds or browser-picked coordinates.
- Runtime listing tile requests calling Platform Core per tile.

## 4. Runtime Data Flow

### 4.1 Static and Reference Map Layers

Platform Core publishes vector tile artifacts and an active runtime manifest. Gongzzang reads:

```text
NEXT_PUBLIC_TILES_MANIFEST_URL
or
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL/catalog/v1/vector-tiles/manifest
```

Gongzzang registers Platform Core-owned layers from that manifest. Gongzzang does not mutate the
manifest and does not infer object key prefixes outside the manifest contract.

### 4.2 Listing Marker Tiles

Gongzzang listing marker tiles remain Gongzzang-owned:

```text
GET /api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=...
```

The tile source uses Gongzzang listing semantics joined to the local `parcel_marker_anchor` read
model. Successful tiles must truthfully represent all eligible active listings for the tile.

### 4.3 PNU Anchor Projection Sync

Platform Core publishes an immutable anchor snapshot artifact and emits a Catalog event after the
snapshot becomes active.

The event payload must identify:

- event id;
- event type;
- active anchor snapshot id;
- source geometry version;
- artifact manifest URL or object key resolved by Platform Core;
- artifact checksum;
- row count;
- published timestamp.

Gongzzang receives the event through `/platform-core/events`, validates the headers and body, and
records the event id for idempotency. A Gongzzang importer then fetches the immutable artifact,
validates checksum and row count, and upserts `parcel_marker_anchor`.

After upsert, Gongzzang reprojects only affected listings:

```text
anchor snapshot change -> parcel_marker_anchor upsert -> listing_marker_projection refresh
```

The importer must be replayable. Replaying the same event must not duplicate rows or regress to an
older anchor snapshot.

## 5. Event Receiver Policy

`/platform-core/events` remains the single Gongzzang receiver path for Platform Core outbox events.

The receiver must:

- require `x-platform-core-event-id`;
- require `x-platform-core-event-type`;
- require `x-platform-core-outbox-scope`;
- reject header/body mismatches;
- return an acknowledgement body with `event_id`, `effect`, and `status`;
- be idempotent by event id;
- dispatch by event type through an explicit registry;
- emit structured telemetry for accepted, duplicate, rejected, and failed events.

Initial supported event effects:

| Event type | Gongzzang effect |
|---|---|
| `catalog.industrial_complex.gold_pointer.published.v1` | invalidate catalog cache |
| `catalog.parcel_marker_anchor.snapshot.published.v1` | enqueue or execute anchor projection import |

## 6. Gongzzang Remainder Boundary

The rest of Gongzzang is acceptable if it stays in these lanes:

| Area | Status | Rule |
|---|---|---|
| Listing, listing photo, listing filters, listing marker PBF | Gongzzang-owned | Keep in Gongzzang |
| B2C user, bookmarks, notifications, search history | Gongzzang-owned | Keep in Gongzzang |
| Market and insight product behavior | Gongzzang-owned | Keep in Gongzzang |
| IndustrialComplex, Parcel, Building, Manufacturer crates | Transitional Catalog assets | Keep read/ETL changes aligned with ADR 0034 phase matrix |
| V-World and data.go.kr clients | Transitional Catalog ETL assets | Do not add new Gongzzang-owned write paths |
| Staff/workforce identity | Platform Core target owner | Do not merge with B2C user semantics |

The transitional Catalog crates are not a failure by themselves. They become a failure only if new
product work treats them as Gongzzang-owned canonical Catalog sources after cutover.

## 7. Observability

Required Gongzzang signals:

- `platform_core.event.accepted_total`;
- `platform_core.event.rejected_total`;
- `platform_core.event.duplicate_total`;
- `platform_core.anchor_import.started_total`;
- `platform_core.anchor_import.failed_total`;
- `platform_core.anchor_import.row_count`;
- `platform_core.anchor_import.lag_seconds`;
- `listing_marker_projection.anchor_snapshot_lag_seconds`;
- `listing_marker_tile.completeness_violation_total`;
- `platform_core.manifest.fetch_failed_total`.

Required dimensions:

- `event_type`;
- `scope`;
- `anchor_snapshot_id`;
- `source_geometry_version`;
- `effect`;
- `failure_reason`.

## 8. Verification Gates

The implementation is not complete until these checks pass:

1. Gongzzang unit tests prove receiver dispatch, header/body validation, idempotency, and anchor
   event acknowledgement.
2. Gongzzang DB tests prove anchor artifact rows upsert into `parcel_marker_anchor` and refresh
   affected `listing_marker_projection` rows.
3. Gongzzang PNU marker guardrail passes.
4. Platform Core contract tests prove the published anchor event fixture and receiver contract match.
5. A local dual-service smoke test proves Platform Core can publish an event to Gongzzang and receive
   the expected acknowledgement.
6. Live/deployed receiver E2E remains blocked unless operator-provided HTTPS endpoints and cutover
   evidence are present.

## 9. Rollout

Phase 1 closes the contract and receiver path.

- Add an explicit Gongzzang event dispatch registry.
- Keep the existing industrial-complex cache invalidation behavior.
- Add support for the anchor snapshot published event.
- Add idempotency storage or a clearly bounded local substitute if persistent storage is deferred.

Phase 2 closes anchor projection import.

- Define the anchor artifact input schema.
- Implement checksum and row-count validation.
- Upsert into `parcel_marker_anchor`.
- Refresh affected listing marker projections.

Phase 3 closes cross-repo smoke.

- Use the Platform Core webhook receiver E2E runner against Gongzzang local receiver.
- Store local evidence under `target/audit`.
- Keep deployed evidence blocked until real endpoints exist.

Phase 4 closes cutover readiness.

- Stabilize Platform Core dirty anchor/manifest work into a clean baseline.
- Make Gongzzang consume that baseline.
- Run both repos' focused contract and guardrail suites.

## 10. Non-Goals

- Do not move Gongzzang listing search, filters, or pricing rules into Platform Core.
- Do not remove all transitional Catalog crates from Gongzzang in this slice.
- Do not add national data collection execution from Gongzzang.
- Do not claim deployed production cutover from local evidence.
