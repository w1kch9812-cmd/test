# Circuit Breaker

This document is the Gongzzang backend SSOT for protected outbound HTTP calls.

## 1. Rule

Every production outbound call from Gongzzang must pass through an owning adapter boundary that provides:

- timeout
- retry
- circuit breaker
- service authentication when required
- traceable error mapping
- audit or lineage logging when the call is audit-relevant

The current Rust crate `crates/circuit-breaker` provides the first three:

- `Policy`
- `Breaker`
- `execute`

It does not currently implement idempotency keys, audit persistence, rate limiting, or provider-specific lineage by itself. Those remain the owning adapter or service responsibility.

## 2. Ownership

### Gongzzang-Owned External Calls

Allowed only when the data/source is Gongzzang-owned and approved by ADR or policy.

Examples:

- Gongzzang-owned law, identity, map, notification, or media integrations
- Gongzzang service-to-service calls to Platform Core published APIs
- Gongzzang lakehouse registry calls through the approved Platform Core contract

### Platform Core-Owned Catalog Calls

Gongzzang must not call V-World or data.go.kr Catalog sources directly.

For Catalog facts, Gongzzang calls Platform Core published contracts only. The adapter may live in `services/api/src/platform_core_*` or another approved service boundary, but the raw source adapter and raw lineage belong to Platform Core.

## 3. Current Policy

The current built-in policy is `Policy::platform_core_default()`.

| Field | Value |
|---|---:|
| `timeout_ms` | `5_000` |
| `max_retries` | `1` |
| `retry_base_ms` | `500` |
| `open_threshold` | `5` |
| `open_window_ms` | `10_000` |
| `open_cooldown_ms` | `30_000` |

Meaning:

- one request attempt can run for up to 5 seconds;
- one retry is allowed after a 500ms base backoff;
- 5 failures inside 10 seconds open the circuit;
- an open circuit blocks calls for 30 seconds before a half-open trial.

## 4. Required Adapter Shape

An outbound adapter should own one reusable `reqwest::Client`, one reusable `Breaker`, and one named `Policy`.

The call path should look like:

```rust
let response = execute(
    &self.breaker,
    &self.policy,
    "platform_core.catalog.get_parcel_by_pnu",
    || {
        let client = self.client.clone();
        let url = url.clone();
        let auth = self.auth.clone();
        async move { send_provider_get(&client, url, auth.as_ref()).await }
    },
)
.await?;
```

Do not create a new breaker per request. A per-request breaker cannot remember recent failures and is therefore not a real circuit breaker.

## 5. Retriable Statuses

Provider adapters must convert retryable HTTP statuses into errors inside the closure passed to `execute`.

Current Platform Core adapters treat these as retryable:

- HTTP 5xx
- HTTP 429

Non-retryable statuses should return a successful HTTP response from the protected call and then be mapped by the adapter. For example, Platform Core parcel lookup maps HTTP 404 to `Ok(None)` after the protected call returns.

## 6. Error Mapping

Adapters must not leak raw provider schemas into domain crates.

Required mapping:

- HTTP/client failures become adapter-specific infra errors.
- Circuit breaker failures become product-facing backend errors.
- Provider response JSON becomes Gongzzang-owned DTOs or read models.
- Unexpected response values become parse/contract errors.

Domain crates should depend on ports and value objects, not `reqwest`, provider SDKs, or provider response structs.

## 7. Audit And Lineage

The circuit breaker crate does not write audit records.

Adapters must decide whether a call is audit-relevant:

- user-visible mutation: audit required;
- admin/security-sensitive read: audit required;
- raw public-data lineage: owning service lineage required;
- ordinary Platform Core read-through lookup: trace/logging usually enough unless policy says otherwise.

Catalog raw lineage belongs to Platform Core. Gongzzang-owned external API raw lineage requires an ADR-approved archive/lineage contract before implementation.

## 8. Forbidden Patterns

Do not:

- call V-World/data.go.kr Catalog APIs directly from Gongzzang runtime;
- instantiate a new `Breaker` per request;
- use ad-hoc `reqwest::get` in production adapters;
- retry non-idempotent mutations without an explicit idempotency key or operation key;
- log Authorization, Cookie, Set-Cookie, provider API keys, service tokens, or raw PII;
- hide external-call failures behind silent fallbacks unless the fallback contract is explicitly documented.

## 9. Existing Good Examples

Current examples to follow:

- `services/api/src/platform_core_parcel_lookup.rs`
- `services/api/src/building_reader.rs`
- `services/outbox-publisher/src/platform_core_lakehouse_registry.rs`

These adapters keep Platform Core calls behind service-owned boundaries and reuse `reqwest::Client`, `Breaker`, and `Policy`.

## 10. Verification

After changing external-call behavior, keep the Platform Core (dependency)
boundary and platform-integration policy intact. The Platform Core catalog
boundary is enforced by `scripts/lefthook/catalog-m1-boundary.sh` and the
boundary contract `docs/architecture/platform-core-boundary.v1.json`.

Run Rust checks for the circuit breaker crate and affected service:

```bash
cargo test -p circuit-breaker
cargo check -p api
```

If the adapter touches Platform Core event, lakehouse, or marker contracts, also run the matching contract guardrail.
