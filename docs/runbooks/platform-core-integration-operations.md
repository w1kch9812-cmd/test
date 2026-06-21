# Platform Core Integration Operations Runbook

## Scope

This runbook covers Gongzzang runtime calls to Platform Core and Platform Core
webhook delivery into Gongzzang.

Policy SSOT:
`docs/architecture/platform-integration/operations-policy.v1.json`

## Required Telemetry

Every integration span or log event must include the non-secret routing context:

- `service.name`
- `peer.service`
- `http.request.method`
- `url.path`
- `platform_integration.call_id`
- `platform_integration.policy_id`
- `platform_integration.direction`
- `platform_integration.decision`
- `correlation_id`

Webhook events must also include:

- `platform_core.event_id`
- `platform_core.event_type`

Never log service tokens, webhook secrets, cookies, or authorization headers.

## SLOs

Platform Core catalog reads:

- Availability: 99.9%
- p95 latency: 300 ms
- p99 latency: 1000 ms
- Timeout: 5000 ms

Platform Core webhook receiver:

- Availability: 99.9%
- p95 latency: 250 ms
- p99 latency: 1000 ms
- Duplicate event acknowledgement: 100%
- Dead-letter alert threshold: 1 event

## Alerts

`platform_core_catalog_read_slo_burn`

- Page the owner when error budget burn or latency exceeds the SLO window.
- Check Platform Core health, network egress, and recent deploys first.
- If the circuit is open, keep serving degraded responses rather than adding
  direct database access.

`platform_core_catalog_circuit_open`

- Page the owner when the circuit breaker opens.
- Confirm whether failures are timeouts, 429s, or 5xx responses.
- Do not bypass the breaker with ad hoc clients.

`platform_core_webhook_dead_letter_or_latency`

- Page the owner when a poison event reaches the dead-letter path or receiver
  latency exceeds SLO.
- Preserve the event id, event type, and correlation id.
- Fix schema compatibility before replay.

`platform_core_webhook_replay_surge`

- Create an operations ticket when duplicate event rate spikes.
- Treat it as a publisher retry or replay investigation, not a reason to apply
  side effects again.

## Anchor Artifact Import Recovery

Anchor snapshot events are durable in `platform_core_event_inbox`. The importer
uses `PLATFORM_CORE_EVENT_ID` to mark an event `processing`, import the
immutable artifact into `parcel_marker_anchor`, refresh affected
`listing_marker_projection` rows, and then mark the event `processed` or
`failed`.

If an importer process exits while the event is `processing`, rerun the importer
with the same `PLATFORM_CORE_EVENT_ID` or run the importer without local
artifact path variables to process a pending inbox batch. Batch mode reads
`pending_import` and `processing` anchor snapshot events from
`platform_core_event_inbox`, with
`PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT` defaulting to 10 and capped at 100.
When local artifact path environment variables are absent, the importer loads
the stored event payload, fetches `artifact_manifest_url`, verifies
`artifact_checksum_sha256`, resolves manifest object keys relative to the
manifest URL, and imports the fetched JSONL objects. A processing event is
intentionally claimable again, and the importer holds a PostgreSQL advisory lock
derived from the event id so two workers cannot import the same event
concurrently. The lock is released automatically when the process connection
dies. Batch mode skips already locked events but exits failed if any importable
event fails.

Inspect pending or interrupted anchor imports:

```sql
select event_id, status, anchor_snapshot_id, source_geometry_version, received_at,
       processed_at, failed_at, failure_reason
from platform_core_event_inbox
where event_type = 'catalog.parcel_marker_anchor.snapshot.published.v1'
  and status in ('pending_import', 'processing', 'failed')
order by received_at asc;
```

Retry a transient failure only after verifying the event payload's
`artifact_manifest_url`, `artifact_checksum_sha256`, and `published_at` match
the Platform Core release record. Do not edit `parcel_marker_anchor` or
`listing_marker_projection` directly.

## Listing Marker Freshness Operations

Gongzzang listing markers compose runtime visibility as:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

Platform Core owns PNU anchor source data. Gongzzang owns listing semantics,
projection, delta logs, tombstone logs, and dirty-tile rebuild decisions.

Watch these metrics from `/internal/metrics`:

- `gongzzang_listing_marker_dirty_tiles_pending`
- `gongzzang_listing_marker_dirty_tile_oldest_age_seconds`
- `gongzzang_listing_marker_tombstones_active`
- `gongzzang_listing_marker_deltas_active`

If tombstones or deltas grow unexpectedly, inspect `listing_marker_dirty_tile_queue`,
`listing_marker_tombstone_log`, and `listing_marker_delta_log`. Do not bypass this by adding
listing-owned latitude/longitude or public `bbox` marker APIs.

## Load And Fault Verification

Required tests:

- `webhook_duplicate_burst_ack` proves duplicate bursts acknowledge without
  repeated side effects.
- `webhook_dead_letter_poison_event` proves invalid events enter a dead-letter
  path instead of repeatedly failing.
- `anchor_import_processing_reclaim` proves a processing anchor import can be
  safely reclaimed for retry after a worker exit.
- `catalog_circuit_breaker_timeout_fault` proves timeouts record failures.
- `catalog_circuit_breaker_open_fault` proves an open circuit blocks calls.

Production readiness requires these tests plus the platform-integration policy
contract in `docs/architecture/platform-integration/index.v1.json` to stay
intact.
