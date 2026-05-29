# Platform Core physical extraction DB schema handoff

Date: 2026-05-28

## Status

Gongzzang code, workspace membership, runtime dependencies, extracted service paths, and CI
boundary gates now enforce Platform Core physical extraction for Catalog/ETL/raw-capture code.

Gongzzang's Platform Core Catalog API consumer surface is pinned in
`docs/architecture/platform-core-catalog-api-contract.v1.pin.json`.
`scripts/ci/check-platform-core-catalog-api-contract.ps1` verifies the local
parcel/building clients and, when present, the sibling Platform Core OpenAPI.
`services/data-pipeline` and `services/scraper-py` are treated as Platform Core-owned
Catalog ETL paths and must remain absent from Gongzzang.

User approval for Gongzzang DB cleanup was received on 2026-05-28. The approved
forward migration is `migrations/30015_drop_platform_core_legacy_schema.sql`.
It targets the Gongzzang database only and does not touch the Platform Core database.
This approval is limited to legacy schema cleanup. It does not approve the
future durable Platform Core anchor event inbox/import migration, which is
reserved as `migrations/30016_platform_core_event_inbox_anchor_import.sql` and
still requires separate explicit DB schema approval.

The boundary gate also blocks direct SQL usage of Platform Core canonical Catalog
tables (`industrial_complex`, `parcel`, `building`, `manufacturer`) in Gongzzang
code and migrations. Gongzzang may keep product-owned external references, such
as bookmark or featured-content target kinds, but must not create/read/write
those canonical tables locally.

Legacy M1 Catalog event schema code in Gongzzang shared-kernel was removed as a
Platform Core-owned event schema asset. Gongzzang keeps pinned contract copies
and receiver code, not local Catalog event publishers.

## Cleanup Migration

`migrations/30015_drop_platform_core_legacy_schema.sql` drops the following legacy
Platform Core-owned remnants from the Gongzzang DB:

- `api_health_check`
- `parcel_external_data`
- `pipeline_run`
- `pipeline_schedule`

The migration intentionally does not use `CASCADE`; unexpected remaining dependencies must
fail the migration instead of being silently removed. Historical applied migrations remain
immutable and are not edited.

## Legacy Schema Token Ledger

These are the only allowed legacy schema tokens in `migrations/*.sql`.
They are recorded in `docs/architecture/platform-core-boundary.v1.json`
under `allowed_legacy_schema_tokens`.

| Token | Migration | Reason |
|---|---|---|
| `pipeline_schedule` | `migrations/10004_pipeline_tables.sql` | historical legacy ETL schema creation |
| `pipeline_run` | `migrations/10004_pipeline_tables.sql` | historical legacy ETL schema creation |
| `force_pipeline_run` | `migrations/10005_operations_tables.sql` | historical migration comment for legacy ETL admin action |
| `parcel_external_data` | `migrations/30006_parcel_external_data.sql` | historical legacy Catalog raw table creation |
| `parcel_external_data` | `migrations/30010_parcel_external_data_r2_pointer.sql` | historical legacy Catalog raw table alteration |
| `parcel_external_data` | `migrations/30011_parcel_external_data_r2_key_idx.sql` | historical legacy Catalog raw index |
| `api_health_check` | `migrations/30007_api_health_check.sql` | historical legacy Catalog API drift health table creation |
| `api_health_check` | `migrations/30015_drop_platform_core_legacy_schema.sql` | approved Gongzzang DB cleanup migration |
| `parcel_external_data` | `migrations/30015_drop_platform_core_legacy_schema.sql` | approved Gongzzang DB cleanup migration |
| `pipeline_run` | `migrations/30015_drop_platform_core_legacy_schema.sql` | approved Gongzzang DB cleanup migration |
| `pipeline_schedule` | `migrations/30015_drop_platform_core_legacy_schema.sql` | approved Gongzzang DB cleanup migration |

Any occurrence outside this ledger fails `scripts/ci/check-platform-core-boundary.ps1`.
The same boundary gate also verifies `tests/migrations/test_v001_full.sh`: dropped
Platform Core legacy tables must not remain in `EXPECTED_TABLES`, and must be
listed in `FORBIDDEN_TABLES` so the DB migration smoke proves they are absent.
It also verifies `.github/workflows/db-migrations.yml` is Gongzzang-owned and
keeps running `bash tests/migrations/test_v001_full.sh` against
`postgis/postgis:17-3.5`, so this smoke cannot be removed from CI silently.

## Canonical Catalog Table Guard

`docs/architecture/platform-core-boundary.v1.json` records
`forbidden_canonical_catalog_tables`. `scripts/ci/check-platform-core-boundary.ps1`
scans `apps/`, `services/`, `crates/`, `packages/`, and `migrations/` for direct SQL
usage patterns including `create table`, `alter table`, `references`, `from`, `join`,
`insert into`, `update`, and `delete from`.
The guard also rejects schema-qualified forms such as `catalog.parcel`, so a local
query cannot bypass the boundary by adding a schema prefix.

This keeps product references distinct from canonical Platform Core storage:

- Allowed: `bookmark_external.target_kind = 'industrial_complex'`
- Forbidden: `select * from industrial_complex`
- Forbidden: `select * from catalog.parcel`
- Forbidden: `create table building (...)`

## Direct Platform Core DB Guard

Gongzzang must consume Platform Core through published HTTP/event/artifact
contracts, not a Platform Core database connection string. The boundary gate now
scans code roots, GitHub workflow YAML, plus root config files such as `.env.example`, and rejects
direct database aliases such as `PLATFORM_CORE_DATABASE_URL`,
`PLATFORM_CORE_DB_URL`, `PLATFORM_CORE_POSTGRES_DSN`, and PostgreSQL URLs whose
database name contains `platform_core`.

## Workflow Guard

The boundary gate scans `.github/workflows/*.yml` and `.github/workflows/*.yaml`
for forbidden Catalog source tokens such as `api.vworld.kr` and
`apis.data.go.kr`. This prevents reintroducing Platform Core-owned ETL or API
drift jobs under a new workflow filename.

## Runtime Env and Local DB Guard

`.env.example` now documents the Platform Core integration as HTTP-only:
`PLATFORM_CORE_API_BASE_URL` for the Rust API and
`NEXT_PUBLIC_PLATFORM_CORE_BASE_URL` for the web runtime. It no longer exposes
V-World, data.go.kr, Catalog ETL, generic Catalog R2, or LLM provider settings
as Gongzzang runtime configuration.

The boundary gate rejects those stale env examples if they are reintroduced.
Local Gongzzang Postgres now defaults to host port `15432` through
`POSTGRES_HOST_PORT`, because this Windows machine reserves `5500` in the TCP
excluded port range. The local `.env` was updated to
`localhost:15432/gongzzang`; this still targets the Gongzzang DB only.

## Verification Already Run

- `scripts/ci/check-platform-core-boundary.tests.ps1`
- `scripts/ci/check-platform-core-boundary.ps1 -Root .`
- `scripts/ci/check-platform-core-catalog-api-contract.tests.ps1`
- `scripts/ci/check-platform-core-catalog-api-contract.ps1 -Root .`
- `scripts/ci/check-platform-core-dependency-boundary.tests.ps1`
- `scripts/ci/check-platform-core-dependency-boundary.ps1 -Root .`
- Disposable PostGIS migration smoke via `cargo sqlx migrate run --source migrations`
  against `postgis/postgis:17-3.5`: `tables=20`, `legacy_tables=0`,
  `postgis_extensions=1`
- Actual local Gongzzang Docker Postgres volume:
  - `DATABASE_URL` host `localhost`, port `15432`, database `gongzzang`
  - `cargo sqlx migrate info --source migrations` shows `30015/installed`
  - `_sqlx_migrations` contains `30015|t`
  - legacy Platform Core table count for `api_health_check`,
    `parcel_external_data`, `pipeline_run`, and `pipeline_schedule` is `0`
- Disposable PostGIS re-smoke on host port `16432` via Windows `cargo sqlx migrate run`:
  `tables=20`, `legacy_tables=0`, `postgis_extensions=1`,
  `migration_30015_success=t`
- `cargo fmt --check`
- `cargo check --workspace` with `SQLX_OFFLINE=true`
- `cargo clippy --workspace --all-targets -- -D warnings` with `SQLX_OFFLINE=true`
- `cargo test --workspace` with `SQLX_OFFLINE=true`
- `cargo test -p parcel-lookup`
- `cargo test -p api` with `SQLX_OFFLINE=true`
- `cargo test -p db` with `SQLX_OFFLINE=true`
- `cargo test -p shared-kernel`
- `cargo test -p admin-action-domain`
- `cargo test -p etl-base-layer` with `SQLX_OFFLINE=true`
