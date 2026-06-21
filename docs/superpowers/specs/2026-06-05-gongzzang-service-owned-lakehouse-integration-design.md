# Gongzzang Service-Owned Lakehouse Integration Design

Status: Approved design
Date: 2026-06-05
Owner: `gongzzang`
Counterpart: `../../../../platform-core/docs/adr/0009-cross-service-lakehouse-registry-control-plane.md`
Enterprise benchmark: `../../research/2026-06-07-enterprise-lakehouse-media-registry-benchmark.md`
Policy SSOT: `../../architecture/platform-integration/lakehouse-registry-policy.v1.json`

## Summary

Gongzzang data gets its own lakehouse storage namespace. Platform Core keeps the registry that knows
where governed data is and which version is active.

This means:

- Gongzzang owns Onbid, court auction, listing marker, listing media, and market datasets.
- Platform Core owns PNU anchors, parcel/building geometry, industrial complexes, and public spatial
  references.
- Platform Core's Lakehouse Registry is the cross-service address book and governance layer.
- R2 Data Catalog / Iceberg is a table/catalog mechanism for queryable lakehouse tables, not a
  replacement for Platform Core's cross-service registry.

## Target Flow

```text
Gongzzang pipeline
-> write to gongzzang-owned R2 bucket
-> verify checksum/size/row count/source metadata
-> register artifact/version/lineage in platform-core Lakehouse Registry
-> Gongzzang runtime resolves active artifacts through registry/API contracts
```

## Target Storage

```text
gongzzang-lakehouse-prod/
|-- bronze/source=onbid-sale/...
|-- bronze/source=court-auction/...
|-- silver/dataset=onbid-sale/...
|-- silver/dataset=court-auction/...
|-- gold/listing-marker-tiles/...
|-- media/listing-photo/...
`-- __r2_data_catalog/
```

The current bucket that exposes root-level `bronze/` and `gold/` must not receive new Gongzzang-owned
datasets unless it is explicitly reclassified as a Gongzzang-owned namespace.

## First Datasets

| Dataset | Layer start | Owner | Notes |
|---|---|---|---|
| Onbid sale API raw | Bronze | Gongzzang | data.go.kr/KAMCO source, Gongzzang market domain |
| Court auction raw | Bronze | Gongzzang | court auction domain, not Platform Core Catalog |
| Listing marker tiles | Gold | Gongzzang | derived from listing projection plus Platform Core anchor lineage |
| Listing photos | Media/object set | Gongzzang | stored under `media/listing-photo/`, may be governed when attached to listing/AI lineage |

## Required Platform Core Registry Calls

For every governed write:

1. Start or register ingestion/build run.
2. Register object artifacts with checksum and size.
3. Register dataset version.
4. Register lineage to source assets.
5. Submit quality evidence.
6. Promote only after policy passes.

## Configuration Contract

Gongzzang-owned lakehouse writers use:

- `GONGZZANG_LAKEHOUSE_R2_ACCOUNT_ID`
- `GONGZZANG_LAKEHOUSE_R2_ACCESS_KEY`
- `GONGZZANG_LAKEHOUSE_R2_SECRET_KEY`
- `GONGZZANG_LAKEHOUSE_R2_BUCKET`

`LISTING_PHOTO_R2_*` remains at the runtime edge for upload signing, download signing, object
verification, and authorization. It may point at the same bucket as `GONGZZANG_LAKEHOUSE_R2_BUCKET`,
but listing-photo object keys must live under `media/listing-photo/`.

The `media/` namespace is for Gongzzang-owned binary media objects. Do not put every AI or search
artifact there: extracted text, normalized tags, embeddings, model outputs, and quality evidence are
derived datasets/indexes that belong in governed dataset or vector/index storage with lineage back to
the media object.

## Guardrails

- No direct V-World/data.go.kr Catalog raw pipeline in Gongzzang.
- No canonical marker latitude/longitude in listing rows.
- No raw Bronze body in Postgres JSONB as primary archive.
- No direct guessing of Platform Core R2 keys.
- No writes to shared root `bronze/` or `gold/` without owner namespace.
- `scripts/ci/check-lakehouse-registry-integration` must pass before Gongzzang-owned lakehouse
  artifact writers or media namespace changes are accepted.

## Implementation Order

1. Document the cross-service decision.
2. Add Platform Core Lakehouse Registry schema/API plan.
3. Add Gongzzang lakehouse env/config contract.
4. Create Gongzzang-owned Onbid/court Bronze writers against the Gongzzang namespace.
5. Register artifacts in Platform Core Registry.
6. Add CI guardrails for root-prefix contamination and owner mismatch.

## Self-Review

- The design does not make Platform Core the owner of Gongzzang data.
- The design does not create a fourth service above Platform Core.
- The bucket/prefix rule is explicit enough to prevent root-level medallion mixing.
- Registry and R2 Data Catalog responsibilities are separate.
