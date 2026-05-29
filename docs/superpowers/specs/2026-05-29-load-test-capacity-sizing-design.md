# Load Test Capacity Sizing Design

## Purpose

This design turns `docs/research/2026-05-29-rust-aws-sizing-vs-gongzzang-develop.md`
from a pre-sizing analysis into an executable capacity discovery program.

The research document's recommended AWS shape is not treated as a final
production answer. It is the first controlled test specimen. Gongzzang will run
load, stress, spike, soak, and fault tests against that specimen, identify the
first bottleneck, then derive a launch spec with explicit headroom and upgrade
triggers.

## Enterprise Reference Patterns

This design follows patterns from public large-scale reliability and cloud
engineering references:

| Source | Pattern to adopt |
|---|---|
| [Google SRE Book: Introduction](https://sre.google/sre-book/introduction/) | Capacity planning must include regular load testing that maps raw capacity, such as servers and disks, to service capacity. Demand planning must include both organic product growth and inorganic events such as launches or campaigns. |
| [Google SRE Book: Cascading Failures](https://sre.google/sre-book/addressing-cascading-failures/) | Preventing overload starts with testing server capacity limits and overload failure modes. Capacity planning should be coupled with performance testing to find the load at which a service fails. |
| [AWS Distributed Load Testing on AWS](https://docs.aws.amazon.com/solutions/distributed-load-testing-on-aws/) | Enterprise load testing needs distributed runners, downloadable raw results, latency/error analysis, baseline comparison across runs, and visibility into RDS, CloudFront/CDN, ECS/EKS, and other resource bottlenecks. |
| [AWS DLT customer examples](https://docs.aws.amazon.com/solutions/distributed-load-testing-on-aws/) | Major launch tests can run at multiples of expected traffic; AWS publishes a customer example of testing at six times expected traffic before launch. Gongzzang treats 6x as an event-rehearsal option, not a default always-on target. |
| [Microsoft Azure Well-Architected: Performance Testing](https://learn.microsoft.com/en-us/azure/well-architected/performance-efficiency/performance-test) | Define acceptance criteria, establish baselines, use hypothesis-driven experiments, compare future runs against baselines, and collect live resource metrics during tests. |
| [Microsoft Azure Well-Architected: Reliability Testing](https://learn.microsoft.com/en-us/azure/well-architected/reliability/testing-strategy) | Reliability tests should validate load spikes, dependency failures, self-healing, self-preservation, graceful degradation, and blast-radius controls. |
| [Netflix Tech Blog: Performance Under Load](https://netflixtechblog.medium.com/performance-under-load-3e6fa9a60581) | Static concurrency limits become stale as topology and latency change. Under overload, unbounded queues raise latency, trigger timeouts, and can cascade. Gongzzang must test bounded queues, rejection, and circuit behavior explicitly. |

These references change the design from "run k6 and read p95" to a controlled
capacity engineering loop: hypothesis, production-like test specimen, staged
load, bottleneck evidence, overload behavior, mitigation, repeat, and baseline
comparison.

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

## Enterprise Quality Bar

Every load test is an experiment, not an ad hoc traffic blast. Each experiment
must declare:

| Field | Requirement |
|---|---|
| hypothesis | What should happen and what might fail first |
| scenario | Exact workload mix and route groups |
| specimen | Exact AWS/container/database/cache/CDN settings |
| data shape | Listing count, marker tile state, cache warm/cold state |
| SLO gates | Latency, error, resource, and recovery gates |
| stop condition | When the test must stop before damaging shared systems |
| evidence path | Immutable output directory under `target/audit/load-tests` |
| decision owner | Person or team responsible for accepting the sizing result |
| follow-up action | Tune query/cache/config/spec, then re-run same scenario |

The perf environment should be production-like for the components under test:
same runtime build, same schema, same migrations, same connection pool policy,
same route/auth controls, and representative data volume. Production data must
be scrubbed or synthetically generated; production PII is not allowed.

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
| Recovery after stress | Returns to baseline latency/error without restart |
| Queue/concurrency guard | Rejects or sheds before unbounded latency growth |

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

Major launch or campaign rehearsal adds explicit traffic multipliers:

| Rehearsal | Target |
|---|---:|
| expected peak | 1x forecast peak traffic |
| elevated peak | 2x forecast peak traffic |
| severe peak | 4x forecast peak traffic |
| executive launch gate | 6x forecast peak traffic, only for high-risk launch events |

The 6x gate is not a permanent sizing target. It is a one-time confidence test
for campaign or launch risk, inspired by public AWS Distributed Load Testing
customer examples. The launch spec is still sized from verified breakpoint,
business forecast, SLO headroom, and cost.

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

## Overload And Self-Preservation

Enterprise-grade testing must prove how the system behaves when it cannot serve
all traffic. A test that only records latency until the service falls over is not
sufficient.

Gongzzang should verify these overload controls:

| Control | Expected behavior |
|---|---|
| bounded request concurrency | in-flight requests stay within a known limit |
| bounded queueing | excess requests are rejected cheaply instead of waiting until timeout |
| route priority | interactive read traffic is protected before background or low-value traffic |
| CircuitBreaker | failing dependencies stop consuming request capacity |
| retry discipline | retries do not create a self-amplifying storm |
| graceful degradation | non-critical data can be omitted or served stale when policy allows |
| recovery | latency and error rate return to baseline after overload is removed |

Route priority for overload tests:

| Priority | Route class |
|---|---|
| P0 | health/readiness, auth/session safety, critical Listing read |
| P1 | Listing detail, panel read, marker tile cache hit |
| P2 | marker tile cache miss, expensive filter/mask reads |
| P3 | writes that can return retryable errors without data loss |
| P4 | diagnostics, non-critical background refresh, bulk maintenance |

This follows the Netflix-style lesson that protecting the service is often
better than accepting every request and allowing queues to cascade into
timeouts. Gongzzang does not need Netflix's adaptive implementation on day one,
but it must measure queueing, in-flight request count, rejection rate, and
recovery behavior.

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

## Result Classification

Each run must classify the result before making a sizing recommendation:

| Classification | Meaning | Default action |
|---|---|---|
| healthy | all SLO and resource gates pass | keep as candidate baseline |
| latency breakpoint | p95/p99 fails before resource saturation is clear | inspect queueing, slow spans, DB queries |
| resource breakpoint | CPU, memory, connection, IOPS, or cache saturation appears first | tune resource owner or scale the constrained layer |
| dependency breakpoint | Platform Core, R2/CDN, Valkey, or another dependency dominates latency | enforce timeout/circuit/cache behavior before scaling API |
| data-shape breakpoint | only a region, tile, filter, or hot partition fails | fix index/projection/precompute/data distribution |
| overload-safe | some requests are rejected or degraded, but core route SLO recovers | document guard limits and autoscaling trigger |
| overload-unsafe | queue growth, retry storm, or cascading errors continue after load stops | block launch sizing until fixed |

The final launch spec can be accepted only from `healthy` or `overload-safe`
results. `overload-safe` requires explicit product acceptance of degraded or
rejected lower-priority behavior.

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
| `baseline-comparison.md` | comparison against the previous accepted baseline |

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
