# Layers

This document describes Gongzzang's current dependency direction.

## 1. Layer Rule

Dependency direction:

```text
apps / services
  -> crates/db, adapters, route DTOs
  -> crates/domain ports and value objects
```

Domain crates must not depend on runtime frameworks, databases, HTTP clients, provider SDKs, or UI code.

## 2. Domain Layer

Domain layer owns business meaning and compile-time rules.

Current examples:

- `crates/domain/core/listing`
- `crates/domain/core/listing-photo`
- `crates/domain/core/user`
- `crates/domain/core/shared-kernel`
- `crates/domain/market/real-transaction`
- `crates/domain/market/court-auction`
- `crates/domain/insights/*`
- `crates/domain/audit/*`

Allowed dependencies:

- shared value objects
- repository ports
- pure domain errors
- serializable DTOs when they are domain-owned

Forbidden dependencies:

- `reqwest`
- `sqlx`
- Axum
- Next.js
- provider-specific response structs

## 3. Adapter Layer

Adapters translate between domain ports and infrastructure.

Current examples:

- `crates/db`
- `services/api/src/platform_core_parcel_lookup.rs`
- `services/api/src/building_reader.rs`
- `services/api/src/photo_upload.rs`
- `services/outbox-publisher/src/platform_core_lakehouse_registry.rs`

Adapters may use `reqwest`, `sqlx`, S3/R2 clients, or Redis clients when the owning boundary requires them.

## 4. Service Layer

Services compose repositories, adapters, route state, middleware, and startup policy.

Current services:

- `services/api`
- `services/outbox-publisher`
- `services/etl-base-layer`

`services/etl-base-layer` is a fail-closed handover stub. It must not become active Catalog ETL again.

## 5. App Layer

Frontend apps own user interaction and product UI.

Current app of record:

- `apps/web`

Important frontend boundaries:

- user-facing strings should go through typed i18n;
- public API access should go through approved proxy/client paths;
- Platform Core event receiver is a narrow integration route, not a general Catalog client.

## 6. Policy And Registry Layer

Cross-cutting rules are registered in JSON/policy files and checked by scripts.

Important registries:

- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `docs/architecture/platform-core-boundary.v1.json`
- `docs/architecture/platform-integration/index.v1.json`
- `docs/architecture/verification-transition-ratchet.v1.json`

Generated or derived runtime files must follow those registries.

## 7. Build/Verification Layer

Bazel is the direction for reproducible verification.

Current state:

- native Bazel targets exist for key frontend/build/guardrail paths;
- some CI tasks remain explicit transitions;
- transition state is tracked by `docs/architecture/verification-transition-ratchet.v1.json`.

The goal is not to hide shell scripts. The goal is to retire transitional runners into native Bazel evidence targets when each replacement exists.

## 8. Guardrails

Layer changes should pass:

```powershell
./scripts/ci/check-platform-core-dependency-boundary.ps1
./scripts/ci/check-platform-integration-policy.ps1
./scripts/ci/check-bazel-transition-ratchet.ps1
```
