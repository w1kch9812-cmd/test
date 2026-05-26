# ADR 0038 - Listing Marker Serving Index And Filter Mask

| Field | Value |
|---|---|
| Date | 2026-05-26 |
| Status | Accepted |
| Preceded by | [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md), [ADR 0018](./0018-pnu-first-identity-no-coordinates.md), [ADR 0037](./0037-pnu-anchor-pbf-marker-tiles.md) |
| Refines | [Gongzzang-owned listing PBF marker tiles design](../superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md) |

## Context

ADR 0037 fixed the launch marker contract: parcel-attached listing markers are addressed by tile
coordinate, rendered from Gongzzang-owned marker PBF, and positioned only through platform-core PNU
anchors.

The next decision is how high-cardinality listing filters behave when the product exposes advanced
industrial real-estate filters similar to the design lab:

- asset type: factory, warehouse, land;
- deal type: sale, jeonse, monthly rent;
- numeric ranges: price, area, floor height, floor load, power, water, waste-water capacity;
- boolean and enum flags: crane, dock, drive-in, clean room, usage area, land category, court;
- namespace semantics: factory OR warehouse OR land OR auction.

Generating every filter combination as a separate tile set is not viable. Directly querying the
listing OLTP tables for every map pan, zoom, or slider movement is also not viable. Cache helps, but
cache cannot be the primary strategy for numeric input filters because price and area ranges have a
very high number of possible values.

## Decision

Gongzzang listing marker serving uses a dedicated read model and filter index. The launch table
names are `listing_marker_projection` and `listing_marker_filter_registry`:

```text
listing OLTP rows
-> listing marker projection
-> marker/filter index
-> base marker tile and optional filter mask APIs
-> browser instant filter and Canvas/GL renderer
```

The map runtime is a hybrid:

1. Static polygon/reference layers stay separate and are served through platform-core contracts.
2. Gongzzang listing markers are served from a Gongzzang-owned marker projection/index, not directly
   from listing OLTP rows.
3. The first marker payload for a visible tile is a base marker tile containing safe, minimal marker
   fields for instant browser-side filtering.
4. Simple visible-tile filters are applied immediately in the browser.
   Guardrail label: browser instant filtering.
5. Exact nationwide counts, unseen tiles, permission-sensitive results, and complex filters are
   handled by server-side indexes.
6. When a filter changes after a base marker tile is already present, the server may return a small
   filter mask instead of re-sending the full marker tile.
7. Cache is a secondary accelerator, not the correctness or scalability mechanism.

## Ownership

| Fact | Owner |
|---|---|
| Listing business data, price, area, status, exposure | Gongzzang |
| Listing location identity | Gongzzang, as PNU only |
| PNU anchor coordinate and lineage | platform-core |
| Listing marker projection and filter index | Gongzzang |
| Listing marker tile and filter mask APIs | Gongzzang |
| Parcel, building, industrial complex, and public/reference spatial layers | platform-core unless a later ADR says otherwise |

Listing rows must still not own canonical marker latitude/longitude. Marker serving payloads may
include anchor coordinates, but those values are copies derived from platform-core anchor snapshots
and must carry anchor lineage/version.

## Rejected Options

### A. Precompute every filter combination as marker tiles

Rejected.

This explodes for numeric range filters such as price and area, and it couples product filter UX to
tile artifact lifecycle. It also creates excessive invalidation when a single listing changes.

### B. Query listing OLTP tables directly for every map request

Rejected.

The write model should not be the high-throughput map serving model. This increases lock/query
pressure on listing tables and makes traffic spikes from map gestures compete with listing writes,
review, and back-office operations.

### C. Make all filters apply only after a modal "Apply" action

Rejected as the only interaction model.

Advanced modal filters may use a draft/apply flow, but fast map filters such as asset type, deal
type, price, and area must feel immediate on the current viewport. The system still coalesces and
cancels server work, but the client visual state changes immediately.

### D. Put all marker domains into one combined marker tile

Rejected.

Listing, auction, real transaction price, parcel anchor, official land price, and industrial complex
markers differ in ownership, freshness, permissions, filters, and invalidation. They must be
separate layers that compose visually on one map.

## Runtime Policy

### Layer Separation

The map may show many layers at once, but serving remains layer-specific:

```text
parcel polygon layer          -> platform-core static vector tile
building polygon layer        -> platform-core static or batch vector tile
industrial complex layer      -> platform-core static or batch vector tile
listing marker layer          -> Gongzzang dynamic marker projection/index
auction marker layer          -> source owner decided by auction ADR
real transaction marker layer -> platform-core or data-domain reference layer
```

### Filter Execution

Browser-side instant filters:

- asset type;
- deal type;
- visible-tile price and area ranges;
- safe public flags present in the base marker tile;
- purely visual toggles.

Server-side indexed filters:

- nationwide and region counts;
- unseen tiles;
- authorization and private visibility;
- complex namespace OR logic;
- high-cardinality numeric ranges across the full corpus;
- auction dates/courts;
- industrial attributes that are not present in the base marker tile;
- exact results after draft modal filters are applied.

### Filter Mask

A filter mask is an optional compact response for an already-loaded base marker tile. It identifies
which marker ids in that tile remain visible under a normalized filter contract.

Allowed shapes:

- show list, when few markers remain;
- hide list, when most markers remain;
- compressed bitmap, when the tile has many markers and stable marker ordinal assignment.

The mask must be keyed by:

```text
layer + z + x + y + filter_hash + marker_projection_version + anchor_snapshot_id + auth_scope
```

The mask is an optimization. A client must be able to fall back to requesting the full marker tile
for the same normalized filter when the mask is missing, stale, or unsupported.

### Numeric Filters

Numeric filters are not cache-first.

Price, area, floor height, floor load, power, water, and date ranges use:

- browser-side comparisons for the current base marker tile when the needed values are present;
- server-side range indexes for full-corpus counts and unseen tiles;
- optional cache only for common normalized buckets or hot repeated requests.

### Write Freshness

Listing marker freshness target:

| Surface | Target |
|---|---|
| Creator's immediate edit/register UI | synchronous local UI update after write success |
| Creator's map overlay | immediate optimistic or confirmed overlay |
| Public marker projection | 1-5 seconds after publish/update/withdraw event |
| Filter/count projection | 1-10 seconds, exact when refreshed |
| Static polygon tiles | not affected by listing writes |

Listing writes must invalidate or version only affected marker projection rows, tile ids, and filter
index entries. They must not rebuild nationwide marker artifacts.

## API Direction

Existing ADR 0037 tile path remains valid:

```text
GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash={filter_hash}
```

New companion surfaces are allowed:

```text
POST /map/v1/marker-filters/listing
GET /map/v1/marker-masks/listing/{z}/{x}/{y}?filter_hash={filter_hash}&base_version={version}
GET /map/v1/marker-counts/listing?filter_hash={filter_hash}
```

The names are directional, not final route commitments. Implementation planning must choose exact
routes and types.

`filter_hash` is derived from a typed, normalized filter contract. It is not raw JSON order, raw SQL,
or user-provided code.

## Guardrails

Implementation must enforce:

- no public launch marker request shape named `bbox`, `bounds`, `south`, `west`, `north`, or `east`;
- no listing-owned canonical latitude/longitude for parcel-attached listings;
- no successful tile/mask response that silently drops eligible records;
- no cross-service movement of listing price/status/exposure into platform-core;
- no static all-filter-combination tile generation;
- no map request path that reads directly from listing OLTP tables when a marker projection/index is
  required.

## Consequences

Positive:

- Map traffic is bounded by tile id, layer, filter identity, and read indexes.
- Price and area filters remain responsive without relying on unbounded cache keys.
- A listing write updates affected projection/index entries instead of rebuilding national tiles.
- Layer ownership remains clear as auction, real transaction, and listing markers evolve separately.
- The browser can feel immediate while the server remains authoritative for exact counts and
  unseen regions.

Cost:

- Gongzzang must maintain a marker projection and filter index in addition to listing OLTP rows.
- The frontend must reconcile base tiles, browser-side instant filters, server masks, and count
  results.
- Filter normalization becomes a contract surface and needs tests.
- Projection lag must be observable and handled in UX.

## Revisit Triggers

- Listing marker projection lag exceeds the public freshness target.
- Base marker tile payload becomes too large for acceptable mobile map performance.
- Filter mask complexity exceeds the cost of returning filtered marker tiles.
- Numeric filters require exact global counts faster than the chosen range index can support.
- Authorization rules become too viewer-specific for shared tile or mask caching.

## References

- [ADR 0017 - Listing marker rendering](./0017-listing-marker-render-canvas-bitmap-stamp.md)
- [ADR 0018 - PNU-first identity](./0018-pnu-first-identity-no-coordinates.md)
- [ADR 0037 - PNU Anchor PBF Marker Tiles](./0037-pnu-anchor-pbf-marker-tiles.md)
- [Listing PBF marker tiles design](../superpowers/specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md)
