# Caching

This document describes the current caching and freshness model.

## 1. Rule

Cache is an accelerator, not the source of truth.

Authoritative data remains in:

- Gongzzang Postgres for Gongzzang product records;
- Platform Core for Catalog facts and PNU anchors;
- R2/lakehouse objects for immutable media or data artifacts;
- Redis/Valkey-compatible stores only for cache, session, rate limit, locks, and inbox deduplication.

## 2. Runtime Cache Uses

Current Redis-compatible uses include:

- Next.js session storage: `apps/web/lib/session/store.ts`
- session refresh single-flight: `apps/web/lib/session/single-flight.ts`
- frontend/API proxy rate limiting: `apps/web/lib/ratelimit.ts`
- Platform Core event inbox dedupe: `apps/web/lib/platform-core/event-inbox.ts`
- Rust API JTI denylist: `crates/auth/src/jti_denylist.rs`
- Rust API backend rate limit: `services/api/src/backend_rate_limit.rs`
- listing marker serving cache/single-flight: `services/api/src/listing_marker_serving`

## 3. Production Redis Requirement

Rust API startup treats missing Redis differently by environment:

- development: Redis-dependent checks may degrade or skip;
- production: missing `REDIS_URL` is fail-fast where security would otherwise fail open.

Important file:

- `services/api/src/startup.rs`

Next.js env validation also requires a production-safe Redis URL.

Important file:

- `apps/web/lib/env.ts`

## 4. Marker Cache Model

Listing marker serving uses cache and single-flight as secondary protection.

The primary scaling strategy is:

- precomputed/derived serving indexes;
- stable filter hashes;
- tile-shaped marker requests;
- delta/tombstone overlays for freshness.

Numeric filters should not create unbounded cache-only correctness. Cache can help hot repeated requests, but correctness must come from indexes and normalized filter contracts.

## 5. Platform Core Cache Invalidation

Platform Core events can invalidate catalog-related cache and trigger anchor projection import.

Important files:

- `apps/web/app/platform-core/events/route.ts`
- `apps/web/lib/platform-core/event-inbox.ts`
- `services/api/src/routes/platform_core_events.rs`
- `docs/architecture/platform-core-webhook-receiver-contract.v1.pin.json`

## 6. Static Tile Cache

Static reference tile lifecycle is Platform Core-owned.

Gongzzang's map client may consume manifests and immutable tile URLs, but Gongzzang does not own the build/promote lifecycle for Platform Core reference tiles.

## 7. Guardrails

The traffic/auth, Platform Core event-receiver, and PNU-anchor PBF marker
contracts are enforced in CI and pre-commit hooks. The Platform Core boundary is
guarded by `scripts/lefthook/catalog-m1-boundary.sh`; the traffic/auth policy
artifacts are regenerated from the registry with
`cargo run -p api --bin generate-traffic-auth-policy`.
