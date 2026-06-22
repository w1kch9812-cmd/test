# ADR-0047: Collection Event Fabric — Kafka-Shaped Bronze-Ingestion Control Plane (Broker Deferred)

| | |
|---|---|
| Date | 2026-06-22 |
| Status | Accepted — **design now, broker DEFERRED**; this records the contract + migration ladder, not a build order |
| Scope | platform-core Bronze-ingestion collection pipeline (Catalog public-API → R2 Bronze). Cross-repo because the `collection.raw_written` contract and event-type names are a published consumer contract. |
| Owner | perfectoryinc (platform owner) |
| Governs under | [✱ Product-first](../../AGENTS.md) · [ADR-0044](./0044-bazel-transition-reconciliation.md) (no premature infra) · refines [ADR-0046](./0046-kafka-kubernetes-preliminary-design.md) (Kafka transport ladder) · [ADR-0032](./0032-eventual-consistency-strategy.md) (outbox/eventual consistency) · [ADR-0026](./0026-bronze-api-archive-r2-not-postgres-jsonb.md) (Bronze in R2) |

> This is a **preliminary (선행) design** of the *control plane* for national Bronze collection. It decides
> the topic taxonomy, event schemas, claim-check rule, idempotency/retry/DLQ model, and the ledger↔offset
> reconciliation — **all as a transport-agnostic contract** — and that we run it on the **existing
> Postgres/outbox/ledger backing now, with no Kafka broker pre-launch**. Building MSK before its trigger is
> the same "infra-before-users" trap ADR-0044 reversed. The point of designing it Kafka-shaped *now* is so
> the eventual broker swap is a wiring change, not a rewrite.

---

## Context

### Current reality (what already exists, code-confirmed in platform-core)

National collection today is **ledger-driven**, and ~90% of this fabric is already built — it is just not yet *named* as a streaming control plane:

- **Job plan/command** — the Planner (`national_data_collection_plan_compile.rs` / `national_data_collection_async/plan.rs`) writes a JSONL **execution ledger** of `LedgerEntry` rows with `status:"planned"`, each carrying `job_id`, `idempotency_key`, `collection_snapshot_id`, `compiler_input_hash_sha256`, `shard_id`, `scope_unit_id`, `request_fingerprint_sha256` (`platform-core.bronze_request_fingerprint.v1`), provider/endpoint, page window, and `request_count_estimate`.
- **Job dispatch/consume** — `select_pending_jobs` (`ledger.rs`) windows pending rows under a `request_cap` and skips already-`succeeded` job ids; `ledger_execute/.../runner.rs` executes them.
- **Raw write / claim ticket** — workers PUT raw bytes to **R2 Bronze** under the deterministic key `bronze/source=.../page=<N>/part-<M>.json` (`expand_bronze_object_keys`, `national_bronze_object_manifest.rs`) and append a `succeeded`/`job_reused` event (`events.rs`) carrying `bronze_object_key`, `storage_driver:"r2"`, `request_count`, `source_record_count`, `request_fingerprint_*`, `collection_snapshot_id`.
- **Audit** — `national_data_collection_coverage_ledger_check.rs` already computes Chaperone-class reconciliation (collected / duplicate / missing / extra / empty / late) and **double-entry checks** each evidence file's self-reported counts against the recomputed event log; it requires `missing == extra == duplicate == failed == 0` for rollout.
- **Pluggable transport** — `crates/outbox-publisher` has the `EventBroadcaster` trait (`LoggingBroadcaster`, `WebhookBroadcaster`, `CatalogEventBroadcaster`) + `OutboxWorker` (`FOR UPDATE SKIP LOCKED`, `retry_count`, quarantine) and the `catalog.outbox_quarantine` DLQ table (`failure_stage`, `failure_code`, `attempt_count`, `resolution_kind IN ('replayed','discarded','superseded')`, idempotent `ON CONFLICT … version = version+1`).
- **Per-source rate policy** — `provider-rate-policy.v1.json` + `public_provider_rate_policy.rs` (AIMD token-bucket lanes, `daily_request_budget_env`, throttle signals, `defer_without_drop`/`pause_lane`) + `provider_request_spacing.rs`.

### The Kafka-centric target

We want a single **Collection Event Fabric**: a small set of named topics, two core event schemas (`job` command + `raw_written` claim-check), and explicit idempotency/retry/DLQ/audit rules — shaped exactly as Kafka topics + partition keys + consumer offsets, so that downstream consumers (Silver / Gold / AI enrichment / search indexer / notify) can fan out from one ordered log. ADR-0046 already put Kafka on the transport ladder behind a trigger; this ADR makes the *collection control plane specifically* the place that fabric lives, and pins the wire contract so it is broker-independent.

### Claim-Check (the spine)

**Raw bytes NEVER travel on any stream.** Every message carries only a *claim ticket*: the R2 Bronze object pointer + `sha256` + `byte_size` + `record_count` + lineage. Losing a ticket costs an idempotent re-fetch, never data loss. This is the [Enterprise Integration Patterns](https://www.enterpriseintegrationpatterns.com/patterns/messaging/StoreInLibrary.html) **Claim-Check** pattern; the broader design uses the standard **transactional Outbox**, **Dead-Letter Channel**, **Competing Consumers**, and **Idempotent Receiver** EIP patterns, plus Uber's **Chaperone**-style end-to-end audit for completeness/duplication accounting.

---

## Decision

**Define the Collection Event Fabric as a transport-swappable contract — topic names, partition keys, payload schemas, compatibility rules, idempotency/retry/DLQ semantics — and run it on the existing Postgres + outbox + JSONL-ledger backing now. Do not stand up a Kafka broker (MSK) pre-launch.** Workers and the Planner depend only on the transport interface; swapping to Kafka is a wiring change with no schema, consumer, or audit change.

Two narrow traits (not one), because dispatch needs more verbs than publishing:

- **`JobBus`** — the dispatch side (Planner produces, Worker consumes): `publish_jobs`, `poll_jobs(group, max, lease)`, `ack(lease_token)`, `nack(lease_token, retryable)`. `ack` ≡ commit offset; `nack(retryable=false)` ≡ DLQ.
- **`RawWrittenSink`** — the **producer** seam: a collection worker hands its typed
  `collection.raw_written` payload to the sink when it `ack`s a job. The production sink **inserts a
  `catalog.outbox_event` row**; the **existing `OutboxWorker` + `EventBroadcaster`** then fan that row
  out to consumers unchanged. So `EventBroadcaster` is still reused — for the *fan-out* half — but it
  is **not** the producer seam itself.

> **Resolved contradiction (facets 1/2 vs 4):** facets 1–3 spoke of "the `EventBroadcaster` trait" for both directions; facet 4 correctly observed `EventBroadcaster` is publish-only and cannot express pull/lease/ack. **We adopt the two-trait split.** `JobBus` owns dispatch; `RawWrittenSink` owns producer-side emission; `EventBroadcaster` owns fan-out. Both new traits are `Arc<dyn …>`-injected, identical to how `OutboxWorker` holds `Arc<dyn EventBroadcaster>` today.
>
> **Refinement (2026-06-22, from the Slice 3-A implementation):** the producer seam is a **distinct typed trait `RawWrittenSink`**, *not* `EventBroadcaster` directly. Reason: `EventBroadcaster::publish` takes an `EventEnvelope` carrying an outbox `event_id` + `OutboxScope` that exist **only after** the outbox row is persisted, whereas the producer must emit *before* persisting (its input is the typed `CollectionRawWrittenV1`). `RawWrittenSink::emit(&CollectionRawWrittenV1)` is therefore the pre-persist producer port; its production impl persists the row and the existing `OutboxWorker`/`EventBroadcaster` path fans it out. This keeps producer-shaping out of the fan-out trait and avoids the "two `EventBroadcaster`s" the split rejected.

**The ledger remains the SSOT** for "what should be collected" and "what was collected." The bus is *only* dispatch + lease + fan-out. If the bus is wiped, the ledger + event logs fully reconstruct state (the coverage check already does exactly this). Kafka never becomes the audit source of truth (SSS pillars 3 traceability + 6 SSOT).

---

## Topic taxonomy + event schemas

### Topics (5, deliberately minimal — YAGNI: provider is a partition-key dimension and a payload field, not a topic)

| Topic | Direction | Partition key | Notes |
|---|---|---|---|
| `collection.jobs` | Planner → Workers | **`scope_unit_id`** | Command stream; one record per planned job. = ledger `status:"planned"` rows. |
| `collection.raw_written` | Workers → downstream | **`scope_unit_id`** | Claim-check event; one record per Bronze object (page/part). Fan-out point for Silver/Gold/AI/search/notify. |
| `collection.job_status` | Workers → audit | **`scope_unit_id`** | Lifecycle: `running`/`succeeded`/`failed`/`retryable`/`reused`/`empty`. Keep full history (the Chaperone late/dup trail is the product value — no compaction). |
| `collection.jobs.retry` | retry scheduler → Workers | `scope_unit_id` | Delayed re-delivery; backoff in `not_before_utc` so it doesn't head-of-line-block fresh jobs. |
| `collection.jobs.dlq` | Workers/scheduler → operator | `scope_unit_id` | Terminal; logical view over the existing `catalog.outbox_quarantine` (see Reliability). |

Naming rule: lowercase dotted `collection.<noun>[.<modifier>]`. **No env, no version in the topic name.**

### Partition key — `scope_unit_id` (resolved)

Partition by the existing canonical `scope_unit_id` (`scope:legal-dong:<sigungu><bjdong>`, `scope:sigungu-month:<lawd_cd>:<deal_ymd>`), **not** by source (~3 keys → no parallelism) and **not** by `job_id` (max parallelism but zero per-scope ordering). `scope_unit_id` gives tens of thousands of keys (abundant parallelism) **and** per-scope total order; `page_number` is the secondary order within a multi-page job. The same key across `collection.jobs` / `collection.job_status` / `collection.raw_written` co-partitions the three streams for free.

> **Resolved contradiction (facet 1 vs facets 2/4):** facet 1 chose `scope_unit_id` as the partition key; facets 2/4 floated `idempotency_key`/`request_fingerprint_sha256` as the Kafka *message key*. These are different roles. **Decision: partition key = `scope_unit_id` (ordering/locality); message dedup key = `idempotency_key` (carried in the record so redeliveries of one job land in order and dedup).** `request_fingerprint_sha256` is the *content identity* used for reuse/dedup of bytes (below), not the partition key. See OQ-1 for the one remaining nuance.

### Event-type / versioning convention

- Event types follow the shared-kernel pattern `collection.<aggregate>.<action>.v<N>` (e.g. `collection.raw_written.v1`), aligned with `catalog_v1.rs`. **Topic name drops the `.vN`** (topics are version-spanning); the payload's `schema_version` discriminates.
- Backward-compatible evolution = **add optional fields only.** Any removal/rename/semantic change = new `.v2` type + new struct variant coexisting on the same topic (mirrors `…CreatedV1`/`V2`).
- Unknown `type`/`schema_version` → route to `collection.jobs.dlq` (or `*.unknown` sink) with telemetry, never silently drop (matches the §10 Migration BLOCKER and the existing fail-closed posture).
- A **compatibility-corpus test** (frozen example JSON per version) gates schema changes — codec/struct roundtrip only, a unit test, **not a running broker**.

### `collection.job.planned.v1` (= `collection.jobs` value)

A projection of the existing `LedgerEntry` plus four command-control fields. Every field already exists on the ledger except `attempt`, `deadline_utc` (claim lease — optional now, enforced only when concurrent workers exist), `rate_budget`/`lane_id`, and `request_cap_share`. Provider-specific scope fields (`sigungu_cd`, `bjdong_cd`, `lawd_cd`, `deal_ymd`, …) ride inside a `provider_request` sub-object reused verbatim from `plan_compile.rs`. No `serviceKey`/`raw_payload` (existing forbidden-token scan applies to bus payloads).

### `collection.raw_written.v1` (= claim ticket)

One event per Bronze object (per page/part), 1:1 with a `national_bronze_object_manifest` entry, so the manifest is a materialized replay of this stream and they must be byte-compatible. Carries `claim_check { storage_driver, object_key | (last_object_key, object_count), sha256, byte_size, record_count }`, `request_fingerprint_*`, `collection_snapshot_id`, `page`, `source`, and a mandatory `lineage` block, plus `occurred_at_utc` (emit) distinct from `fetched_at_utc` (upstream fetch).

> **Resolved contradiction (facet 1 per-page vs facet 4 per-job):** **per-Bronze-object (per page/part)** is the canonical granularity — it gives exact manifest parity and finer downstream parallelism. To keep messages tiny, a single `raw_written` for a multi-page job MAY carry `(last_object_key, object_count)` and let consumers reconstruct page keys via the existing deterministic `expand_bronze_object_keys` rule, rather than inlining N keys. (Per-job-with-`object_keys[]` is rejected; it loses per-page fan-out. Volume sanity-check against national page counts is OQ-4.)

---

## Claim-Check, R2 pointer, and per-source rate-limit

### R2 Bronze key (the claim-check pointer)

Keep the existing grammar; formalize the segment order so it is parseable and `expand_bronze_object_keys` keeps working unchanged:

```
bronze/source=<source_slug>/endpoint=<endpoint_slug>/snapshot=<collection_snapshot_id>/
       scope=<scope_unit_id>/job=<job_id>/page=<NNNN>/part=<MMMM>.json
```

- `collection_snapshot_id` is the **immutability boundary** — a re-collection mints a *new* snapshot and never overwrites. Write-once is enforced by the existing `create_new(true)` semantics and the manifest's duplicate-key blocker.
- Pointer is provider-relative (manifest blocks anything not starting `bronze/source=`); `storage_driver` must be `r2` in any published manifest (`local` is dev-only).

### Integrity — the one real prerequisite

**`bronze_checksum_sha256` is currently emitted as `Some("")` on the R2 path** (`events.rs`; `bronze_result.rs` hashes on the local path but returns an empty hash on the R2 path). A claim-check without a content hash is not trustworthy. **Closing this — the worker computing and emitting a real lower-hex `sha256` for each R2 object (tee-hash the upload stream or use the R2 ETag/sha if trustworthy, OQ-5) — is the single must-build producer change in this ADR.** The manifest's existing `is_lower_sha256` gate then applies to the content hash exactly as it already does for the fingerprint.

### Lineage (mandatory)

`raw_lineage { source, endpoint_slug, fetched_at_utc, license, srid, request_count, source_record_count }` travels with the claim, satisfying AGENTS.md §8 / §10.3-14 traceability **without inlining bytes**. `srid` is required (non-null) for spatial sources (V-World cadastral/NED = `EPSG:4326`), `null` for attribute-only registers (building register, real-transaction) and per OQ confirmation for V-World land-register. `license` comes from `endpoint_catalog`.

### Per-source rate-limit / quota (three-layer enforcement, reusing the existing AIMD lanes)

Do **not** rebuild the rate policy. Bind each job to a lane and enforce in three layers:
1. **Planner sizes to budget** — `request_cap` for a pull = remaining daily budget of that lane (`daily_request_budget_env`), so the fabric structurally cannot dispatch more upstream calls than the quota allows. The existing `select_pending_jobs` greedy packing + fail-closed-if-first-job-exceeds-cap is reused.
2. **Worker token-bucket throttles in real time** — acquire from the lane's in-flight/rps window, space with `ProviderRequestSpacing`, classify responses via `is_throttle_signal`, feed `update_lane_state` (AIMD).
3. **Lane pauses + defers on exhaustion** — `on_quota_exhausted → pause_lane`; in-flight jobs flip to `defer_without_drop` (re-queued, status stays `planned`, **not** failed) so coverage can still reach `missing == 0` later. Deferral ≠ loss.

---

## Reliability (idempotency / retry / DLQ / ledger↔offset reconciliation)

### The one rule

**A job is collected iff a `job_succeeded`/`job_reused` event exists in the event log that the coverage ledger reconciled against the plan — never because a topic/offset said so.** The offset is *liveness*, not *truth*.

### Idempotency — exactly-once *effect* under at-least-once delivery (three gates)

1. **Gate 1 — reuse (skip the external call):** before fetching, consult the reuse manifest (`reuse_manifest.rs`, keyed by `request_fingerprint_sha256`). If the bytes already exist in Bronze, emit `job_reused` (0 provider quota) instead of re-fetching. This must be the **mandatory first step** of consuming a `collection.jobs` message.
2. **Gate 2 — deterministic R2 key:** keys are a pure function of `(fingerprint, page)`, so a redelivered fetch PUTs to the same key (recommend `If-None-Match: *` for a cheap no-op). At-least-once fetch → at-most-once distinct object.
3. **Gate 3 — ledger dedup accounting:** even if both gates are bypassed (racing workers), `inspect_succeeded_event` raises `duplicate_succeeded_event` and the coverage check fails closed. Duplicate *bytes* are harmless; duplicate *accounting* is a hard blocker.

### Retry + DLQ

- **Retry:** transient errors (`http_429`, `http_5xx`, `timeout`, `circuit_open`, `r2_5xx`) → `collection.jobs.retry` with `backoff = provider_spacing(provider) × 2^attempt` + jitter, capped, on the *per-source* clock. Poison (`http_400/401/403`, `schema_reject`, `auth_key_invalid`) → straight to DLQ, no wasted quota. `retry_on`/`fail_fast_on` live in `endpoint_catalog` (one SSOT); the job carries only the resolved `max_attempts`.
- **DLQ:** **reuse `catalog.outbox_quarantine` — do not build a new table.** Add one `consumer_key='collection-worker'`, one `source_outbox_table='catalog.collection_jobs'`, and one `failure_stage='fetch'` to the CHECK enums. The idempotent `ON CONFLICT … version = version+1` means a poison job redelivered N times = **one** DLQ row with rising `attempt_count`. Every `failure_message` passes the existing `safe_runner_error_message` scrubber + `FORBIDDEN_TOKENS` scan (no keys, no `raw_payload` — DLQ holds the ticket, not bytes).

> **Resolved contradiction (facet 4 OQ "separate DLQ?"):** **unify on `catalog.outbox_quarantine`.** The `(source_outbox_table, event_id, consumer_key)` key already distinguishes collection failures from broadcast failures, so they do not actually conflate; a second table is unjustified ceremony.

- **Operator actions** reuse existing indexes/states: `replayed` (re-publish the stored claim → idempotent by the three gates), `discarded` (job stays `missing` forever → correctly blocks rollout until an operator records why), `superseded` (a re-plan with a new `compiler_input_hash` obsoleted it).

### Ledger ↔ offset reconciliation (Chaperone-style audit)

`national_data_collection_coverage_ledger_check.rs` **is** Chaperone and is reused unchanged — it already emits collected / duplicate / late (`started − succeeded`) / missing / extra / empty counts per `(provider, endpoint)`, and double-entry-checks evidence vs recomputed event log. The offset's only job is liveness/lag. The single new alarm: **`planned − (succeeded ∪ in_DLQ ∪ in_flight)` with an offset advanced *past* the claim = "lost-without-trace"** → emit `collection.reconcile.gap`. `collection.raw_written` is treated by downstreams as at-least-once and deduped on `request_fingerprint_sha256` + `bronze_object_key`; if lost, re-derive from the event log.

> **Resolved open decision (facet 2 OQ-1, `job_reused` dup):** make Gate-3 **ignore a `job_reused` that follows an existing `job_succeeded` for the same `job_id + fingerprint`** (idempotent redelivery, tolerated), while still blocking **two fresh `job_succeeded`** for one `job_id`. This is a small, named rule in `inspect_succeeded_event` and is part of this ADR's accepted scope.

---

## Migration ladder (Postgres/outbox now → Kafka/MSK on trigger)

This is the ADR-0046 Kafka ladder, instantiated for the collection control plane. Same contract on every rung.

| Fabric concept | Rung 1 — pre-launch backing (ships now, 0 brokers) | Rung 2 — Kafka/MSK (swap on trigger) |
|---|---|---|
| `collection.jobs` | ledger rows `status='planned'` (the plan *is* the queue) | partitioned topic, key=`scope_unit_id`, dedup=`idempotency_key` |
| worker pull + lease | `select_pending_jobs` wrapped as `JobBus::poll_jobs`; `FOR UPDATE SKIP LOCKED` = lease | consumer group |
| `ack` / offset | append `job_succeeded`; `read_succeeded_job_ids(compiler_input_hash)` excludes it | offset commit |
| backoff / retry | `poll_interval` + `provider_request_spacing`; `collection.jobs.retry` via `not_before_utc` | retry/delay topics |
| `collection.jobs.dlq` | `catalog.outbox_quarantine` (exists) | DLQ topic + mirror to quarantine |
| `collection.raw_written` | outbox row + `OutboxWorker` + `EventBroadcaster` (exists) | topic via a `KafkaRawWrittenBroadcaster` (`EventBroadcaster` impl) |
| Chaperone audit | `coverage_ledger_check` (exists) — reads ledger, **not** topic | **unchanged** — still reads ledger, not topic |

**The swap trigger (turn on `KafkaJobBus` + `KafkaRawWrittenBroadcaster`) — adopt when ANY fires** (concrete, refining ADR-0046's rung-3 triggers for this pipeline):

1. **Throughput:** a national plan epoch's pending-job backlog or sustained dispatch rate exceeds what one Postgres-polled worker drains within the plan's freshness SLO **and** within the V-World/data.go.kr daily quota window.
2. **Multi-consumer fan-out:** ≥2 *independent* real-time consumers of `collection.raw_written` need their own offsets/replay (e.g. AI enrichment + search indexer running concurrently and falling behind), beyond what outbox-table fan-out + per-consumer cursors can serve.
3. **Replay/retention:** need to replay `raw_written` history to a *new* consumer without re-running collection (Kafka log retention beats reconstructing from JSONL).

**What changes at the trigger: only the adapter wiring** — add `KafkaJobBus` (impl `JobBus`) and `KafkaRawWrittenBroadcaster` (impl `EventBroadcaster`), bind topics (`collection.jobs` key=`scope_unit_id`; `collection.raw_written` key=`scope_unit_id`; `collection.jobs.dlq`). Planner / Worker / Bronze-manifest / coverage-check code: **unchanged**. Run it **managed (MSK / Redpanda Cloud) before ever self-hosting brokers** (per ADR-0046). The request-cap *quota gate* must be re-homed into the consumer as a rate-limiter post-cutover — it must not be lost in translation (OQ-2).

---

## How this refines ADR-0046

ADR-0046 deferred Kafka generally and put it on a transport ladder behind triggers. ADR-0047 **does not reverse that defer** — it sharpens it for the Bronze-ingestion pipeline:

- ADR-0046 framed Kafka purely as a `WebhookBroadcaster → SqsBroadcaster → KafkaBroadcaster` *publish-side* ladder for Catalog/Workforce domain events. ADR-0047 adds the missing **dispatch side** (`JobBus` with pull/lease/ack) — collection needs a job queue, not just fan-out — and names **Kafka as the eventual collection control-plane**, designed now so the contract is fixed.
- ADR-0046's rung-3 Kafka triggers (replay, many consumers, throughput) are **instantiated with concrete, measurable conditions for collection** (the three triggers above).
- The **broker stays deferred.** ADR-0047's rung 1 (Postgres/outbox/ledger) is not a stopgap — at 0 users it is the correct YAGNI choice, and it is the same backing ADR-0046 already endorsed (`EventBroadcaster` over outbox). Nothing here advances Kafka adoption; it only fixes the seam so adoption stays a config flip.

This ADR **does not** alter ADR-0046's Kubernetes ladder.

---

## Consequences

- **Positive:** one fixed, broker-independent contract for national collection → agents/humans stop re-deriving topic/partition/idempotency decisions per session. The eventual MSK swap is a wiring change, not a rewrite. The ledger stays the single audit SSOT regardless of transport (pillars 3 + 6). ~90% is already built; net new work is small and product-visible.
- **Net new work (build-now):** (1) the **real R2 content hash** (the one true prerequisite); (2) the two narrow traits (`JobBus`, reusing `EventBroadcaster` as `RawWrittenSink`) wrapping existing pull/ack/DLQ; (3) widen two `outbox_quarantine` CHECK enums + add `consumer_key='collection-worker'`; (4) wire the reuse-manifest gate as the mandatory first consume step; (5) the Gate-3 `job_reused`-after-`job_succeeded` tolerance rule; (6) add `raw_lineage` + `lane_id` fields; (7) `request_cap = remaining lane budget`.
- **Negative / honest limitations:** the trait seam carries a small upfront cost even if Kafka never lands (≈ one trait + one adapter; acceptable). Pre-launch leasing is tx-scoped (`SKIP LOCKED`) — concurrent multi-worker collection needs an explicit `lease_owner`/`lease_until` and a shared lane-budget counter *before* it is enabled (OQ-3). Per-page `raw_written` is N× message volume for multi-page jobs (mitigated by the `(last_object_key, object_count)` compaction; sanity-check pending, OQ-4).
- **Affected:** platform-core `services/outbox-publisher/src/national_data_collection_async/*`, `national_bronze_object_manifest.rs`, `national_data_collection_ledger_execute/support/{bronze_result,reuse_manifest,job_outcome}.rs`, `national_data_collection_coverage_ledger_check.rs`, `crates/outbox-publisher/{broadcaster,worker,lineage}.rs`, `migrations/*outbox_quarantine*`. gongzzang consumes only the published `collection.raw_written` event-type names + schema (boundary: the fabric is Platform-Core-private; only event-type names are a shared consumer contract — OQ-6).

---

## NOT building now (product-first guard, ADR-0044 / AGENTS.md ✱ compliance)

- **No Kafka broker, no MSK, no Redpanda** — `KafkaJobBus`/`KafkaRawWrittenBroadcaster` are deferred behind the named trigger; until then they don't exist (no feature-flagged dead broker scaffolding).
- **No new DLQ table, no new audit machine, no new registry/ratchet/evidence-bundle** — reuse `outbox_quarantine`, `coverage_ledger_check`, and the AIMD rate lanes. The only schema delta is widening two CHECK enums.
- **No new PowerShell** (AGENTS.md rule 5) — all logic is Rust; the compatibility-corpus test is a unit test, not infra.
- **No per-provider topics, no `deadline_utc`/lease enforcement, no shared lane-budget counter** until concurrent workers actually exist (define optional fields now, enforce on demand).
- **Every guard answers "what bug does it stop?":** dup-accounting blocker → silent double-count of quota/rows; missing-job blocker → claiming national coverage we don't have; lost-without-trace alarm → a consumer ate a command and produced no data; secret scrubber → API-key leakage onto the wire; content-hash gate → serving a corrupt/wrong Bronze object downstream.

---

## Open decisions for the owner

> **Pre-implementation status (2026-06-22):** OQ-2, OQ-5, OQ-6 are **RESOLVED** below — they had
> to close before Bronze collection is built, because each one shapes the worker/outbox code. OQ-1,
> OQ-3, OQ-4, OQ-7, OQ-8 stay on their **recommended default** until their own trigger fires; they
> do not block the first implementation.

1. **OQ-1 — Kafka message key on cutover.** Confirmed: partition key = `scope_unit_id`. Remaining nuance: should the *record key* be `idempotency_key` (per-job dedup ordering) given partitioning is by `scope_unit_id`? (Kafka couples key→partition; we'd set partition explicitly or accept `scope_unit_id` as both.) Recommend `scope_unit_id` as the Kafka key and `idempotency_key` as a header-level dedup token.
2. **OQ-2 — quota gate post-cutover. RESOLVED (2026-06-22).** The `request_cap`/daily-budget gate currently lives in `select_pending_jobs` (DB-side, naturally serialized while there is one dispatcher). **Principle adopted:** on Kafka cutover the quota gate moves into a *consumer-side rate limiter* — it must never be approximated by partition count or broker config. Re-homing the gate into the consumer is a **required pre-cutover task** recorded here, not a free decision at cutover time. Pre-Kafka, it stays in `select_pending_jobs` unchanged.
3. **OQ-3 — pre-launch lease model.** Keep tx-scoped `SKIP LOCKED` (single-worker, simplest) vs. add `lease_owner`/`lease_until` + a shared Postgres lane-budget counter now to enable multi-worker without Kafka? Recommendation: stay tx-scoped until trigger #1/#2.
4. **OQ-4 — `raw_written` volume.** Per-page granularity is chosen; confirm acceptable after a sanity-check against national page counts, or default multi-page jobs to the `(last_object_key, object_count)` compact form.
5. **OQ-5 — R2 content hash source. RESOLVED (2026-06-22).** The worker **tee-hashes the upload stream** (compute `sha256` over the bytes as they are streamed to R2) and that producer-computed digest is the integrity hash in `raw_written`. We **do not** trust the R2/S3 `ETag`: for multipart uploads the ETag is an MD5-of-part-MD5s, not a content `sha256`, so it is neither a stable nor a verifiable whole-object digest. Hashing is therefore provider-independent and survives any future move off R2. Cost: the worker needs a streaming hasher in the upload path (cheap; one pass, no extra read).
6. **OQ-6 — boundary publication. RESOLVED (2026-06-22).** The fabric stays **Platform-Core-private**. The **only** thing published as the gongzzang/dawneer consumer contract is the `collection.raw_written` event-type name(s) + schema (the existing `shared-kernel` catalog event surface). The internal topics — `collection.jobs`, `collection.job_status`, `.retry`, `.dlq` — and the `JobBus` trait are **not** a public contract and must not leak into `shared-kernel` or any consumer's allowed-call matrix. This keeps the dispatch mechanism swappable (Postgres → Kafka) without a cross-repo contract change.
7. **OQ-7 — SRID for V-World land-register (`ladfrlList`).** Confirm `srid = null` (attribute-only) rather than inheriting cadastral's `EPSG:4326`.
8. **OQ-8 — DLQ replay authorization.** Replay re-spends provider quota — should operator-initiated replay require the same rollout-approval gate (`national_data_collection_rollout_approval_check.rs`) as a fresh run, or be exempt?

---

## References

- Refines [ADR-0046](./0046-kafka-kubernetes-preliminary-design.md) (Kafka transport ladder + triggers). Builds on [ADR-0026](./0026-bronze-api-archive-r2-not-postgres-jsonb.md) (Bronze in R2), [ADR-0032](./0032-eventual-consistency-strategy.md) (outbox/eventual consistency), [ADR-0039](./0039-service-owned-lakehouse-registry-integration.md) (service-owned lakehouse). Governed by [ADR-0044](./0044-bazel-transition-reconciliation.md) + [AGENTS.md](../../AGENTS.md) ✱ product-first.
- Patterns: EIP Claim-Check / Transactional Outbox / Dead-Letter Channel / Competing Consumers / Idempotent Receiver; Uber Chaperone (Kafka end-to-end audit).
- platform-core code: `services/outbox-publisher/src/national_data_collection_async/{ledger,events,plan}.rs`, `national_bronze_object_manifest.rs`, `national_data_collection_ledger_execute/support/{runner,bronze_result,reuse_manifest,job_outcome}.rs`, `national_data_collection_coverage_ledger_check.rs`, `public_provider_rate_policy.rs` + `provider_request_spacing.rs` + `docs/catalog/provider-rate-policy.v1.json`, `crates/outbox-publisher/src/{broadcaster,worker,lineage}.rs`, `crates/shared-kernel/src/events/catalog_v1.rs`, `migrations/20260519000001_postgis_mirror_dlq.sql`.
