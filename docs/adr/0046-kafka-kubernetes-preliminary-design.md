# ADR-0046: Kafka & Kubernetes — Preliminary Design (Deferred Until Triggers)

| | |
|---|---|
| Date | 2026-06-22 |
| Status | Accepted — **both DEFERRED**; this records the trigger + migration path, not a build order |
| Scope | cross-repo event transport + deployment runtime (gongzzang · platform-core · dawneer) |
| Owner | perfectoryinc (platform owner) |
| Governs under | [✱ Product-first](../../AGENTS.md) · [ADR-0044](./0044-bazel-transition-reconciliation.md) (no premature infra) · [sss-charter.md](../sss-charter.md) B-2 reliability |

> This is a **preliminary (선행) design**: it decides *when, why, and how* we would adopt Kafka and
> Kubernetes — and that we do **not** build either now. Building either before its trigger is the same
> "infra-before-users" trap ADR-0044 reversed. Pre-launch (0 users), neither earns its operational cost.

## Context (current reality, 2026-06-22)

- **Eventing:** Catalog/Workforce changes are written to an **outbox table**; `OutboxWorker`
  (`crates/outbox-publisher`) polls it and publishes through a **pluggable `EventBroadcaster` trait**.
  The live adapter is `WebhookBroadcaster` (HTTP fan-out to the gongzzang/dawneer consumers); a
  `LoggingBroadcaster` exists for dev. The outbox already gives at-least-once delivery + per-aggregate
  ordering. **A new transport is an adapter, not a rewrite.**
- **Deployment:** there is **no production runtime yet** — release artifacts are `cargo build --release`
  binaries; there are **0 Dockerfiles**; `infrastructure/` (Pulumi) provisions no compute (ECS/EKS/EC2)
  yet; local dev uses `docker-compose` (Postgres). ~2–3 services per repo.
- **Scale:** pre-launch, 0 users, 3-service architecture ([ADR-0030](./0030-three-service-architecture.md)).

## Decision

1. **Defer Kafka and Kubernetes.** Neither is built pre-launch. Keep Outbox→WebhookBroadcaster for
   events; deploy via the simplest runtime that works when we first deploy (see ladders below).
2. **Adopt only on a named trigger** (below). No "선행 구축" — preliminary *design* only.
3. **Prefer the cheaper rung first.** Kafka and K8s are the *top* rungs of their ladders, reached only
   when the rungs below genuinely cannot meet a measured need.
4. **The architecture is already positioned for both** (pluggable broadcaster; stateless binaries), so
   waiting costs us nothing — adoption stays low-friction whenever a trigger fires.

## Kafka — transport ladder

**Ladder (adopt the lowest rung that meets the need):**
1. **WebhookBroadcaster (current)** — direct HTTP fan-out. Fine for a few known consumers.
2. **Managed queue/topic — AWS SQS/SNS** — durable, zero-ops fan-out; a `SqsBroadcaster`/`SnsBroadcaster`
   adapter. First step when webhook delivery reliability/backpressure becomes the pain.
3. **Kafka (or Redpanda)** — only when **log/replay semantics** are the actual requirement.

**Triggers to reach rung 3 (Kafka):**
- Need **durable replay / new-consumer backfill** from the event log (SQS/SNS can't replay history).
- **Many** consumers or high-throughput fan-out where per-partition ordering + consumer groups matter.
- Stream processing / CDC pipelines that consume the same ordered log.

**Path when triggered:** implement `EventBroadcaster for KafkaBroadcaster` (outbox stays the source of
truth → publishes to a topic); consumers switch from the webhook endpoint to a Kafka consumer group.
No domain or outbox rewrite. Run it **managed** (MSK / Redpanda Cloud) before ever self-hosting brokers.

**Do NOT** stand up Kafka for 3 known consumers pre-launch — its brokers/partitions/KRaft ops cost
dwarfs the benefit; webhook (or SQS/SNS) covers it.

## Kubernetes — runtime ladder

**Ladder (adopt the lowest rung that meets the need):**
1. **Dockerfile per service** — the prerequisite for *any* container runtime. Do this first, when we
   actually deploy. (Cheap, unlocks every rung above.)
2. **Managed container runtime — AWS ECS Fargate or App Runner** (Pulumi-provisioned). No cluster to
   operate; autoscaling + rollout built in. Expected home for a long time.
3. **Kubernetes (EKS)** — only when fine-grained orchestration the managed runtime can't do is needed.

**Triggers to reach rung 3 (K8s):**
- Many services + need for advanced scheduling / autoscaling / self-healing / service mesh that
  Fargate/App Runner cannot express.
- Operational requirements (multi-tenant isolation, complex networking, GPU/batch scheduling) beyond
  managed runtimes.

**Path when triggered:** containers already exist (rung 1), so EKS adoption is provisioning + manifests
via Pulumi, not an app rewrite.

**Do NOT** stand up EKS pre-launch — a cluster for ~3 services and 0 users is pure operational burden
(upgrades, security, node ops) with no payoff. Fargate/App Runner is the right first production runtime.

## Consequences

- One clear forward story for both, with explicit triggers — humans/agents stop debating "Kafka now?".
- Zero new ops burden pre-launch; engineering stays on the data/product mainline (Bronze ingestion).
- Low switching cost preserved: events are adapter-swappable; services are stateless binaries → containers.
- Honest limitation: when we DO first deploy, expect to write Dockerfiles + pick Fargate/App Runner —
  that work is **not** done here (this ADR only sets the direction).

## Re-evaluation triggers (revisit this ADR when any fires)

- Webhook delivery reliability or throughput becomes a measured incident source → consider SQS/SNS, then Kafka.
- A second+ team/service needs to replay or independently consume the event history → Kafka.
- Service count or orchestration needs outgrow Fargate/App Runner → EKS.
- Until then: **no Kafka, no Kubernetes, no `MODULE`-style preemptive scaffolding.**

## References

- [ADR-0030](./0030-three-service-architecture.md) three-service architecture; [ADR-0032](./0032-eventual-consistency-strategy.md) eventual consistency (outbox).
- [ADR-0044](./0044-bazel-transition-reconciliation.md) product-first / no premature infra; [AGENTS.md](../../AGENTS.md) ✱ product-first.
- `crates/outbox-publisher` (platform-core): `EventBroadcaster` trait + `WebhookBroadcaster`.
