# Platform Core Anchor Manifest Runtime Design

## Goal

Gongzzang map runtime must consume the active Platform Core vector tile manifest as the SSOT for public PNU anchor tiles.

## Architecture

Platform Core owns parcel geometry, parcel marker anchors, and public/reference spatial layers. Gongzzang owns listing semantics and Gongzzang listing marker PBF tiles. Gongzzang must not call the retired Platform Core marker contract or hardcode R2 prefixes.

The runtime reads `NEXT_PUBLIC_TILES_MANIFEST_URL` first, or `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL/catalog/v1/vector-tiles/manifest` otherwise. Tile URLs are materialized from `tiles_url_template` by replacing `{object_key_prefix}` with each artifact's `object_key_prefix`.

## Layers

- `parcel_anchor_aggregate`: low-zoom aggregate anchor layer. It is not clickable as a single parcel.
- `parcel_anchor`: exact PNU anchor layer. It is clickable and resolves parcel detail by `pnu` or `detail_ref`.
- `listing`: Gongzzang-owned listing marker layer served through same-origin listing marker tile routes.

## Non-Goals

- Do not move listing semantics into Platform Core.
- Do not add listing latitude/longitude or `geom_point`.
- Do not restore public bbox/bounds marker requests.
- Do not implement parcel polygon generation in Gongzzang.

## Verification

Unit tests must prove:

- Manifests without `parcels` are accepted when anchor artifacts are present.
- `{object_key_prefix}` templates materialize exact tile URLs.
- Root-relative manifest templates resolve against the fetched manifest origin.
- Platform Core anchor style registration creates aggregate and exact layers.
- Gongzzang listing marker tiles remain same-origin and coordinate-free.
