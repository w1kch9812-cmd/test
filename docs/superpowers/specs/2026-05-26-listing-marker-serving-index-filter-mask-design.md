# Listing Marker Serving Index And Filter Mask Design

| Field | Value |
|---|---|
| Date | 2026-05-26 |
| Status | Design accepted for planning |
| Related ADRs | [ADR 0017](../../adr/0017-listing-marker-render-canvas-bitmap-stamp.md), [ADR 0018](../../adr/0018-pnu-first-identity-no-coordinates.md), [ADR 0037](../../adr/0037-pnu-anchor-pbf-marker-tiles.md), [ADR 0038](../../adr/0038-listing-marker-serving-index-filter-mask.md) |
| Refines | [2026-05-22 listing PBF marker tiles design](./2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md) |

## 1. Objective

Design the scalable serving model for Gongzzang listing markers and advanced filters.

The product needs a map that can support:

- immediate changes for common filters such as asset type, deal type, price, and area;
- advanced industrial filters like crane capacity, dock, floor height, power, water, waste-water,
  industry code, usage area, land category, auction court, and auction date;
- exact nationwide and region counts;
- fast listing publish/update/withdraw reflection;
- platform-core PNU anchor ownership;
- no marker omission hidden behind visual collision or tile budgets.

This design does not replace ADR 0037. It defines the serving/index layer below that PBF marker tile
contract.

## 2. Short Version

Final runtime shape:

```text
base marker tile
+ browser instant filter
+ server marker/filter index
+ optional filter mask
+ platform-core PNU anchor
```

The listing OLTP model is not the map serving model. Gongzzang builds `listing_marker_projection`
and a filter index from listing events, joins PNU anchors from platform-core projections, and serves map
requests from that read model.

## 3. Scope

In scope:

- listing marker projection shape;
- filter normalization role;
- browser-side instant filter policy;
- server-side index policy;
- optional filter mask contract;
- layer separation;
- publish/update freshness;
- verification gates.

Out of scope:

- implementing the projection tables or migrations;
- final route names;
- choosing a specific external search engine;
- moving auction or real transaction ownership;
- changing platform-core parcel polygon tile generation.

## 4. Layer Model

The map is visually composed but logically separated.

| Layer | Owner | Serving style | Freshness |
|---|---|---|---|
| Parcel polygons | platform-core | static vector tile | batch/versioned |
| Building polygons | platform-core | static or batch vector tile | batch/versioned |
| Industrial complex polygons/markers | platform-core | static or batch vector tile | batch/versioned |
| Listing markers | Gongzzang | marker projection/index, dynamic tile/mask | near-real-time |
| Auction markers | Owner remains undecided until a dedicated auction ADR is accepted | separate marker layer | batch or near-real-time |
| Real transaction markers | platform-core or data-domain reference layer | separate marker layer | batch |
| Regulation/usage layers | platform-core | static or batch vector tile | batch/versioned |

Do not merge all marker domains into one universal marker tile. Ownership, filtering, permissions,
and invalidation differ by layer.

## 5. Data Ownership

| Data | SSOT |
|---|---|
| Listing id, price, area, status, deal type, exposure | Gongzzang |
| Listing location identity | Gongzzang `listing.parcel_pnu` |
| PNU anchor coordinate | platform-core |
| Anchor lineage/version | platform-core |
| Listing marker projection | Gongzzang projection of listing + anchor snapshot |
| Listing marker display payload | Gongzzang serving projection |
| Selected listing detail | Gongzzang JSON detail/card API |

The marker serving payload may include `anchor_lng` and `anchor_lat`, but those fields are derived
copies from the platform-core PNU anchor snapshot. They are not listing-owned coordinates.

## 6. Marker Projection

The marker projection is a read model optimized for map serving.

Minimum logical fields:

| Field | Purpose |
|---|---|
| `marker_id` | Stable marker feature identity within Gongzzang. |
| `listing_id` | Source listing id. |
| `pnu` | PNU join key. |
| `anchor_lng`, `anchor_lat` | Serving copy of platform-core anchor. |
| `anchor_snapshot_id` | Anchor version used for this projection row. |
| `source_geometry_version` | Platform-core geometry lineage. |
| `projection_version` | Gongzzang marker projection build/event version. |
| `tile_id_z*` | Precomputed tile or spatial cell keys for supported zoom tiers. |
| `asset_type` | Factory, warehouse, land, or other allowed product type. |
| `deal_type` | Sale, jeonse, monthly rent, etc. |
| `price_krw` | Canonical numeric price. |
| `area_sqm` / `area_pyeong` | Canonical comparable area fields. |
| `public_flags` | Safe flags for browser instant filter. |
| `visibility_scope` | Public/private/auth scope. |
| `status` | Published, hidden, withdrawn, etc. |
| `updated_at` | Source update timestamp. |

The projection may be PostgreSQL-backed at launch. The architecture must keep the boundary clean so
the same logical index can move to a columnar/search engine later if traffic demands it.

## 7. Filter Contract

Filters are typed and normalized before they reach serving indexes.

Normalization rules:

- sort enum arrays;
- remove default/no-op values;
- convert all money to KRW;
- convert all area to canonical units;
- normalize open ranges as explicit `null` min/max;
- normalize asset namespace semantics explicitly;
- include auth scope and visibility scope in the server-side identity;
- produce a stable `filter_hash`.

Example logical shape:

```json
{
  "asset_namespaces": {
    "factory": {
      "deal_types": ["sale"],
      "price_krw": { "min": null, "max": 5000000000 },
      "area_sqm": { "min": 990, "max": null },
      "has_crane": true
    },
    "warehouse": {
      "deal_types": ["monthly_rent"],
      "monthly_rent_krw": { "min": null, "max": 3000000 }
    }
  },
  "combine": "or"
}
```

The hash is not a cache-only concept. It is also the audit, metrics, request coalescing, and test
identity for a filter contract.

## 8. Client And Server Responsibilities

### Client

The browser owns immediate visual response for already-loaded data.

Client-side instant filters:

- layer visibility toggles;
- asset type;
- deal type;
- visible-tile price ranges when price is present;
- visible-tile area ranges when area is present;
- simple safe public flags included in the base marker tile;
- display-only modes like label density, marker size, color mode, and unit display.

The client must not:

- fetch nationwide marker data for filtering;
- compute canonical PNU anchors from polygon display tiles;
- treat marker coordinates as listing-owned truth;
- infer exact nationwide count from visible tiles;
- keep stale server masks after projection or anchor versions change.

### Server

The server owns correctness and full-corpus evaluation.

Server-side indexed filters:

- nationwide count;
- region count;
- unseen map tiles;
- permission/private visibility;
- complex OR namespace evaluation;
- high-cardinality numeric range filters across the corpus;
- auction dates/courts if auction layer adopts this model;
- advanced industrial attributes not present in the base tile;
- exact filtered result after modal filter application.

The server must use marker projection/index reads, not listing OLTP scans, for map serving.

## 9. UX Policy

Use two interaction modes:

| Surface | Behavior |
|---|---|
| Fast filter bar | Immediate apply. Includes asset type, deal type, price, area, layer toggles. |
| Advanced filter modal | Draft state inside modal; apply/result action commits to map. |

This prevents the server from processing every intermediate advanced-filter edit while keeping the
main map responsive.

Slider/input filters such as price and area still update the current viewport immediately when the
base marker tile has the needed values. Full-corpus count and unseen tiles update through server
index results.

## 10. Base Marker Tile

A base marker tile is the first marker payload for a layer/tile/version.

It should include only fields that are:

- required for rendering;
- safe to expose publicly for the layer;
- useful for instant client filtering;
- small enough for mobile map performance.

Initial candidate fields:

| Field | Reason |
|---|---|
| `marker_id` | Mask and selection identity. |
| `listing_id` or opaque `detail_ref` | Detail lookup. |
| `pnu` | Parcel identity and panel join. |
| `anchor_lng`, `anchor_lat` | Rendering position derived from platform-core anchor. |
| `anchor_snapshot_id` | Stale-mask rejection and lineage. |
| `asset_type` | Instant filter. |
| `deal_type` | Instant filter. |
| `price_krw` or bucket | Instant filter if safe. |
| `area_sqm` or bucket | Instant filter if safe. |
| `public_flags` | Instant filter for safe boolean flags. |
| `rank` | Deterministic rendering priority. |
| `count` | Aggregates or same-PNU grouped markers. |

Privacy-sensitive data, broker info, contact info, owner notes, viewer-specific bookmark state, and
private exposure rules must not be included in public base marker tiles.

## 11. Filter Mask

A filter mask is a compact response for an already-loaded base marker tile.

It answers:

```text
For this layer + z/x/y + base version + filter hash, which existing marker ids remain visible?
```

Possible encodings:

| Encoding | Best when |
|---|---|
| `show` marker id list | Few markers remain. |
| `hide` marker id list | Most markers remain. |
| compressed bitmap | Many markers and stable marker ordinal assignment. |

The mask key includes:

```text
layer
z
x
y
filter_hash
projection_version
anchor_snapshot_id
auth_scope
```

The client discards a mask when its base marker tile version does not match.

Filter masks are optional. The server can return a full filtered marker tile when it is simpler or
smaller.

## 12. Cache Policy

Cache is allowed but not primary.

Good cache candidates:

- static platform-core polygon/reference tiles;
- base marker tiles by layer/tile/version/public scope;
- low-cardinality selected filters;
- hot repeated masks/counts;
- immutable details like batch public reference layers.

Poor cache-primary candidates:

- arbitrary price ranges;
- arbitrary area ranges;
- arbitrary dates;
- continuous numeric sliders;
- viewer-specific permission filters.

Numeric filters should use browser comparisons for loaded tiles and server range indexes for
full-corpus answers. Cache can accelerate hot normalized requests, but it must not be the only plan.

## 13. Index Policy

The marker/filter index should support:

- spatial narrowing by tile id or hierarchical cell id;
- enum/boolean filtering by inverted or bitmap-like indexes;
- numeric filtering by range indexes;
- count/facet queries for nationwide and region summaries;
- fast invalidation by listing id and affected tile/cell ids;
- versioned reads so clients can reject stale masks.

Launch can implement the logical model in PostgreSQL when the data size is still moderate. The
boundary must not leak SQL-specific assumptions into the frontend contract.

## 14. Write Flow

Listing publish/update/withdraw flow:

```text
1. Write listing OLTP row.
2. Emit ListingPublished, ListingUpdated, or ListingWithdrawn event.
3. Projection consumer resolves PNU anchor from local platform-core anchor projection.
4. Update the affected marker projection row.
5. Update affected tile/cell index entries.
6. Update affected count/facet projections.
7. Advance projection_version or listing_visibility_watermark.
8. Invalidate only affected tile/mask cache keys.
```

Do not rebuild nationwide marker artifacts for a single listing change.

Freshness targets:

| Surface | Target |
|---|---|
| Creator form after save | immediate after write success |
| Creator map overlay | immediate optimistic or confirmed overlay |
| Public marker projection | 1-5 seconds |
| Filter/count projection | 1-10 seconds |
| Static polygon tiles | unaffected |

## 15. API Direction

Existing marker tile endpoint remains:

```text
GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash={filter_hash}
```

Candidate companion endpoints:

```text
POST /map/v1/marker-filters/listing
GET /map/v1/marker-masks/listing/{z}/{x}/{y}?filter_hash={filter_hash}&base_version={version}
GET /map/v1/marker-counts/listing?filter_hash={filter_hash}
```

Implementation planning must decide:

- exact route names;
- whether masks are separate endpoints or negotiated by `Accept`;
- whether filter hashes are stored server-side or signed deterministic tokens;
- mask encoding thresholds.

## 16. Error Handling

Successful marker tile responses are PBF. Successful mask/count responses may be JSON or a binary
encoding chosen during implementation planning.

Structured errors use `application/problem+json`.

Required error cases:

| Status | Case |
|---|---|
| 400 | Invalid tile coordinate, malformed filter, unsupported mask version. |
| 401/403 | Viewer cannot access the requested marker layer or auth scope. |
| 404 | Unknown filter hash or base marker version. |
| 409 | Mask requested for a stale base marker tile. |
| 410 | Filter hash expired. |
| 422 | Request cannot be represented truthfully within configured budgets. |
| 503 | Required projection or anchor snapshot is unavailable. |

## 17. Observability

Required metrics/spans:

- `layer`;
- `z`, `x`, `y`;
- `filter_hash`;
- `projection_version`;
- `anchor_snapshot_id`;
- `auth_scope`;
- `base_tile_cache_hit`;
- `mask_cache_hit`;
- `eligible_count`;
- `represented_count`;
- `visible_after_filter_count`;
- `tile_byte_size`;
- `mask_byte_size`;
- `index_query_ms`;
- `mvt_encode_ms`;
- `mask_encode_ms`;
- `projection_lag_seconds`;
- `client_mask_reject_count`.

Invariant for successful full tile responses:

```text
represented_count == eligible_count
```

Invariant for masks:

```text
mask_base_version == client_base_version
```

## 18. Guardrails

CI and review guardrails must reject:

- public marker APIs using `bbox`, `bounds`, `south`, `west`, `north`, or `east`;
- listing-owned canonical marker coordinates;
- marker serving code that scans listing OLTP rows directly instead of the projection/index;
- silent marker drop under tile byte pressure;
- precomputing arbitrary numeric filter combinations into static tile artifacts;
- platform-core taking ownership of Gongzzang listing semantics.

## 19. Implementation Planning Questions

The implementation plan must answer:

- Which projection table/index layout is used for launch?
- Which zoom levels get precomputed tile ids?
- Which fields are safe in public base marker tiles?
- Are price/area exposed as exact values or buckets in public tiles?
- What threshold chooses `show`, `hide`, or bitmap mask encoding?
- How does the frontend reconcile browser instant filter and server authoritative results?
- Which counts are exact synchronously and which may update with projection lag?
- What is the first supported advanced filter subset?

## 20. Verification

Required test families:

- filter normalization determinism;
- equivalent filter order produces the same hash;
- numeric ranges do not require cache keys to be correct;
- browser instant filter fixture tests for visible base tile data;
- server index tests for unseen tiles and nationwide count;
- mask stale-version rejection;
- listing publish/update/withdraw updates only affected projection/index entries;
- multiple listings on one PNU remain truthfully represented;
- guardrails for no bbox/bounds/listing-coordinate regressions;
- load smoke for dense marker tiles and mask payload size.

## 21. Rollout

Recommended sequence:

1. Accept ADR 0038 and this spec.
2. Write an implementation plan with concrete files and verification commands.
3. Add typed filter normalization and tests.
4. Add marker projection schema after explicit DB migration approval.
5. Add projection update flow for listing publish/update/withdraw.
6. Add base marker tile fields needed for instant filters.
7. Add server count/index endpoint.
8. Add optional filter mask endpoint after the base tile path is stable.
9. Wire frontend fast filters to browser instant filtering and server authoritative refresh.
10. Add advanced modal draft/apply flow.
