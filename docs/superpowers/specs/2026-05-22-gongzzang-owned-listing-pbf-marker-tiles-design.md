# Gongzzang-owned listing PBF marker tiles design

| Field | Value |
|---|---|
| Date | 2026-05-22 |
| Status | Accepted for implementation |
| Related ADRs | [ADR 0018](../../adr/0018-pnu-first-identity-no-coordinates.md), [ADR 0037](../../adr/0037-pnu-anchor-pbf-marker-tiles.md), [platform-core ADR 0008](../../../../platform-core/docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md) |

## 1. Objective

Gongzzang listing markers must move to a Gongzzang-owned MVT/PBF tile surface while keeping
marker location as a platform-core-owned PNU anchor.

This design keeps the service boundary clean:

- platform-core owns parcel geometry, parcel marker anchors, and public/reference spatial layers;
- Gongzzang owns listing price, status, exposure, search filters, details, and listing marker PBF
  tiles;
- PNU is the join key between those two worlds;
- no listing row owns a canonical latitude, longitude, or product coordinate.

In short: platform-core owns PNU anchors; Gongzzang owns listing semantics.

Terminology: `PBF` in this spec means an individual Mapbox Vector Tile response encoded with
Protocol Buffers. `PMTiles` is an archive/package format for storing many tiles together. This spec
is about the runtime listing marker tile contract, not about packaging tiles into a PMTiles archive.

## 2. Success Criteria

- No listing-owned canonical coordinate.
- No viewport-bounds public marker API.
- No silent marker drop.
- Listing map traffic is addressed by tile coordinate and typed filter identity, not viewport
  `bounds` or raw bbox parameters.
- Every listing marker position comes from a platform-core PNU anchor snapshot.
- Gongzzang listing semantics never move into platform-core.
- A successful tile response never silently drops eligible listing records.
- PBF marker tiles can render rich labels, compact badges, dots, or aggregates without changing the
  data ownership model.
- Listing card/detail JSON remains a post-click detail surface, not the map-wide marker surface.
- CI guardrails can detect regressions toward listing coordinates, bbox marker APIs, or
  platform-core listing ownership.

## 3. Non-Goals

- Moving listing business data into platform-core.
- Rebuilding static parcel polygon tiles.
- Implementing real transaction price, official land price, or auction tiles.
- Designing arbitrary freehand marker objects. Those need a separate ADR because they are not
  parcel-attached listings.

## 4. Ownership Boundary

| Fact | SSOT | Notes |
|---|---|---|
| Listing id, title, price, status, exposure | Gongzzang | Product semantics remain in the market domain. |
| Listing location identity | Gongzzang | Stored as `listing.parcel_pnu`. |
| Parcel polygon geometry | platform-core | Served as static or semi-static vector tiles. |
| Parcel marker anchor coordinate | platform-core | Derived from parcel geometry with lineage. |
| Listing marker PBF tile | Gongzzang | Generated from Gongzzang listings joined to platform-core anchors by PNU. |
| Public/reference marker PBF tile | platform-core | Examples: parcel anchor, official land price, real transaction price. |
| Selected listing details | Gongzzang | JSON detail API after marker/card selection. |

The local Gongzzang anchor read model, if added, is a cache/projection. It is not the source of
truth for marker location.

## 5. External API Shape

Gongzzang read path:

```text
GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash={filter_hash}
```

Gongzzang filter registration path:

```text
POST /map/v1/marker-filters/listing
```

The filter registration endpoint accepts a typed listing filter payload, validates it, normalizes it,
and returns:

```json
{
  "filter_hash": "lst_filter_...",
  "expires_at": "2026-05-22T12:00:00Z"
}
```

`filter_hash` is the cache identity of a validated filter contract. It is not raw SQL, not a
free-form JSON expression, and not a way to bypass typed listing filters.

Tile responses use:

```text
Content-Type: application/vnd.mapbox-vector-tile
```

An empty result is a valid empty MVT tile with HTTP 200, not a JSON body.

## 6. Tile Feature Contract

Minimum layer name inside the MVT tile:

```text
listing
```

Minimum point feature properties:

| Property | Meaning |
|---|---|
| `id` | Listing id or aggregate id. |
| `pnu` | Parcel identity when the feature represents one PNU. |
| `kind` | Stable style discriminator, initially `listing` or `listing_aggregate`. |
| `count` | Number of listing records represented by the feature. |
| `rank` | Deterministic display priority for label collision. |
| `detail_ref` | Opaque reference used to fetch cards or detail data. |

The tile must not include owner contact data, private notes, broker PII, or viewer-specific
bookmark state. Those belong in authenticated JSON detail/card APIs.

## 7. Data Flow

1. The browser registers or reuses a typed listing filter and receives `filter_hash`.
2. The map engine requests listing marker tiles from Gongzzang by `{z}/{x}/{y}` and `filter_hash`.
3. Gongzzang validates the filter hash and resolves it to a normalized filter contract.
4. Gongzzang selects active, visible listings from its own DB.
5. Gongzzang joins listing PNUs to a platform-core anchor snapshot.
6. Gongzzang encodes marker points or truthful aggregates as MVT/PBF.
7. The browser renders labels, badges, dots, or aggregates from the PBF layer.
8. Clicking a feature calls Gongzzang detail/card JSON by `detail_ref`, `id`, or `pnu`.

No production tile request should call the platform-core database directly. Cross-service data must
arrive through a typed API, outbox/consumer projection, or another explicit integration contract.

## 8. Anchor Read Model

For production, Gongzzang uses a local anchor read model replicated from platform-core, rather than
making a remote platform-core request for every tile.

Minimum logical fields:

| Field | Meaning |
|---|---|
| `pnu` | Parcel identity. |
| `anchor_point` | `geometry(Point, 4326)` anchor point. This is intentionally one column, not duplicated longitude/latitude columns. |
| `algorithm` | Anchor algorithm name, such as `polylabel`. |
| `algorithm_version` | Stable version for reproducibility. |
| `anchor_snapshot_id` | Platform-core anchor snapshot identity. |
| `source_geometry_version` | Platform-core parcel geometry build/version. |
| `source_geometry_checksum_sha256` | Lineage checksum for the source geometry input. |
| `platform_core_updated_at` | Source update time. |
| `synced_at` | Gongzzang projection sync time. |

The implementation uses `parcel_marker_anchor` as the Gongzzang-local projection table. The
important rule is that the record is explicitly a projection of platform-core, not a competing
coordinate source.

The user approved creating this read model and its DB migration on 2026-05-22. Future schema
changes still require explicit DB migration approval before implementation.

## 9. Completeness Rule

A tile response may aggregate, but it must not lie.

Allowed success cases:

- one point feature per eligible listing;
- one point feature per PNU with `count` when several listings share the same parcel anchor;
- deterministic zoom-dependent aggregate features with `count` and drill-down `detail_ref`;
- dot-only rendering when labels do not fit.

Forbidden success cases:

- `LIMIT N` before representation, followed by HTTP 200;
- dropping lower-ranked listings because the marker label has no visual space;
- deriving listing marker position from product-owned latitude/longitude columns;
- accepting public launch marker requests by `bounds`, `bbox`, `south`, `west`, `north`, or `east`.

If a tile cannot be represented within configured byte or feature budgets, Gongzzang must either
return a truthful aggregate or a structured failure. It must not return a partial tile as success.

## 10. Error Handling

The tile endpoint returns PBF only for successful tile responses.

Structured errors use `application/problem+json`:

| Status | Case |
|---|---|
| 400 | Invalid tile coordinate, malformed filter hash, or unsupported layer path. |
| 401/403 | Viewer lacks access to a non-public listing marker layer. |
| 404 | Filter hash does not exist. |
| 410 | Filter hash expired. |
| 422 | Tile cannot be represented truthfully within configured budgets. |
| 503 | Required anchor snapshot is unavailable or too stale for the layer policy. |

All errors must include a correlation id and must not expose SQL text, internal table names, PII, or
private listing data.

## 11. Caching

The cache key is:

```text
layer + z + x + y + filter_hash + anchor_snapshot_id + listing_visibility_watermark
```

Initial cache policy:

- public active listing tiles: short `public` cache with `ETag`;
- authenticated or viewer-specific tiles: `private` cache or no shared cache;
- expired filter hashes: no tile cache reuse;
- anchor snapshot changes: invalidate or change `anchor_snapshot_id`.

The marker tile must not contain personalized fields. Personalized state belongs in detail/card
JSON so shared tile caching remains possible.

## 12. Observability

Required span or metric dimensions:

- `layer`
- `z`
- `x`
- `y`
- `filter_hash`
- `anchor_snapshot_id`
- `eligible_count`
- `represented_count`
- `feature_count`
- `aggregate_count`
- `tile_byte_size`
- `db_query_ms`
- `mvt_encode_ms`
- `cache_hit`
- `anchor_snapshot_lag_seconds`

The invariant is:

```text
represented_count == eligible_count
```

when the response is HTTP 200.

## 13. Frontend Runtime

The frontend treats the PBF source as the map marker data plane.

Rendering degradation order:

1. rich listing label;
2. compact listing badge;
3. dot marker;
4. truthful aggregate marker.

The renderer choice is separate from the data contract. Naver GL, Canvas, WebGL, or another renderer
may be used, but it must consume the same PBF feature contract and must not recreate marker
positions from listing rows.

## 14. Tests And Guardrails

Implementation must add tests before or alongside code:

- filter normalization and hash determinism tests;
- tile query tests proving no eligible listing is lost;
- aggregate tests proving `count` and `detail_ref` preserve drill-down;
- API tests for content type, cache headers, and problem+json errors;
- integration tests with multiple listings on one PNU and multiple PNUs in one tile;
- frontend tests proving map marker loading no longer depends on listing latitude/longitude or
  viewport bounds;
- guardrails rejecting launch marker paths that reintroduce `bbox`, `bounds`, listing coordinates,
  or platform-core listing ownership wording.

The existing ADR guardrails must be extended rather than replaced.

## 15. Rollout Sequence

1. Review and approve this design spec.
2. Write an implementation plan with explicit file ownership and verification commands.
3. Define the marker filter contract and hash type.
4. Add the Gongzzang anchor read model only after DB migration approval.
5. Implement the listing marker tile query and PBF encoder.
6. Add the Gongzzang listing marker tile route.
7. Switch frontend listing marker rendering to the Gongzzang `listing` PBF layer.
8. Add local smoke checks for desktop/mobile map rendering.
9. Re-run guardrails, Rust checks, SQLx checks, frontend checks, and migration smoke tests.

## 16. Open Decisions For Implementation Planning

These are not product blockers, but the implementation plan must choose them explicitly:

- exact anchor projection table name in Gongzzang;
- whether filter hashes are stored in PostgreSQL, Redis, or deterministic signed tokens;
- initial tile byte budget and aggregate bucket policy by zoom;
- whether the first implementation exposes only public active listings or also authenticated layers.
