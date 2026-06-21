# ADR 0039 - Service-Owned Lakehouse With Platform Core Registry

| Field | Value |
|---|---|
| Date | 2026-06-05 |
| Status | Accepted |
| Preceded by | [ADR 0030](./0030-three-service-architecture.md), [ADR 0034](./0034-catalog-ownership-handover-to-platform-core.md), [ADR 0036](./0036-static-vector-tile-runtime-contract.md), [ADR 0038](./0038-listing-marker-serving-index-filter-mask.md) |
| Platform Core counterpart | `../../../platform-core/docs/adr/0009-cross-service-lakehouse-registry-control-plane.md` |
| Enterprise benchmark | [2026-06-07 enterprise lakehouse/media/registry benchmark](../research/2026-06-07-enterprise-lakehouse-media-registry-benchmark.md) |
| Gongzzang policy SSOT | [Lakehouse Registry integration policy](../architecture/platform-integration/lakehouse-registry-policy.v1.json) |

## Context

Gongzzang owns product data such as listings, listing photos, listing marker projections, Onbid sale
data, court auction data, and market data. Platform Core owns Catalog/common data such as parcels,
buildings, industrial complexes, PNU anchors, and public/reference spatial layers.

Both kinds of data need Bronze/Silver/Gold pipelines, but they must not be mixed in a root-level R2
namespace where `bronze/` and `gold/` do not reveal the owner.

The enterprise benchmark confirms the target pattern: domain-owned physical storage with central
registry/catalog governance, lineage, access policy, and active-version control. R2 Data Catalog or
Iceberg can govern queryable tables, but Platform Core's Lakehouse Registry remains the cross-service
asset identity and consumer-contract control plane.

## Decision

Gongzzang uses a Gongzzang-owned lakehouse storage namespace for Gongzzang-owned datasets.

```text
gongzzang-lakehouse-prod/
|-- bronze/
|-- silver/
|-- gold/
|-- media/
`-- __r2_data_catalog/
```

As of 2026-06-05, `gongzzang-lakehouse-prod` is provisioned in the active Cloudflare R2 account with
the APAC location hint and Standard storage class.

Gongzzang does not write Gongzzang-owned Onbid, court auction, listing, market, or media pipeline
objects into Platform Core-owned R2 namespaces.

Platform Core remains the top-level control plane. Its Lakehouse Registry records Gongzzang-owned
asset locations, active versions, lineage, quality evidence, and consumer bindings. Registry
metadata does not transfer data ownership from Gongzzang to Platform Core.

```text
Gongzzang collector / worker
-> writes Gongzzang-owned objects to Gongzzang-owned R2 namespace
-> verifies checksum, size, row count, and lineage evidence
-> registers run/artifact/version in Platform Core Lakehouse Registry

Gongzzang app/API
-> does not guess canonical R2 keys
-> resolves active governed assets through Platform Core Registry/API contracts
```

## Ownership Matrix

| Asset | Data owner | Storage namespace | Registry owner |
|---|---|---|---|
| Listing OLTP data | Gongzzang | Gongzzang DB | Gongzzang |
| Listing photos | Gongzzang | Gongzzang lakehouse `media/listing-photo/` | Platform Core registry for governed assets only |
| Listing marker Gold tiles/indexes | Gongzzang | Gongzzang lakehouse `gold/` | Platform Core Lakehouse Registry |
| Onbid Bronze/Silver/Gold | Gongzzang | Gongzzang lakehouse | Platform Core Lakehouse Registry |
| Court auction Bronze/Silver/Gold | Gongzzang | Gongzzang lakehouse | Platform Core Lakehouse Registry |
| Parcel/building/PNU anchor | Platform Core | Platform Core lakehouse | Platform Core Lakehouse Registry |

## Forbidden

- Writing Gongzzang-owned lakehouse data into a Platform Core-owned root `bronze/` or `gold/`
  namespace.
- Reintroducing V-World/data.go.kr Catalog ingestion crates into Gongzzang.
- Treating Platform Core R2 object keys as a public API.
- Storing Bronze raw API response bodies in Gongzzang Postgres JSONB as the canonical archive.
- Adding a fallback path that silently writes to legacy `R2_*` settings.

## Configuration Boundary

Gongzzang lakehouse pipelines use `GONGZZANG_LAKEHOUSE_R2_*` configuration.

Listing photo upload storage remains `LISTING_PHOTO_R2_*` at the runtime edge because upload signing,
download signing, object verification, and user-media authorization have different runtime concerns
from batch pipelines. Its bucket may be the same Gongzzang lakehouse bucket, but its object namespace
must stay under `media/listing-photo/`.

The `media/` namespace stores Gongzzang-owned binary media objects such as listing photos, future
listing videos, floor plans, and broker-uploaded documents. It does not replace Bronze/Silver/Gold:
AI extraction outputs, embeddings, normalized captions, quality reports, and searchable metadata are
registered as governed datasets or indexes derived from those media objects.

Platform Core Catalog/raw-data storage remains outside Gongzzang runtime configuration. Gongzzang
resolves governed Catalog artifacts through Platform Core contracts instead of reading Platform Core
R2 object keys directly.

## Current Bucket Interpretation

If an existing R2 bucket currently shows root-level `bronze/`, `gold/`, and `silver-handoff/`, it is
not automatically a cross-service bucket. Unless a migration explicitly says otherwise, Gongzzang
must treat that root-level namespace as Platform Core-owned current/legacy lakehouse material and not
add new Gongzzang-owned Bronze or Gold datasets there.

## Consequences

Positive:

- Gongzzang keeps ownership of product data.
- Platform Core remains the central control plane without becoming the owner of all business facts.
- R2 object layout, active version, lineage, and quality evidence are discoverable through one
  governed registry.
- Later AI/vector indexing can read registered assets across Platform Core and Gongzzang without
  scraping bucket folders.

Cost:

- Gongzzang pipelines must register artifacts after writing them.
- New env/config and guardrails are needed for Gongzzang-owned lakehouse namespaces.
- Existing root-level R2 objects must be inventoried before cleanup or migration.

## Migration Notes

1. Do not delete existing R2 objects until Platform Core inventory classifies owner, lineage, and
   active/retention status.
2. Create or designate a Gongzzang-owned lakehouse namespace before new Onbid/court auction Bronze
   writes.
3. Add Platform Core Registry registration to Gongzzang pipeline completion.
4. Add CI checks that reject new shared root medallion prefixes without explicit owner namespace.
5. Keep Platform Core Catalog consumption through published API/event/artifact contracts.

## Enforcement

The lakehouse registry integration policy SSOT is
`docs/architecture/platform-integration/lakehouse-registry-policy.v1.json`, wired into the
platform integration index. The contract requires consistency with the Platform Core boundary
contract, the required R2 env bucket names, the listing photo media namespace, and the absence of
unmanaged root `gongzzang/bronze`, `gongzzang/silver`, or `gongzzang/gold` writes in active
implementation paths.
