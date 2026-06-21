# Platform Core anchor inbox DB schema approval request

Date: 2026-05-29

## Status

Approved and implemented. This document was originally created as an approval
request only. Do not create or apply any follow-up DB schema migration from this
document; a new approval record is required for further schema changes.

Historical gate text retained for CI traceability: This is an approval request only.
Do not create or apply the migration until the user explicitly approves this DB
schema change.

The previous 2026-05-28 DB approval covered only
`migrations/30015_drop_platform_core_legacy_schema.sql`. It does not cover this
anchor inbox/import schema.

## Requested migration

Reserved migration:
`migrations/30016_platform_core_event_inbox_anchor_import.sql`

Requested changes:

- Widen `parcel_marker_anchor.algorithm_version` from `varchar(32)` to
  `varchar(128)`.
- Create `platform_core_event_inbox` as Gongzzang's durable inbox for Platform
  Core webhook events.
- Add pending-event and anchor-snapshot indexes for import/retry operations.

## Why this is needed

The current public `/platform-core/events` receiver can validate, acknowledge,
deduplicate, and dead-letter events at the web edge. That is not enough for a
durable anchor import pipeline because the accepted anchor snapshot event must
survive process restarts and be replayable by the Rust importer.

The proposed table is not Platform Core canonical storage. It is a Gongzzang
read-model inbox for event idempotency, auditability, retry state, and failure
inspection.

## Boundary

Allowed:

- Store Platform Core event metadata and payload for Gongzzang-side import.
- Store local read-model copy state needed to refresh
  `listing_marker_projection`.
- Join Gongzzang listings to imported PNU anchors by PNU.

Forbidden:

- Direct reads from the Platform Core database.
- Local canonical parcel geometry ownership.
- Listing-owned canonical latitude, longitude, or geometry columns.
- Silent anchor import failure without an inspectable inbox status.

## Proposed SQL

```sql
-- Durable inbound Platform Core event inbox and anchor importer compatibility.
--
-- `parcel_marker_anchor` remains a Gongzzang-local read model copied from
-- Platform Core. The inbox records Platform Core webhook events by event id so
-- replays are idempotent and import failures are inspectable.

alter table parcel_marker_anchor
    alter column algorithm_version type varchar(128);

create table platform_core_event_inbox (
    event_id uuid primary key,
    event_type varchar(128) not null,
    scope varchar(32) not null,
    effect varchar(64) not null,
    status varchar(32) not null,
    payload jsonb not null,
    anchor_snapshot_id varchar(128),
    source_geometry_version varchar(128),
    received_at timestamptz not null default now(),
    processed_at timestamptz,
    failed_at timestamptz,
    failure_reason text,
    constraint platform_core_event_inbox_scope_chk
        check (scope = 'catalog'),
    constraint platform_core_event_inbox_status_chk
        check (status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')),
    constraint platform_core_event_inbox_effect_chk
        check (effect in ('invalidate_catalog_cache', 'enqueue_anchor_projection_import')),
    constraint platform_core_event_inbox_anchor_payload_chk
        check (
            event_type <> 'catalog.parcel_marker_anchor.snapshot.published.v1'
            or (
                anchor_snapshot_id is not null
                and source_geometry_version is not null
                and effect = 'enqueue_anchor_projection_import'
            )
        )
);

create index platform_core_event_inbox_pending_idx
    on platform_core_event_inbox(event_type, received_at)
    where status = 'pending_import';

create index platform_core_event_inbox_anchor_snapshot_idx
    on platform_core_event_inbox(anchor_snapshot_id)
    where anchor_snapshot_id is not null;
```

## Verification after approval

After approval and implementation, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-migration-version-prefixes -Root .
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract -Root .
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary -Root .
cargo test -p db --features integration --test platform_core_anchor_import_integration
cargo test -p api platform_core_events
cargo test -p api platform_core_anchor_import
```

## Approval statement needed

Required user approval wording:

`30016 Platform Core anchor inbox/import DB schema migration creation is approved.`

## Approval record

Approval status: approved

Approved migration: `migrations/30016_platform_core_event_inbox_anchor_import.sql`

Approved statement:
`30016 Platform Core anchor inbox/import DB schema migration creation is approved.`

Approval source: user approved all remaining work in this session on
2026-05-29 with "승인할게 전부".
