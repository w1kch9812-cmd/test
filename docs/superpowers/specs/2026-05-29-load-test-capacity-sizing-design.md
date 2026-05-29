# Load Test Capacity Sizing Design

## Purpose

This design turns `docs/research/2026-05-29-rust-aws-sizing-vs-gongzzang-develop.md`
from a pre-sizing analysis into an executable capacity discovery program.

The research document's recommended AWS shape is not treated as a final
production answer. It is the first controlled test specimen. Gongzzang will run
load, stress, spike, soak, and fault tests against that specimen, identify the
first bottleneck, then derive a launch spec with explicit headroom and upgrade
triggers.

## Decision

Use the research document's realistic launch candidate as the first perf
environment:

| Component | Baseline test specimen |
|---|---|
| `gongzzang api` | Small Rust API tasks, starting at 0.5-1 vCPU / 1-2 GiB each |
| `gongzzang web` | 0.5 vCPU / 1 GiB, or 1 vCPU / 2 GiB if SSR load requires it |
| `platform-core api` | Controlled Platform Core environment or contract-faithful mock |
| Database | RDS `db.m7g.large`, gp3 300 GiB, Single-AZ |
| Cost-down comparison | RDS `db.t4g.medium`, gp3 200 GiB |
| Cache | Valkey/Redis small fixed node or equivalent controlled service |
| Load balancer | One ALB |
| Static spatial payload | R2/CDN/immutable artifact path, not repeated PostGIS hot path |

The test order is:

1. Run baseline tests on the baseline specimen.
2. Run capacity discovery until an SLO or resource guard fails.
3. Record the first bottleneck and failure mode.
4. Apply the smallest justified change: query/index/cache/config/spec.
5. Re-run the same scenario.
6. Set launch spec at a conservative fraction of the verified breakpoint.

## SLO Gates

The following gates decide whether a load level is healthy:

| Surface | Gate |
|---|---:|
| API p95 latency | 300 ms or lower |
| API p99 latency | 1000 ms or lower |
| API error rate | Less than 1% |
| Platform Core catalog read p95 | 300 ms or lower |
| Platform Core catalog read p99 | 1000 ms or lower |
| Platform Core webhook receiver p95 | 250 ms or lower |
| Marker tile cache hit p95 | 100 ms or lower |
| Marker tile cache miss p95 | 500 ms or lower |
| Marker tile p99 | 1500 ms or lower |
| API CPU | Sustained average 60% or lower at launch target |
| API memory | No OOM; sustained usage below 70% |
| DB connections | 70% of max or lower |
| DB CPU | No sustained saturation at launch target |
| Slow queries | No repeated hot endpoint slow query |
| CircuitBreaker | Opens only for injected dependency failure, not normal load |

Any exceeded gate creates a breakpoint candidate. A breakpoint is accepted only
after telemetry identifies the first bottleneck.

## Workload Model

Capacity is reported in RPS first, not raw concurrent users. Concurrent users are
derived from the behavior model:

```text
estimated users = RPS * average user think time
in-flight requests = RPS * average response time
```

Use a conservative public launch model until production RUM exists:

| User behavior | Assumption |
|---|---:|
| Search/list/detail interaction interval | 8-12 seconds |
| Map pan/zoom burst window | 1-5 seconds |
| Write traffic share | 10-20% of read traffic |
| Auth/session refresh traffic | Background, low RPS, included in mixed scenario |

Example: 100 read RPS with 10 seconds average think time represents roughly
1,000 active users in the modeled behavior, while in-flight request count is
based on latency.

## Scenarios

### Scenario A: API Read Mix

Purpose: verify Listing and panel read paths under normal B2C traffic.

Traffic mix:

| Path group | Share |
|---|---:|
| health/readiness | 5% |
| Listing list/search | 40% |
| Listing detail | 20% |
| panel open read | 20% |
| Platform Core catalog read through published contract | 15% |

### Scenario B: Map And Marker Mix

Purpose: find whether map gestures or marker tiles create the first bottleneck.

Traffic mix:

| Path group | Share |
|---|---:|
| Listing marker PBF tile cache hit | 55% |
| Listing marker PBF tile cache miss | 15% |
| Listing marker count/filter/mask endpoints | 20% |
| Listing list/detail follow-up reads | 10% |

Canonical coordinates remain owned by Platform Core. Gongzzang tests
Gongzzang-owned Listing semantics and marker serving only.

### Scenario C: Write Mix

Purpose: prove writes do not starve user read paths.

Traffic mix:

| Path group | Share |
|---|---:|
| Listing read mix | 80% |
| Bookmark or Inquiry write | 10% |
| Listing owner write path, when available | 5% |
| auth/session refresh | 5% |

### Scenario D: Platform Core Event Receiver

Purpose: verify webhook resilience separately from user traffic.

Test cases:

| Case | Expected result |
|---|---|
| Signed valid event burst | accepted within SLO |
| Duplicate event burst | 100% duplicate ack without repeated side effects |
| Poison event | dead-letter recorded; no receiver crash |
| Replay surge | alertable signal, not repeated side effects |

### Scenario E: Dependency Faults

Purpose: prove failures degrade safely.

Injected faults:

| Fault | Expected result |
|---|---|
| Platform Core timeout | timeout recorded; CircuitBreaker policy enforced |
| Platform Core 5xx burst | retry/circuit behavior within policy |
| Valkey latency | API latency impact measured; cache fallback explicit |
| DB slow query | first affected route and query identified |
| R2/CDN miss latency | marker/static payload path impact measured |

## Load Levels

The initial levels are inherited from the research document:

| Level | Load |
|---|---:|
| smoke | 5 read RPS + 2 health RPS |
| beta | 20 read RPS + 5 write RPS |
| launch rehearsal | 50 read RPS + 10 write RPS |
| spike | 100-200 read RPS burst |

Capacity discovery extends those levels:

| Step | Target |
|---|---:|
| S1 | 50 RPS mixed |
| S2 | 100 RPS mixed |
| S3 | 200 RPS mixed |
| S4 | 300 RPS mixed |
| S5 | 400 RPS mixed |
| S6 | 600 RPS mixed |
| S7 | 800 RPS mixed, only if earlier steps remain healthy |

Each step holds long enough to stabilize p95/p99, DB metrics, cache metrics, and
service CPU/memory. Stress tests stop when an SLO gate fails, error rate exceeds
threshold, or a shared dependency approaches unsafe saturation.

## Test Types

| Type | Duration | Purpose |
|---|---:|---|
| smoke | 5-10 minutes | verify deployment and basic telemetry |
| baseline load | 30-60 minutes | prove launch rehearsal target |
| stress | step-based | find breakpoint and first bottleneck |
| spike | 1-5 minute bursts | test sudden map/search bursts |
| soak | 6-12 hours first, 24 hours before launch | find leaks and connection churn |
| fault | targeted windows | verify dependency failure behavior |

## Sizing Algorithm

The launch spec is derived from the verified breakpoint:

1. Pick the highest healthy sustained level.
2. Identify the first unhealthy level and the first bottleneck.
3. If the bottleneck is query/index/cache design, fix design before increasing
   AWS size.
4. If the bottleneck is API CPU or memory, scale ECS task CPU/memory or task
   count and re-test.
5. If the bottleneck is DB CPU, IOPS, connections, or PostGIS latency, tune
   query/index/pool settings first, then evaluate RDS class or gp3 IOPS.
6. If the bottleneck is marker tile cache miss, improve precompute/cache/CDN
   before raising DB size.
7. Set launch target at 30-50% below the verified breakpoint, depending on
   failure sharpness.
8. Define autoscaling and upgrade triggers below the breakpoint, not at it.

Example:

| Observation | Sizing decision |
|---|---|
| 200 RPS healthy; 300 RPS p95 fails due API CPU | add API task headroom |
| 200 RPS healthy; 300 RPS p99 fails due DB slow query | tune query/index first |
| 400 RPS fails only during marker miss burst | strengthen marker cache/precompute |
| 600 RPS healthy after fixes | set launch target near 250-350 RPS |

## Evidence

Every run writes immutable local evidence before any sizing claim:

```text
target/audit/load-tests/YYYY-MM-DD/<environment>/<scenario>/
```

Required files:

| File | Contents |
|---|---|
| `run.json` | environment, git SHA, scenario id, start/end, operator |
| `spec.json` | AWS task/RDS/cache/ALB/CDN settings actually tested |
| `thresholds.json` | SLO gates used by the run |
| `k6-summary.json` | latency, error, checks, RPS, VU metrics |
| `cloudwatch.json` | ECS, RDS, ALB, Valkey, and relevant AWS metrics |
| `otel.json` | route latency and dependency spans |
| `bottleneck.md` | first bottleneck analysis |
| `recommendation.md` | recommended launch spec and upgrade triggers |

The final human-readable report lives under:

```text
docs/research/YYYY-MM-DD-load-test-result.md
```

## Tooling

Use k6 for HTTP load generation because it supports scenario mix, thresholds,
RPS control, staged load, JSON output, and CI-friendly execution.

Do not add a new npm or cargo package without approval. If k6 is not already
available on the runner, install it as external operator tooling or containerize
the runner outside `apps/`, `services/`, `crates/`, and `packages/`.

## Safety Rules

- Do not run stress or spike tests against production user traffic paths.
- Do not run tests that consume VWorld or OpenDataPortal quota from Gongzzang.
- Do not bypass Platform Core ownership for Catalog data.
- Do not test with production PII.
- Do not put tokens, cookies, or service secrets in k6 logs or evidence.
- Do not increase AWS resources as the first response to a slow query.
- Do not claim a launch spec without evidence artifacts.

## Deliverables

Implementation should produce:

| Deliverable | Purpose |
|---|---|
| `docs/testing/load.md` | load-test operating manual |
| `tests/load/` | k6 scenarios and shared helpers |
| `scripts/load/` | local run wrappers and evidence normalization |
| CI/manual workflow entry | controlled smoke or rehearsal trigger |
| `docs/research/*-load-test-result.md` | completed result report |

## Non-Goals

This design does not choose a final AWS production spec by itself. It also does
not introduce public traffic replay, production PII replay, direct public API
load, or a new observability stack. It defines the controlled process that must
produce the evidence for a final spec.
