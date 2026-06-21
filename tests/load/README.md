# Load Scenarios

These k6 scenarios are operator tooling for perf/staging capacity discovery.
They are not imported by `apps/`, `services/`, `crates/`, or `packages/`.

Run each scenario with `k6 run --summary-export` and write the evidence under
`target/audit/load-tests` (see `docs/testing/load.md`).

Example:

```bash
k6 run --summary-export target/audit/load-tests/.../k6-summary.json \
  tests/load/scenarios/api-read-mix.js
```

Runs require an approved target host. Add non-default private perf hosts on the
load runner through `LOAD_APPROVED_TARGET_HOSTS`, using comma-separated hostnames
without scheme, path, port, query, or credentials.

For authenticated API read paths, set `LOAD_AUTH_BEARER_TOKEN` in the runner
environment. Do not put bearer tokens in workflow inputs or committed files.

For marker runs, set `LOAD_FILTER_HASH` and optionally `LOAD_FILTER_HASH_MISS`
to known fixture hashes from the perf dataset. The default miss path reuses the
same valid hash and changes only the requested tile.
