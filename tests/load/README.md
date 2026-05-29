# Load Scenarios

These k6 scenarios are operator tooling for perf/staging capacity discovery.
They are not imported by `apps/`, `services/`, `crates/`, or `packages/`.

Run through `scripts/load/run-k6.ps1` so every run writes evidence.

Example:

```powershell
./scripts/load/run-k6.ps1 -Scenario api-read-mix -TargetBaseUrl https://perf.gongzzang.internal -Profile smoke
```

The wrapper requires an approved target host. Add non-default private perf hosts
on the load runner through `LOAD_APPROVED_TARGET_HOSTS`, using comma-separated
hostnames without scheme, path, port, query, or credentials.

For authenticated API read paths, set `LOAD_AUTH_BEARER_TOKEN` in the runner
environment. Do not put bearer tokens in workflow inputs or committed files.
