# Load Testing

## Purpose

This manual defines how Gongzzang operators run k6 load tests for perf and
staging capacity discovery. Load tests are evidence-producing operator tooling,
not application runtime dependencies.

## Safety Rules

- Do not run stress, spike, or soak tests against production user traffic paths.
- Do not run tests that consume VWorld or OpenDataPortal quota from Gongzzang.
- Do not test with production PII.
- Do not log Authorization, Cookie, Set-Cookie, Platform Core service tokens, or webhook secrets.
- Do not claim a launch spec without evidence under `target/audit/load-tests`.

## Enterprise Gates

Every run must use `scripts/load/run-k6.ps1` so evidence is written under
`target/audit/load-tests`. Static CI checks verify that load scenarios,
profiles, scripts, and evidence schema files stay in the approved locations.

Launch readiness requires the scenario registry, generated evidence, result
classification, and SLO comparison to agree. Missing evidence is a failed gate.

Targets are allowlisted, not just denylisted. The wrapper accepts only
`perf.gongzzang.internal`, `staging.gongzzang.internal`, local loopback hosts for
`local` and `ci`, plus explicit hostnames supplied by the load-test runner through
`LOAD_APPROVED_TARGET_HOSTS`. The wrapper rejects URL paths, credentials, query
strings, fragments, and production `gongzzang.com` hosts before k6 starts.

The scenario registry `maxSafeRps` is an executable cap. A selected profile whose
`LOAD_RPS` exceeds the scenario cap fails before k6 starts.

Authenticated read scenarios use `LOAD_AUTH_BEARER_TOKEN` from the runner
environment when present. The wrapper passes this value to k6 but does not write
it to `run.json`, `spec.json`, logs, or generated reports.

## Run Types

- `smoke`: short validation that the scenario, target, credentials, and evidence
  writer are working.
- `baseline`: representative read path load for normal capacity measurement.
- `stress`: controlled non-production search for upper capacity limits.
- `spike`: controlled non-production burst behavior validation.
- `soak`: controlled non-production long-running stability validation.

## Scenario Matrix

| Scenario | Purpose | Max safe RPS |
| --- | --- | ---: |
| `api-read-mix` | Mixed API read capacity discovery. | 50 |
| `map-marker-mix` | Listing marker tile hit and miss behavior. | 50 |
| `capacity-stress` | Non-production capacity ceiling discovery. | 800 |
| `platform-core-events` | Platform Core event consumer path validation. | 50 |

## Evidence

Each run must preserve the scenario id, profile, target base URL, started and
finished timestamps, k6 summary, threshold output, SLO comparison, and result
classification. Evidence belongs under `target/audit/load-tests` and must be
reviewable without rerunning the test.

## Result Classification

- `pass`: all thresholds and SLO gates pass with complete evidence.
- `warn`: the run completes with non-blocking concerns that need review before
  launch use.
- `fail`: any threshold fails, evidence is incomplete, safety rules are broken,
  or the target is not approved for the selected run type.
