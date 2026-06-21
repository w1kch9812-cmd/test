# etl-base-layer

`etl-base-layer` is retained only as a fail-closed handover stub.

Platform Core Catalog owns the static vector tile artifact lifecycle:

- source acquisition
- bronze/gold build steps
- R2/CDN object layout
- manifest publication
- rollback pointer management
- lineage metadata

Gongzzang must not run static vector tile ETL. Legacy subcommands remain only so
old scheduled jobs fail with a clear ownership notice instead of mutating stale
state.

## Commands

These commands intentionally exit with code `2`:

```bash
cargo run -p etl-base-layer -- bronze
cargo run -p etl-base-layer -- gold
cargo run -p etl-base-layer -- promote
cargo run -p etl-base-layer -- cleanup-manifest-backups
```

Each command logs that Platform Core Catalog owns the artifact lifecycle and
that Gongzzang is a consumer only.

## Guardrails

The package tests block legacy implementation source from returning:

```bash
cargo test -p etl-base-layer
cargo clippy -p etl-base-layer --all-targets -- -D warnings
```

Repository-level boundary checks also require Platform Core-owned SP9 tooling
and workflows to stay absent from Gongzzang. This is enforced by
`scripts/lefthook/catalog-m1-boundary.sh` and the boundary contract
`docs/architecture/platform-core-boundary.v1.json`.

See [ADR 0036](../../docs/adr/0036-static-vector-tile-runtime-contract.md).
