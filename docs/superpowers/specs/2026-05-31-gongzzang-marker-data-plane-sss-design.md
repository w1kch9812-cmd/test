# Gongzzang Marker Data Plane SSS Design

| Field | Value |
|---|---|
| Date | 2026-05-31 |
| Status | Draft for review |
| Scope | Gongzzang listing marker serving, platform-core map/control-plane integration |
| Related | [ADR 0037](../../adr/0037-pnu-anchor-pbf-marker-tiles.md), [ADR 0038](../../adr/0038-listing-marker-serving-index-filter-mask.md), [Platform Core ADR 0008](../../../../platform-core/docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md) |

## 1. Executive Summary

Gongzzang should not build a separate map platform beside platform-core. platform-core already owns
the map control-plane pieces that should be reused: PNU anchors, vector tile manifest lifecycle,
artifact promotion/rollback, reference spatial layers, service identity, traffic budgets, outbox
events, readiness, and metrics.

The right SSS-level structure is:

```text
platform-core = Map Control Plane and Catalog/Workforce authority
gongzzang     = Listing Marker Data Plane and product semantics authority
```

This means platform-core defines and publishes the map substrate, while Gongzzang serves listing
marker data that uses that substrate. Listing price, status, exposure, filter, detail, private
visibility, and listing write freshness stay in Gongzzang.

The target is not just "add rate limits" or "cache harder". The structural target is:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

Base tiles handle scale, delta overlays handle fast writes, tombstones hide stale deleted/private
records immediately, and authorization always remains server-enforced for non-public data.

## 2. Current Platform Core Inventory

The checked platform-core workspace currently contains these first-class modules:

| Module | Current role | Integration implication |
|---|---|---|
| `crates/shared-kernel` | Shared IDs, PNU value object, event primitives, common errors | Gongzzang should reuse wire contracts, not reimplement PNU/event meaning differently. |
| `crates/api-types` | Published Catalog and Workforce DTOs plus contract tests | Gongzzang should pin to generated/published API contracts, not hand-copy drifting shapes. |
| `crates/catalog/catalog-domain` | Catalog facts, PNU anchor contract, vector tile manifest domain, spatial layers | Map substrate definitions belong here or are derived from here. |
| `crates/catalog/catalog-app` | Catalog use cases and ports for manifest promote/rollback, anchor rebuild, lakehouse audit | Gongzzang should mirror the promote/rollback pattern for listing tile artifacts if it adds static listing artifacts. |
| `crates/catalog/catalog-infra` | Postgres, external source clients, lakehouse/object-store infrastructure | Gongzzang must not import this or call platform-core DB directly. |
| `crates/workforce/*` | Staff identity/session/role integration with Zitadel | Admin operations such as manifest promote or anchor rebuild are staff-authorized. |
| `crates/outbox-publisher` | Outbox publishing, webhook, object storage, vector tile manifest pointer support | Gongzzang should consume platform-core events through webhook/inbox only. |
| `services/api` | Catalog/Workforce HTTP API, `/health`, `/ready`, `/metrics`, service identity, traffic budgets | Gongzzang calls stable HTTP/API surfaces and uses metrics/readiness for operations. |
| `services/outbox-publisher` | Batch/data pipelines and artifact builders, including anchor artifact export and PBF artifact build | Platform-core is already the right place for national reference data artifact generation. |
| `migrations/*` | Catalog, Workforce, vector tile manifest, PostGIS mirror, PNU anchor registry | These tables are platform-core owned and are not Gongzzang runtime write targets. |

Important platform-core map features already exist:

| Capability | Evidence in platform-core | How Gongzzang should use it |
|---|---|---|
| PNU-anchor marker contract | `catalog-domain/src/marker_tile.rs`, ADR 0008 | Listing marker points must be resolved from PNU anchors. |
| Static vector tile manifest contract | `catalog-domain/src/vector_tile.rs`, `catalog.vector_tile_manifest` | Gongzzang web should consume public/reference layers through the manifest. |
| Manifest promote/rollback | `catalog-app` ports and API routes | Listing artifact publish should copy the same blue/green pointer model when introduced. |
| Anchor rebuild lineage | `ParcelMarkerAnchorRebuildCommand` and `parcel_marker_anchor` migration | Gongzzang local anchor rows are projections with snapshot lineage, not coordinate ownership. |
| z0-z11 aggregate, z12+ exact anchor policy | ADR 0008 and marker tile contract constants | Listing marker serving should use the same low-zoom aggregate principle. |
| DB marker tile reference gate | platform-core DB reference marker endpoint disabled by default in production | Gongzzang should not treat DB tile rendering as the final national hot path. |
| Traffic budget and overload protection | `services/api/src/traffic.rs` | Gongzzang should keep route-level budgets in SSOT policy and enforce them consistently. |
| Service identity | `services/api/src/routes/service_identity.rs`, Gongzzang service auth docs | Browser-visible public routes and internal service routes must stay separate. |
| Outbox and quarantine/DLQ | `catalog.outbox_event`, `catalog.outbox_quarantine` | Anchor snapshot events should refresh Gongzzang projections through an inbox/DLQ path. |
| Metrics/readiness | `/metrics`, `/ready`, HTTP duration/request metrics | Marker data plane must expose equivalent lag, freshness, tile error, and budget metrics. |

## 3. Ownership Boundary

### 3.1 Platform Core Owns

platform-core owns the map control-plane and shared substrate:

- PNU identity rules and anchor lineage.
- Parcel geometry and public/reference spatial layers.
- Vector tile manifest schema, artifact metadata, and active pointer lifecycle.
- Anchor snapshot/version events.
- Workforce/staff authorization for platform operations.
- Service-to-service identity contract and public API contract pins.
- Lakehouse/source lineage for public/reference data.

In plain language: platform-core owns the "official map baseboard". It says where each parcel's
anchor is and which immutable spatial artifacts are currently active.

### 3.2 Gongzzang Owns

Gongzzang owns listing semantics and listing marker serving:

- `listing` write model and listing lifecycle.
- Listing price, area, type, status, exposure, and private/verified visibility.
- Listing marker projection/index.
- Listing marker filter registry and normalized filter hash.
- Listing marker base tile, mask, count, delta, tombstone, and detail APIs.
- Product-specific authorization decisions for private or business-verified listing data.

In plain language: Gongzzang owns "what this marker means and who is allowed to see it".

### 3.3 Do Not Move

These must not be moved into platform-core:

- listing price/status/exposure/search-filter/detail payload;
- business-verified listing visibility rules;
- listing marker dirty-tile decisions based on listing writes;
- listing private overlay data;
- Gongzzang product ranking and listing presentation semantics.

These may be standardized through platform-core or a shared contract:

- marker tile feature minimum fields;
- tile manifest shape;
- anchor snapshot event schema;
- cache/version header policy for immutable artifacts;
- spatial tile/H3 addressing conventions;
- layer registry schema.

## 4. Target Architecture

```text
                 platform-core
    -------------------------------------------------
    Catalog source/lakehouse/PostGIS mirror
       -> parcel geometry
       -> parcel_marker_anchor snapshot
       -> public/reference vector tile artifacts
       -> active runtime manifest
       -> anchor snapshot outbox events

                         |
                         | HTTP manifest / immutable artifact / webhook event
                         v

                   gongzzang
    -------------------------------------------------
    listing writes
       -> listing_marker_projection
       -> listing_marker_filter_index
       -> listing_marker_base_tile artifacts or dynamic tile cache
       -> listing_marker_delta_log
       -> listing_marker_tombstone_log
       -> listing_marker_mask/count/detail APIs

                         |
                         v

                     web map
    -------------------------------------------------
    platform-core base/reference layers
    + Gongzzang public listing layer
    + authorized private/verified overlays
```

The browser can see public endpoints in developer tools. That is expected. Security for public
routes comes from data minimization, budgets, abuse controls, and not exposing confidential data.
Security for private, internal, or service-to-service routes comes from sessions, authorization,
service identity, and mTLS or short-lived workload identity where available.

## 5. Listing Marker Data Plane Components

### 5.1 Base Tile Layer

Base listing marker tiles are the scale path. They are addressed by:

```text
layer + z + x + y + filter_hash + projection_version + anchor_snapshot_id
```

The base tile may be dynamic initially, but the SSS target is to support static or semi-static
artifact publishing for hot public layers, using the platform-core manifest/promote/rollback pattern.

Minimum public feature data:

| Field | Meaning |
|---|---|
| `id` | Listing marker id or aggregate id |
| `pnu` | Parcel identity |
| `kind` | Marker style discriminator |
| `count` | Number of represented listings |
| `rank` | Stable visual priority |
| `detail_ref` | Opaque lookup key |
| `projection_version` | Listing marker projection version |
| `anchor_snapshot_id` | Platform-core anchor snapshot identity |

The tile must not include contact data, owner notes, private listing detail, or viewer-specific
state.

### 5.2 Delta Overlay

Delta overlay handles fast listing writes before a base tile is rebuilt or cache expires.

Examples:

- a seller publishes a listing;
- an admin approves a listing;
- price or area changes;
- a listing moves from draft to active.

The delta API returns only safe, recent marker changes for affected tile ids and versions. It is not
the source of truth. It lets the UI feel immediate while the base layer catches up.

### 5.3 Tombstone Overlay

Tombstone overlay hides stale markers immediately.

Examples:

- listing deleted;
- listing withdrawn;
- listing visibility changed from public to private;
- business verification required but missing;
- compliance takedown.

Tombstones are more important than deltas because showing a removed/private listing is worse than
temporarily not showing a new public listing.

### 5.4 Filter Mask

Filter masks are a compact optimization for already loaded tiles:

```text
tile + filter_hash + base_version + auth_scope -> show/hide marker ids
```

The mask never returns coordinates. It tells the browser which loaded marker ids remain visible
under a normalized filter. If a mask is missing or stale, the browser must fall back to a full tile
request or server-indexed count.

### 5.5 Dirty Tile Queue

Every listing write computes affected tile ids from the listing PNU anchor and current aggregation
policy. Those tile ids enter a dirty queue.

The queue should support:

- de-duplication by tile id and layer;
- priority for deletes/private transitions;
- bounded worker concurrency;
- retry with DLQ;
- metric for oldest dirty tile age;
- promotion only after artifact validation.

This prevents a single listing write from triggering national rebuilds.

## 6. Low Zoom and Aggregation Policy

The current dynamic query path has a known weak point: `aggregate_count` is still effectively a
placeholder. At SSS level, low zoom must aggregate truthfully.

Policy:

| Zoom | Public listing marker strategy |
|---|---|
| z0-z10 | Region/H3/tile aggregate only; no individual listing features |
| z11-z13 | PNU or grid aggregate depending on density and budget |
| z14+ | Exact listing/PNU marker features when within budget; otherwise truthful aggregate |

This aligns with platform-core's z0-z11 aggregate and z12+ exact anchor idea, while keeping listing
semantics in Gongzzang.

## 7. Public, Private, and Internal Surfaces

### 7.1 Public Browser Surfaces

Public listing marker surfaces may be visible in browser developer tools:

- public listing base tiles;
- public filter registration;
- public count;
- public mask;
- public delta/tombstone for non-confidential public marker ids.

They must expose only minimized derived data. Rate limits do not make them confidential.

### 7.2 Authenticated User Surfaces

Authenticated listing overlays require per-request authorization:

- user's own draft/pending listing overlay;
- saved or personalized state;
- business-verified listing summary;
- broker/admin preview.

These may still be browser-visible, but every request must check session and entitlement.

### 7.3 Service-to-Service Surfaces

Internal surfaces are not browser-callable:

- platform-core anchor artifact import;
- platform-core webhook receiver;
- manifest promotion;
- listing tile artifact promotion;
- admin rebuild and rollback operations.

These use service identity, signed webhook, workload identity or mTLS, audit log, and role checks.

## 8. SSOT Contracts

The following should become explicit SSOT files or remain in existing SSOT files:

| Contract | Suggested SSOT |
|---|---|
| Cross-service ownership | `docs/architecture/platform-core-boundary.v1.json` |
| Public/internal route policies | `docs/architecture/traffic-auth-policy-registry.v1.json` |
| Service auth policy | `docs/architecture/platform-integration/service-auth-policy.v1.json` |
| Platform-core vector manifest | platform-core `catalog.vector_tile_manifest` + API DTO |
| Marker layer registry | new or extended architecture registry, generated from platform-core/Gongzzang layer metadata |
| Listing marker filter schema | Gongzzang listing domain code + generated API DTO |
| Anchor snapshot event | platform-core event schema + Gongzzang webhook contract pin |
| Listing marker artifact promote/rollback | Gongzzang design copied from platform-core manifest lifecycle |

SSOT does not mean "one giant file". It means one authority per fact, with generated or derived
copies clearly marked as copies.

## 9. Required Structural Improvements

### P0: Correctness and Confidentiality

- Add tombstone overlay for delete/withdraw/private transitions.
- Add event-driven delta overlay or confirmed writer overlay for newly published listings.
- Implement truthful low-zoom aggregation instead of placeholder aggregate counts.
- Ensure private/business-verified listing markers are never included in shared public tiles.
- Keep platform-core direct DB access forbidden from Gongzzang runtime.

### P1: Scale and Operations

- Introduce dirty tile queue with affected tile computation and DLQ.
- Add listing marker projection lag metrics, dirty tile age, tombstone age, delta age, tile budget
  errors, and cache hit ratio.
- Add artifact manifest promotion/rollback for public listing marker tiles if load tests continue
  to show live DB tile rendering collapses without cache.
- Add CDN/cache header pass-through for immutable tile artifacts and short-lived dynamic routes.

### P2: Governance and Developer Experience

- Add marker layer registry contract that composes platform-core and Gongzzang layers.
- Generate frontend layer constants from the registry.
- Add CI guardrails for no `bbox`/`bounds`, no listing-owned coordinates, and no direct
  platform-core DB dependency.
- Add compatibility corpus tests for marker feature fields, filter hash, delta, tombstone, and
  manifest versioning.

## 10. QA Gates

Completion claims must be backed by these gates:

| Gate | Required proof |
|---|---|
| No silent marker drop | tests that eligible count equals represented count or aggregate count |
| Delete/private freshness | tombstone test hides stale base-tile marker immediately |
| Write freshness | delta/overlay test shows new public listing before base tile rebuild |
| Low zoom scale | aggregate tests for dense tiles and z0-z13 policy |
| Auth separation | public route cannot return private/business-verified marker ids |
| Platform boundary | no direct platform-core DB import or Catalog write from Gongzzang |
| Route policy SSOT | generated route constants match registry |
| Operational visibility | metrics exist for projection lag, tile errors, dirty queue, DLQ |
| Load proof | public map mix tested with cache hit and cache miss scenarios |

## 11. What This Changes From The Current State

Current state is materially better than the original bbox marker design:

- listing marker endpoint is tile-addressed PBF;
- `filter_hash` is required;
- listing rows do not own canonical coordinates;
- local PNU anchor projection exists as a platform-core-derived copy;
- Redis cache and single-flight reduce duplicate DB work;
- route/security policy registry exists.

But current state is not yet SSS final:

- low zoom listing aggregation is not structurally complete;
- dynamic DB tile generation is still the hot path for listing markers;
- delete/private stale-tile hiding needs tombstone overlay;
- write freshness needs delta overlay or equivalent confirmed writer overlay;
- marker layer registry is not yet a complete cross-service composition contract;
- platform-core's mature manifest promote/rollback pattern is not yet mirrored for Gongzzang listing
  marker artifacts.

## 12. Implementation Sequence

1. Lock this ownership boundary into a reviewed spec.
2. Add or update SSOT registry entries for marker layers, delta, tombstone, and dirty tile policy.
3. Add failing tests for tombstone, delta, low-zoom aggregate, and private/public separation.
4. Implement tombstone overlay first because it prevents stale exposure.
5. Implement delta overlay or writer-confirmed overlay for fresh public writes.
6. Implement low-zoom aggregation and replace placeholder aggregate counts.
7. Add dirty tile queue and metrics.
8. Add optional listing marker artifact publish/promote/rollback if the dynamic path remains
   insufficient under cache-miss load.
9. Update frontend composition to apply base + delta - tombstone - unauthorized.
10. Run route guardrails, unit tests, integration tests, and map-marker load tests.

## 13. Plain Explanation

The map should work like a city notice board.

platform-core owns the official map board: parcel shapes, parcel anchor points, and which base map
files are currently active.

Gongzzang owns the sticky notes placed on that board: listings, prices, visibility, filters, and
who can see which listing.

When a listing is added, Gongzzang can show a fresh sticky note immediately through a delta overlay.
When a listing is deleted or becomes private, Gongzzang puts a tombstone over the old sticky note so
it disappears even if an old cached base tile still exists. Later, the base tile is rebuilt cleanly.

That is stronger than just rate limiting. Rate limiting only slows abuse. This structure prevents
the wrong data from being visible and prevents every map movement from hitting the main listing
tables.

## 14. Korean Delivery Summary

플랫폼코어는 이미 "지도 기준판" 역할을 하고 있다. PNU anchor, parcel geometry,
vector tile manifest, artifact promotion/rollback, outbox event, staff auth, service identity,
readiness, metrics가 platform-core 안에 있다. 따라서 Gongzzang이 별도 지도 플랫폼을 새로
만들면 안 된다.

Gongzzang은 "매물 마커 데이터 plane"을 맡는다. 매물 가격, 상태, 노출 범위, 필터,
비공개/사업자인증 공개 여부, 상세 데이터는 Gongzzang 도메인이다. platform-core로 보내면
관심사 분리가 깨진다.

최종 구조는 다음 한 줄로 설명할 수 있다.

```text
실제 지도에 보이는 매물 = 기본 타일 + 새 변경분 - 삭제/비공개 차단분 - 권한 없는 것
```

- 기본 타일은 대량 트래픽을 버틴다.
- 새 변경분(delta)은 방금 등록/승인된 매물이 빨리 보이게 한다.
- 삭제/비공개 차단분(tombstone)은 캐시가 남아 있어도 사라져야 하는 매물을 즉시 숨긴다.
- 권한 체크는 비공개/사업자인증 매물을 인증 없는 사용자가 가져가지 못하게 한다.

현재 상태는 기존 bbox 방식보다 훨씬 좋다. 하지만 아직 SSS 최종은 아니다. 남은 핵심은
저줌 집계, tombstone, delta, dirty tile queue, cross-service layer registry, 그리고
Gongzzang listing tile artifact promotion/rollback이다.
