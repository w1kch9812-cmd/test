# ADR 0037 - PNU Anchor PBF Marker Tiles

| Field | Value |
|---|---|
| Date | 2026-05-22 |
| Status | Accepted |
| Preceded by | [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md), [ADR 0018](./0018-pnu-first-identity-no-coordinates.md), [ADR 0036](./0036-static-vector-tile-runtime-contract.md) |
| Inherits/refines | `platform-core` [ADR 0008 - PNU Anchor PBF Marker Tile Contract](../../../platform-core/docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md) |

## Context

Gongzzang map runtime originally grew from a fast MVP shape:

- browser viewport changes call listing APIs with `bounds`;
- the backend queries listings by spatial envelope;
- the frontend creates per-listing map markers.

That shape can work for a small demo, but it is not the launch architecture for an SSS-grade
industrial real-estate platform. It makes viewport size part of backend load control, encourages
coordinate ownership inside product rows, and can accidentally hide records when marker limits are
introduced.

The product direction is now fixed:

- parcel identity is PNU-first;
- parcel polygons are served as PBF vector tiles;
- parcel-attached markers also use PBF vector tiles;
- marker location is resolved from a platform-core PNU anchor, not from arbitrary listing
  coordinates.

## Decision

Gongzzang launch map marker surfaces must use **PNU-anchor backed PBF marker tiles**.

Contract constants inherited from platform-core ADR 0008:

```text
marker_tile_response_format = MVT_PBF
marker_position_source = PNU_ANCHOR
bbox_marker_runtime_forbidden = true
dropped_marker_success_forbidden = true
```

The frontend may render large labels, compact badges, small dots, or aggregate symbols. That is
only presentation. The data contract must preserve all eligible records through one of these forms:

- one PBF point feature per record;
- truthful aggregation with `count` and a drill-down reference;
- zoom-dependent simplification that still represents the complete underlying set.

Gongzzang must not treat "there is no visual space for a label" as permission to omit the marker.

## Runtime Model

The launch runtime has three separate concerns:

| Concern | Owner | Format |
|---|---|---|
| Parcel polygon geometry | platform-core Catalog | static flat `.pbf` vector tiles through ADR 0036/0004 manifest |
| Marker anchor position | platform-core Catalog | PNU anchor registry |
| Gongzzang listing marker tiles | Gongzzang market domain | dynamic PBF generated from listing rows joined to platform-core anchors by PNU |
| Public/reference marker tiles | platform-core Catalog | real transaction, official land price, parcel-anchor, and other non-product reference layers |

The map joins visually through PNU:

1. Static parcel PBF gives the selectable parcel shape and `pnu` property.
2. Platform-core owns the PNU anchor coordinate.
3. Gongzzang listing marker PBF uses `listing.parcel_pnu` plus that anchor; it does not store or
   reinterpret coordinates.
4. Selecting a listing marker opens Gongzzang details by `id`, `pnu`, or `detail_ref`.
5. Selecting a parcel can query product data by PNU.

The marker tile PBF does not become a second source of parcel geometry. It only carries marker
points already resolved from the anchor registry.

Platform-core must not own Gongzzang listing price, status, exposure rules, search filters, or
detail payloads. It may expose anchor lookup/tile primitives and public/reference spatial layers.
Gongzzang remains the SSOT for listing semantics and any listing marker tile that represents those
semantics.

## API Shape

Recommended marker tile path:

```text
GET /map/v1/marker-tiles/{layer}/{z}/{x}/{y}.pbf?filter_hash={hash}
```

Stable initial `layer` candidates:

| Layer | Meaning | Freshness |
|---|---|---|
| `listing` | active Gongzzang listings, served by Gongzzang | dynamic |
| `real_transaction_price` | real transaction points, served by platform-core | semi-static batch |
| `auction` | court auction points, owner decided by source/domain ADR | semi-static batch |
| `official_land_price` | official land price indicators, served by platform-core | static or semi-static batch |

`filter_hash` is the identity of a validated server-side filter contract. It is not a raw SQL
fragment and not a free-form JSON expression.

Minimum feature properties:

| Property | Meaning |
|---|---|
| `id` | Listing id, transaction aggregate id, auction id, or aggregate id |
| `pnu` | Parcel identity |
| `kind` | Marker kind for style selection |
| `count` | Number of represented records |
| `rank` | Optional deterministic display priority |
| `detail_ref` | Opaque detail lookup reference |

The detail API may remain JSON because it is fetched after the user selects a feature. The map-wide
marker surface is PBF.

## Current Code Status

Legacy viewport-bounds list-query code has been retired from Gongzzang launch map/listing paths.
Frontend map marker placement is no longer allowed to use listing latitude/longitude or per-listing
Naver Marker objects; it must use the platform-core PNU-anchor PBF marker contract.

Known transitional areas:

- `apps/web/components/listings/listing-map.tsx` now registers the platform-core `parcel_anchor`
  marker PBF source/layer and the Gongzzang-owned `listing` marker PBF source/layer. It rejects
  legacy per-listing Naver marker placement and viewport `bounds` request wiring through CI.
- `crates/domain/core/listing/src/repository.rs` and `crates/db/src/listing.rs` no longer expose
  `find_markers_in_bbox` or the `ListingMarker` lightweight marker projection. Active listing saves
  are rejected when PNU anchors are missing. Guardrail wording: Active listing saves are rejected
  when PNU anchors are missing. `find_listing_marker_tile` keeps a defensive completeness check for
  stale projection gaps.
- `services/api/src/routes/listings.rs` no longer accepts public `bounds` query input.
- `services/api/src/routes/listing_marker_tiles.rs` exposes the Gongzzang listing PBF endpoint:
  `GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1`.
- `crates/db/src/listing.rs` exposes `find_card_summaries`, not a bbox-named card query.
- `Listing` no longer stores a product coordinate; the baseline migration no longer creates the
  former coordinate column or index.

No Gongzzang launch map/listing path may depend on viewport bounds as its public request shape.
Product-specific listing marker PBF tiles are a Gongzzang market-domain runtime surface, not a
platform-core service.

## Coordinate Ownership

For parcel-attached objects, Gongzzang does not own marker coordinates.

Allowed:

- store PNU on listing and market-domain records;
- render the marker at the platform-core anchor for that PNU;
- store non-canonical diagnostic coordinates only when clearly marked as diagnostic or source raw
  data;
- use JSON for selected-object details after a marker click.

Forbidden for launch marker placement:

- using `listing.geom_point`, listing `latitude`, listing `longitude`, or user-picked coordinates as
  the canonical marker position for parcel-attached listings;
- accepting public map marker requests by `bbox`, `bounds`, or raw coordinate envelopes;
- returning a successful marker response that silently drops eligible records;
- treating the PBF tile as the source of truth for the anchor instead of a projection of the
  platform-core anchor registry.

If Gongzzang later needs truly arbitrary user-drawn positions, that is a different object type and
requires a separate ADR. It must not weaken parcel-attached PNU marker semantics.

## Rendering Policy

Renderer implementations may choose Canvas, WebGL, Mapbox GL/Naver GL vector layers, bitmap stamps,
or another efficient renderer. The renderer choice does not change the data contract.

Display degradation order:

1. rich label marker when space and zoom permit;
2. compact badge marker;
3. dot marker;
4. truthful aggregate marker.

No step may drop the underlying record from the represented data set.

## Migration Sequence

1. Keep existing bbox paths only as transitional local behavior.
2. Define PBF marker tile contract and fixture tests.
3. Implement the first marker tile layer using a read-only layer such as real transaction or auction.
4. Add frontend PBF source/layer loading and canvas/vector rendering probes. **Done for the
   platform-core `parcel_anchor` marker layer and the Gongzzang `listing` marker layer.**
5. Move listing markers from bbox JSON to PBF marker tiles. **Done for the current all-active
   launch filter through Gongzzang-owned dynamic MVT/PBF tiles.**
6. Add CI guardrails that reject new launch marker code using `bounds`/`bbox` request shapes.
   **Done for frontend marker placement, frontend list-query `bounds` wiring, and the legacy
   listing marker/card repository paths.**
7. Deprecate legacy listing coordinate and bbox marker paths. **Done for `find_markers_in_bbox`;
   public `/listings` no longer accepts `bounds`.**
8. Remove legacy paths after PBF marker tiles pass desktop/mobile smoke checks.

The Gongzzang-local `parcel_marker_anchor` projection migration was approved by the user on
2026-05-22. Future schema changes still require explicit migration approval before generation.

## Consequences

Positive:

- backend load is bounded by tile identity instead of viewport area;
- marker location has one owner and one lineage path;
- PBF polygon tiles and PBF marker tiles share PNU as the join key;
- the frontend can render dense data as dots without data omission;
- the contract is reusable by Dawneer and future services.

Cost:

- platform-core must provide the anchor registry and public/reference marker tile contracts;
- Gongzzang must provide product-specific listing marker tiles when listing markers move beyond the
  current parcel-anchor layer;
- Gongzzang frontend marker code has been rewritten from `bounds` JSON to PBF tile consumption;
- legacy listing coordinate/bbox code must be retired after the new runtime proves itself;
- filter hashing and truthful aggregation require contract tests.

## Revisit Triggers

- a product marker is not parcel-attached and truly needs arbitrary coordinates;
- the PBF marker layer cannot represent a dense tile without truthful aggregation;
- platform-core anchor generation changes algorithm or geometry source;
- Naver GL integration blocks reliable vector point layer rendering and a renderer fallback is
  needed.

## References

- [ADR 0018 - PNU-first identity](./0018-pnu-first-identity-no-coordinates.md)
- [ADR 0021 - Static vector tile decomposition](./0021-static-vector-tile-decomposition.md)
- [ADR 0036 - Static vector tile runtime contract](./0036-static-vector-tile-runtime-contract.md)
- [platform-core ADR 0008 - PNU Anchor PBF Marker Tile Contract](../../../platform-core/docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md)
