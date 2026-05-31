# Load Test Result

## Tested Spec

- Plan: `docs/superpowers/plans/2026-05-29-load-test-capacity-sizing.md`
- Scenario registry: `tests/load/scenarios.v1.json`
- Scenario script: `tests/load/scenarios/api-read-mix.js`
- Launcher: `scripts/load/run-k6.ps1`
- Normalizer: `scripts/load/normalize-k6-summary.ps1`

This run verifies the evidence pipeline for the Gongzzang load-test harness. It
does not establish a production launch capacity spec.

A real perf/staging operator run remains required before any launch sizing
claim.

## Scenario

- Scenario: `api-read-mix`
- Profile: `smoke`
- Environment: `local`
- Target: `http://127.0.0.1:3000`
- Started at: `2026-05-29T14:29:20.7723653+09:00`
- Finished at: `2026-05-29T14:29:36.1812693+09:00`
- k6 exit code: `99`

## Result Classification

Classification: `error breakpoint`

The local target was unavailable or not accepting the expected requests. The
run is useful as a harness smoke test because it produced the required evidence
files, but it must not be used for launch sizing.

## Capacity Curve

No capacity curve is accepted from this run. The smoke profile used `2` target
RPS for `15s`, and the target returned connection-level failures.

Observed normalized metrics:

| Metric | Value | Limit |
| --- | ---: | ---: |
| `http_req_duration` p95 | `0.000 ms` | `300 ms` |
| `http_req_duration` p99 | missing | `1000 ms` |
| `http_req_failed` rate | `1.000` | `0.01` |

## First Bottleneck

The first bottleneck is target availability, not application capacity. The
normalizer reported:

- `http_req_failed` rate breached the SLO.
- `http_req_duration` percentile metrics are missing.

## Recommendation

Fix target availability before using this scenario for launch sizing. The next
valid sizing run should point at a running local staging or perf endpoint and
must preserve the same evidence files under `target/audit/load-tests`.

## Upgrade Trigger

Upgrade or scale decisions are explicitly blocked until a successful `baseline`
or controlled `stress` run exists against an approved non-production target.
This smoke result only proves that the harness writes evidence and classifies
failed targets.

## Evidence Path

```text
target/audit/load-tests/2026-05-29/local/api-read-mix/20260529T142920+0900
```

Required evidence files were present:

- `run.json`
- `spec.json`
- `thresholds.json`
- `k6-summary.json`
- `bottleneck.md`
- `recommendation.md`
- `baseline-comparison.md`
