# Gongzzang Map Zoom Policy Design

Date: 2026-05-27

## Purpose

Gongzzang needs a product-owned map zoom policy that is separate from platform-core's spatial data
availability contract.

Platform-core owns parcel geometry, PNU marker anchors, and public/reference spatial layers. Its
`parcel_anchor` exact tile availability begins at zoom 12. That is a data-serving contract, not a
Gongzzang rendering rule.

Gongzzang owns listing semantics and listing marker presentation. Listing markers should render at
the parcel interaction level, currently zoom 14 through 22.

## Decision

Add `apps/web/lib/map/map-zoom-policy.ts` as the Gongzzang frontend SSOT for product map zoom policy.

The initial policy is:

- platform-core exact PNU anchor availability: zoom 12+
- Gongzzang parcel interaction level: zoom 14-22
- Gongzzang listing marker rendering: zoom 14-22

Platform-core vector tile artifacts still use the platform-core manifest's `render_min_zoom` and
`render_max_zoom`. Gongzzang does not override those shared spatial layer render contracts.

## Boundaries

- Platform-core decides when exact anchor data can be requested safely.
- Gongzzang decides when product markers are visible to users.
- Gongzzang listing marker PBF routes reject public tile requests outside the Gongzzang listing
  marker zoom range.

## Verification

The implementation is covered by:

- `apps/web/tests/unit/map/map-zoom-policy.test.ts`
- `apps/web/tests/unit/map/marker-tile-contract.test.ts`
- `apps/web/tests/unit/map/marker-tile-style.test.ts`
- `services/api/src/routes/listing_marker_tiles.rs`
- `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
