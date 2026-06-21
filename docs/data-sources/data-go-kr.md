# data.go.kr Source Boundary

Catalog-related data.go.kr integrations are Platform Core Catalog input
sources.

Gongzzang must not add a data.go.kr Catalog client, parser, scheduled ingest
job, raw-response capture path, or drift monitor. Gongzzang consumes building
and parcel facts through Platform Core published contracts only.

## Gongzzang Contract

Allowed Gongzzang usage:

- Platform Core Catalog HTTP API pinned by
  `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`
- Platform Core events pinned by
  `docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json`
- Route-facing translation from Platform Core building responses into
  Gongzzang API response shapes

Disallowed Gongzzang usage:

- Direct data.go.kr HTTP calls for Catalog data
- `data-go-kr-client` or replacement Catalog ACL crates
- building-register sync jobs
- `parcel_external_data` writes
- raw capture binaries or R2 raw archive writers
- data.go.kr-specific drift smoke workflows

## Ownership

Platform Core owns:

- data.go.kr credentials and quota handling
- request/response parsing
- raw response lineage
- schema drift monitoring
- canonical building and parcel Catalog facts

Gongzzang owns:

- `/api/buildings` route shape
- Listing semantics
- Listing marker serving

## Guardrails

- Platform Core catalog boundary — `scripts/lefthook/catalog-m1-boundary.sh`
- Platform Core boundary / dependency-boundary contract — `docs/architecture/platform-core-boundary.v1.json`
- Platform Core Catalog API consumer contract — `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`

If data.go.kr source behavior changes, update Platform Core first. Gongzzang
should only update pinned Platform Core contracts after the Platform Core
API/event contract changes.
