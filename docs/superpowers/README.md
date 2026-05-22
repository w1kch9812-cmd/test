# Superpowers Design Archive

This directory stores historical implementation specs, plans, and handoff notes. It is an archive,
not the current single source of truth.

Current SSOT for listing marker placement:

- [ADR 0018: Listing Identity Is PNU-First](../adr/0018-pnu-first-identity-no-coordinates.md)
- [ADR 0037: PNU Anchor PBF Marker Tiles](../adr/0037-pnu-anchor-pbf-marker-tiles.md)
- [Gongzzang-owned listing PBF marker tiles design](./specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md)

Supersession rule, effective 2026-05-22:

- older specs or plans that mention `listing.geom_point`, listing latitude/longitude marker
  placement, bbox/bounds marker requests, or PostGIS listing marker placement are historical only;
- those older instructions must not be implemented;
- new listing marker work must use Gongzzang-owned listing PBF tiles joined to platform-core PNU
  anchors by PNU.
