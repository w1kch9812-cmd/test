# Platform Core Anchor Projection Import Plan - Part 01: DB Migration Contract

Parent index: [Platform Core Anchor Projection Import Implementation Plan](./2026-05-28-platform-core-anchor-projection-import.md).

## Task 1: DB Migration Contract

**Files:**
- Create after approval: `migrations/30016_platform_core_event_inbox_anchor_import.sql`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests`

- [ ] **Step 1: Write the failing guardrail test**

In `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests`, update the clean migration fixture for `migrations\30012_parcel_marker_anchor_projection.sql` so it contains:

```sql
algorithm_version varchar(128) not null
```

Add a required file fixture:

```powershell
Write-File -Root $Root -RelativePath "migrations\30016_platform_core_event_inbox_anchor_import.sql" -Content @'
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
        check (effect in ('invalidate_catalog_cache', 'enqueue_anchor_projection_import'))
);

create index platform_core_event_inbox_pending_idx
    on platform_core_event_inbox(event_type, received_at)
    where status = 'pending_import';
create index platform_core_event_inbox_anchor_snapshot_idx
    on platform_core_event_inbox(anchor_snapshot_id)
    where anchor_snapshot_id is not null;
'@
```

- [ ] **Step 2: Run the guardrail test and verify RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests
```

Expected: fail because `check-pnu-anchor-pbf-marker-contract` does not yet require migration `30015` or `algorithm_version varchar(128)`.

- [ ] **Step 3: Implement the guardrail requirement**

In `scripts/ci/check-pnu-anchor-pbf-marker-contract`, update the `migrations/30012_parcel_marker_anchor_projection.sql` tokens from:

```powershell
"anchor_snapshot_id",
```

to include:

```powershell
"algorithm_version varchar(128) not null",
"anchor_snapshot_id",
```

Add a new contract entry:

```powershell
[pscustomobject]@{
    RelativePath = "migrations/30016_platform_core_event_inbox_anchor_import.sql"
    Tokens = @(
        "alter table parcel_marker_anchor",
        "alter column algorithm_version type varchar(128)",
        "create table platform_core_event_inbox",
        "event_id uuid primary key",
        "payload jsonb not null",
        "status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')",
        "platform_core_event_inbox_pending_idx"
    )
}
```

- [ ] **Step 4: Create the approved migration**

After user DB approval, create `migrations/30016_platform_core_event_inbox_anchor_import.sql` with exactly this SQL:

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

- [ ] **Step 5: Run the guardrail test and verify GREEN**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests
```

Expected: `check-pnu-anchor-pbf-marker-contract-tests-ok`.
