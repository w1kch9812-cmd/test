# Listing Marker Serving Index And Filter Mask Implementation Plan

> This file is the index for the split implementation plan. The detailed task checklists live in the part files below so each document stays below the 500-line SSS guardrail.

**Goal:** Build scalable Gongzzang listing marker serving so common filters apply instantly in the browser while exact counts, unseen tiles, and advanced filters are backed by a server-side marker projection/index.

**Architecture:** Extend the existing PNU-anchor listing PBF path from `all-active-v1` into a typed normalized filter model. Keep platform-core as PNU anchor SSOT, make Gongzzang own listing marker projection/index, expose base marker tiles with safe filter properties, add count/mask companion APIs, and apply fast map filters through the map layer before server results arrive.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL/PostGIS, Mapbox GL source/layer API through Naver GL bridge, Next.js 16, React 19, Zustand, Vitest.

## Split Plan

- [Part 1: Domain Filter Contract](./2026-05-26-listing-marker-serving-index-filter-mask.part-01-domain-filter.md)
- [Part 2: Projection And Tiles](./2026-05-26-listing-marker-serving-index-filter-mask.part-02-projection-and-tiles.md)
- [Part 3: Counts And Instant Filters](./2026-05-26-listing-marker-serving-index-filter-mask.part-03-counts-and-instant-filters.md)
- [Part 4: Registration, Mask, Docs](./2026-05-26-listing-marker-serving-index-filter-mask.part-04-registration-mask-docs.md)

## Contract Keywords

This index intentionally keeps the architectural guardrail tokens that are checked by `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`:

- `listing_marker_projection`
- `listing_marker_filter_registry`
- `buildListingMarkerLayerFilter`
- `marker-counts/listing`
- `marker-masks/listing`

## Execution Order

1. Complete Part 1 before changing database read models.
2. Complete Part 2 before changing frontend tile semantics.
3. Complete Part 3 before enabling authoritative count refresh behavior.
4. Complete Part 4 before any completion claim.

## Final Verification

The final verification set is preserved in Part 4. Completion claim is allowed only when every command in that section exits 0 and the migration approval gate for `30013_listing_marker_projection.sql` has been satisfied.
