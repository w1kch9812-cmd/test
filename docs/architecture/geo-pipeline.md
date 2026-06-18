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

Some internal market-domain reader ports still contain `fetch_in_bbox` methods.

Current status:

- this is not a public marker API violation;
- it should be revisited before implementing real transaction or auction readers;
- tile/PNU/admin-scope naming may be clearer if those readers become part of the map serving path.

## 6. Static Reference Tiles

Gongzzang does not own static vector tile ETL after Platform Core extraction.

`services/etl-base-layer` remains only as a fail-closed handover stub. It must not regain active source acquisition, build, promote, rollback, or R2 layout responsibility.

## 7. Guardrails

Relevant checks:

```powershell
./scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
./scripts/ci/check-platform-core-boundary.ps1
./scripts/ci/check-platform-core-dependency-boundary.ps1
```
