# Data Flow

This document maps the current Gongzzang request and data paths.

## 1. Product Request Path

```text
Browser
  -> Next.js app / proxy
  -> Gongzzang Rust API
  -> Gongzzang domain port
  -> Gongzzang repository or approved external adapter
  -> response
```

Core runtime files:

- `apps/web/proxy.ts`
- `apps/web/app/api/proxy/[...path]/route.ts`
- `services/api/src/app.rs`
- `services/api/src/routes`
- `crates/domain`
- `crates/db`

The browser should not talk to the Rust API with ad-hoc route knowledge. Public proxy and route exposure policy are controlled by:

- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `docs/architecture/platform-integration/route-exposure-policy.v1.json`
- `apps/web/lib/policies/traffic-auth-policy.generated.ts`
- `services/api/src/traffic_auth_policy.rs`

## 2. Listing Mutation Path

```text
Browser form/action
  -> Next.js proxy
  -> Rust API listing route
  -> Listing domain aggregate
  -> PgListingRepository
  -> Postgres transaction
       -> listing table
       -> audit_log
       -> outbox_event
```

Mutation context and traceability are carried through `MutationContext`.

Important files:

- `services/api/src/routes/listings`
- `crates/domain/core/listing`
- `crates/db/src/listing`
- `crates/domain/audit/audit-log`
- `crates/domain/audit/outbox-event`

## 3. Platform Core Catalog Read Path

```text
Gongzzang route
  -> Gongzzang Platform Core adapter
  -> Platform Core published API
  -> Gongzzang-owned DTO/read model
```

Gongzzang must not call V-World or data.go.kr Catalog APIs directly.

Current approved adapters:

- `services/api/src/platform_core_parcel_lookup.rs`
- `services/api/src/building_reader.rs`

Current supporting policies:

- `docs/architecture/platform-core-boundary.v1.json`
- `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`
- `docs/backend/circuit-breaker.md`

## 4. Platform Core Event Path

```text
Platform Core event
  -> Next.js public receiver
  -> Rust internal API
  -> platform_core_event_inbox
  -> anchor projection import / cache invalidation
```

Important files:

- `apps/web/app/platform-core/events/route.ts`
- `apps/web/lib/platform-core/event-inbox.ts`
- `services/api/src/routes/platform_core_events.rs`
- `services/api/src/platform_core_anchor_import.rs`
- `migrations/30016_platform_core_event_inbox_anchor_import.sql`

The event receiver must be idempotent and signature-protected.

## 5. Listing Marker Data Path

```text
Platform Core PNU anchor projection
  + Gongzzang listing semantics
  -> listing marker projection/index
  -> listing marker tile/count/mask/delta/tombstone API
  -> map client vector source
```

Important files:

- `crates/db/src/platform_core_anchor.rs`
- `crates/db/src/listing/marker_*`
- `services/api/src/listing_marker_serving`
- `services/api/src/routes/listing_marker_*`
- `apps/web/lib/map/marker-tile-contract.ts`
- `apps/web/lib/map/marker-tile-style.ts`

Public marker routes must not use `bbox` or `bounds` launch request shapes.

## 6. Media/Lakehouse Path

```text
Listing photo lifecycle
  -> R2 object operation
  -> Gongzzang lakehouse/media namespace
  -> Platform Core lakehouse registry integration
```

Important files:

- `services/api/src/photo_upload.rs`
- `services/outbox-publisher/src/listing_photo_lakehouse.rs`
- `services/outbox-publisher/src/platform_core_lakehouse_registry.rs`
- `docs/architecture/platform-integration/lakehouse-registry-policy.v1.json`

## 7. Guardrails

Run these when data-flow ownership changes:

```powershell
./scripts/ci/check-platform-core-boundary.ps1
./scripts/ci/check-platform-core-dependency-boundary.ps1
./scripts/ci/check-platform-integration-policy.ps1
./scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
./scripts/ci/check-traffic-auth-policy-registry.ps1
```
