# Naver Maps SDK Data Source Audit

Date: 2026-05-11

Status: retained as historical evidence for ADR 0036.

## Scope

This note records the source and runtime assumptions behind using Naver Maps as the base map while
serving Gongzzang-owned domain vector layers from our own tile contract.

It is not a license grant, production cost estimate, or permanent API contract for Naver internals.

## Findings

- Naver Maps JavaScript SDK is loaded from `https://oapi.map.naver.com/openapi/v3/maps.js`.
- The GL submodule is loaded from the same `oapi.map.naver.com` origin.
- Runtime map assets may come from Naver-controlled `map.naver.*`, `map.naver.net`, and
  `pstatic.net` hosts.
- Naver internal vector sources are implementation details. They must not become Gongzzang domain
  SSOT for PNU, parcel, listing, auction, or industrial-complex data.
- Naver base-map features can be used for visual context, but Gongzzang domain interaction must use
  our own feature IDs and our own vector tile or marker tile contracts.

## Contract Implication

ADR 0036 keeps Naver as the base map provider only. Domain layers must be loaded from the
Gongzzang/platform-core tile contracts:

- static parcel/reference vector tiles: platform-core Catalog contract
- listing marker tiles: Gongzzang-owned PNU-anchor PBF contract
- listing identity: PNU-first, with no listing-owned canonical latitude/longitude

## Follow-Up

Any future Naver SDK upgrade must re-check:

- SDK script origin
- GL submodule loading
- CSP allowlist impact
- attribution visibility
- whether private `_mapbox` access still works in the current runtime

If `_mapbox` access breaks, the fallback decision must be made through an ADR rather than by relying
on a hidden Naver SDK implementation detail.
