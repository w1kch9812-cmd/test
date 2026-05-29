# Load Scenarios

These k6 scenarios are operator tooling for perf/staging capacity discovery.
They are not imported by `apps/`, `services/`, `crates/`, or `packages/`.

Run through `scripts/load/run-k6.ps1` so every run writes evidence.

Example:

```powershell
./scripts/load/run-k6.ps1 -Scenario api-read-mix -TargetBaseUrl https://perf.gongzzang.internal -Profile smoke
```
