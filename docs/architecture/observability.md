# Observability

This document maps the current Gongzzang observability surface.

## 1. Goals

Observability must support:

- request tracing;
- route-level failure diagnosis;
- mutation/audit reconstruction;
- cache and dependency readiness checks;
- production promotion evidence.

## 2. Request Identity

Rust API requests pass through a request-id layer.

Important files:

- `services/api/src/app.rs`
- `services/api/src/http/request_id.rs`
- `services/api/src/http/mutation_ctx.rs`

`MutationContext` carries correlation data into repositories so writes can produce audit records and outbox events inside the same transaction.

## 3. Logging And Tracing

Rust services use `tracing` and `tracing-subscriber`.

Examples:

- `services/api/src/startup.rs`
- `services/api/src/app.rs`
- `services/outbox-publisher/src/main.rs`
- `crates/circuit-breaker/src/execute.rs`
- `crates/circuit-breaker/src/breaker.rs`

Frontend has lightweight OpenTelemetry helper code for panel interactions:

- `apps/web/lib/observability/tracer.ts`
- `apps/web/lib/panel/telemetry.ts`
- `apps/web/instrumentation.ts`

## 4. Health And Metrics

Rust API health routes:

- `/healthz`
- `/healthz/ready`
- `/healthz/db`
- `/internal/metrics`

Important files:

- `services/api/src/routes/health.rs`
- `services/api/src/routes/metrics.rs`
- `docs/architecture/traffic-auth-policy-registry.v1.json`

Readiness checks DB and Redis when configured. Liveness should stay lightweight.

## 5. Audit And Outbox

Audit-critical writes should record:

- actor
- action
- resource kind/id
- before/after state where applicable
- correlation id
- created timestamp

Many DB repositories already use transactional `audit_log` + `outbox_event` patterns.

Important files:

- `crates/db/src/audit_log.rs`
- `crates/db/src/audit_state.rs`
- `crates/db/src/admin_action.rs`
- `crates/db/src/bookmark.rs`
- `crates/db/src/bvq.rs`
- `crates/db/src/listing`

## 6. Catalog Observability Boundary

Catalog public API drift observability belongs to Platform Core, not Gongzzang.

Gongzzang must not reintroduce:

- `.github/workflows/api-drift-smoke-test.yml`
- `crates/operations/api-health`
- `crates/api-health-recorder`
- `crates/db/src/api_health.rs`
- `docs/observability/api-drift-smoke-test.md`

The boundary is enforced by `scripts/lefthook/catalog-m1-boundary.sh` and the
boundary contract `docs/architecture/platform-core-boundary.v1.json`.

## 7. Current Gaps

The repo has tracing, health, audit, policy registries, and load-test evidence scaffolding.

Remaining hardening areas:

- full OTel collector/exporter wiring is not represented here as a completed runtime deployment;
- production SLO dashboards and alert routes are not yet proven in this audit;
- transition from local/script evidence to native Bazel evidence is still in progress.

## 8. Guardrails

The traffic/auth policy registry and the Platform Core boundary are enforced in
CI and pre-commit. The Platform Core catalog boundary is guarded by
`scripts/lefthook/catalog-m1-boundary.sh`; the traffic/auth policy artifacts are
regenerated with `cargo run -p api --bin generate-traffic-auth-policy`.
