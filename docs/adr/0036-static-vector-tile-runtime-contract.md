# ADR 0036 - Static Vector Tile Runtime Contract

| Field | Value |
|---|---|
| Date | 2026-05-12 |
| Status | Accepted, updated for platform-core handover on 2026-05-28 |
| Supersedes / clarifies | ADR 0021, ADR 0027, ADR 0035 |
| Upstream SSOT | `../platform-core/docs/adr/0004-static-vector-tile-runtime-contract.md` |

## Decision

Platform Core owns public and reference static vector tile artifact lifecycle:

- source acquisition
- bronze and gold build steps
- R2/CDN object layout
- manifest publication
- rollback pointer management
- lineage metadata

Gongzzang is a consumer only. Gongzzang runtime may read the published vector tile manifest through one of these contracts:

1. `NEXT_PUBLIC_TILES_MANIFEST_URL`, when an explicit public CDN/R2 manifest URL is configured.
2. `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL/catalog/v1/vector-tiles/manifest`, when the manifest is read through Platform Core Catalog.

Gongzzang must not write or promote static vector tile artifacts. The retained `services/etl-base-layer` binary is a handover stub only. Legacy subcommands (`bronze`, `gold`, `promote`, `cleanup-manifest-backups`) exit with code `2` and log a Platform Core ownership notice.

## Gongzzang Runtime Contract

The frontend reads a manifest that contains the active version pointer and layer artifact metadata. The minimum runtime shape Gongzzang depends on is:

```json
{
  "schema_version": 1,
  "current_version": "v2026_05",
  "previous_version": "v2026_04",
  "tiles_url_template": "https://static.example.com/gold/{version}/{layer}/{z}/{x}/{y}.pbf",
  "published_at": "2026-05-12T00:00:00Z",
  "artifacts": {
    "parcels": {
      "source_layer": "parcels",
      "tile_min_zoom": 8,
      "tile_max_zoom": 16,
      "render_min_zoom": 14,
      "render_max_zoom": 22,
      "tilejson_object_key": "gold/v2026_05/parcels.json",
      "object_key_prefix": "gold/v2026_05/parcels/",
      "lineage": {
        "source_record_id": "uuid",
        "manifest_file_asset_id": "uuid",
        "tilejson_file_asset_id": "uuid",
        "source_file_asset_ids": ["uuid"]
      }
    }
  }
}
```

Gongzzang may interpret this manifest for map rendering. Gongzzang must not override Platform Core lineage, artifact version, object key, or active manifest pointer.

## Current Code Ownership

Current Gongzzang code is limited to:

- `services/etl-base-layer/src/main.rs` - process bootstrap for the handover stub
- `services/etl-base-layer/src/cli.rs` - legacy command parser that routes to disabled handlers
- `services/etl-base-layer/src/handover.rs` - fail-closed Platform Core ownership notice
- `services/etl-base-layer/src/runtime.rs` - tracing and Sentry initialization only
- `services/etl-base-layer/tests/platform_core_handover.rs` - regression tests that block legacy implementation source reintroduction

Deleted Gongzzang responsibilities:

- bronze source download
- DTMK preparation
- tippecanoe build and PMTiles decomposition
- R2 upload helpers
- manifest promotion, rollback, and cleanup implementation

Those responsibilities belong to Platform Core Catalog.

## Rejected Options

### RDS/PostGIS real-time vector tile server

Rejected. It adds runtime query, encoding, cache invalidation, and cost pressure for largely static public/reference layers.

### Naver internal vector tile endpoint as Gongzzang domain source

Rejected. Naver internal tiles are an implementation detail of the Naver SDK and do not provide Gongzzang-owned PNU identity, lineage, versioning, or rollback guarantees.

### Gongzzang-owned static tile ETL fallback

Rejected after Platform Core handover. A fallback ETL path inside Gongzzang would duplicate Catalog ownership and weaken SSOT. The only retained binary behavior is fail-closed handover messaging.

## Gates

Gongzzang must keep these gates green:

- `scripts/lefthook/catalog-m1-boundary.sh` (Platform Core boundary)
- `docs/architecture/platform-core-boundary.v1.json` boundary contract
- `cargo test -p etl-base-layer`
- `cargo clippy -p etl-base-layer --all-targets -- -D warnings`

The boundary SSOT classifies `services/etl-base-layer` as `gongzzang/platform_core_handover_stub`. If legacy implementation files such as `src/bronze`, `src/gold`, or `src/r2_upload` reappear, `services/etl-base-layer/tests/platform_core_handover.rs` must fail.

## Consequences

Positive:

- Gongzzang no longer carries Platform Core ETL implementation code.
- Static vector tile artifact lifecycle has one owner.
- Legacy scheduled jobs fail closed instead of silently mutating stale R2/manifest state.
- Dependency surface for `etl-base-layer` is reduced to stub runtime needs.

Tradeoffs:

- Historical ADRs and plans may still mention removed files as historical context.
- DB schema cleanup for legacy pipeline/raw tables remains blocked until an explicitly approved migration removes them.
