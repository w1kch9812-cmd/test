# V-World Source Boundary

V-World is a Platform Core Catalog input source.

Gongzzang must not add a V-World client, scheduled V-World job, raw-response
capture path, or V-World drift monitor. Gongzzang consumes Catalog facts through
Platform Core published contracts only.

## Gongzzang Contract

Allowed Gongzzang usage:

- Platform Core Catalog HTTP API pinned by
  `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`
- Platform Core events pinned by
  `docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json`
- Immutable PNU anchor artifacts imported into the Gongzzang read model

Disallowed Gongzzang usage:

- Direct V-World HTTP calls
- `vworld-client` or replacement Catalog ACL crates
- `parcel_external_data` writes
- raw capture binaries or R2 raw archive writers
- V-World-specific drift smoke workflows

## Ownership

Platform Core owns:

- V-World credentials and quota handling
- Request/response parsing
- raw response lineage
- schema drift monitoring
- canonical parcel geometry and public/reference spatial layers

Gongzzang owns:

- Listing semantics
- Listing marker serving
- the PNU anchor read-model copy required by listing marker serving

## Guardrails

- `scripts/ci/check-platform-core-boundary.ps1`
- `scripts/ci/check-platform-core-dependency-boundary.ps1`
- `scripts/ci/check-platform-core-catalog-api-contract.ps1`

If V-World source behavior changes, update Platform Core first. Gongzzang should
only update pinned Platform Core contracts after the Platform Core API/event
contract changes.
