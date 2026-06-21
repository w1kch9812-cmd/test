# Geo Pipeline

This document describes Gongzzang's current spatial data responsibilities.

## 1. Ownership Split

Platform Core owns:

- parcel geometry
- building/reference spatial layers
- PNU marker anchors
- public/reference vector tile lifecycle
- Catalog raw lineage

Gongzzang owns:

- listing semantics
- listing visibility/filtering
- listing marker projection/indexes
- listing-owned marker tile/count/mask/delta/tombstone serving

## 2. Current Marker Pipeline

```text
Platform Core PNU anchor snapshot/event
  -> Gongzzang platform_core_anchor projection
  -> listing marker projection
  -> marker serving index
  -> /map/v1/marker-* routes
  -> frontend map vector source
```

Important files:

- `migrations/30012_parcel_marker_anchor_projection.sql`
- `migrations/30013_listing_marker_projection.sql`
- `migrations/30014_listing_marker_filter_registry.sql`
- `migrations/30017_listing_marker_overlay_and_dirty_queue.sql`
- `crates/db/src/platform_core_anchor.rs`
- `crates/db/src/listing/marker_projection.rs`
- `crates/db/src/listing/marker_tile.rs`
- `services/api/src/listing_marker_serving`
- `apps/web/lib/map/marker-tile-contract.ts`

## 3. Public Marker Contract

Public marker routes use tile coordinates and stable filter identifiers.

They must not use:

- `bbox`
- `bounds`
- `south`
- `west`
- `north`
- `east`
- listing-owned canonical latitude/longitude columns

The reason is structural: map panning should load cacheable tile-shaped artifacts, and marker position should remain tied to Platform Core PNU anchors.

## 4. Listing Coordinates

Listing rows must not become the canonical owner of marker coordinates.

Allowed:

- PNU identity on listing/domain records
- derived marker projection based on Platform Core anchor data
- overlay/delta/tombstone indexes for serving freshness

Forbidden:

- `listing.latitude`
- `listing.longitude`
- product-owned `geom_point` as canonical marker source

## 5. Internal Spatial Queries

Internal market-domain reader ports use `shared_kernel::spatial_scope::SpatialScope`.

Supported scope shapes:

- `PNU`
- `Sido`
- `Sigungu`
- `Eupmyeondong`
- validated slippy-map tile coordinates

The goal is to keep product-side query intent explicit without reintroducing public
`bbox`/`bounds` marker request shapes. `BoundingBox` may still exist as a low-level geometry value
object, but market reader contracts should prefer `SpatialScope` unless a future ADR approves a
different contract.

## 6. Static Reference Tiles

Gongzzang does not own static vector tile ETL after Platform Core extraction.

`services/etl-base-layer` remains only as a fail-closed handover stub. It must not regain active source acquisition, build, promote, rollback, or R2 layout responsibility.

## 7. Guardrails

The PNU-anchor PBF marker contract and the Platform Core (dependency) boundary
must stay intact. The Platform Core catalog boundary is enforced by
`scripts/lefthook/catalog-m1-boundary.sh` and the boundary contract
`docs/architecture/platform-core-boundary.v1.json`.
