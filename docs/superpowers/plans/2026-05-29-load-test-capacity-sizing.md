# Load Test Capacity Sizing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an enterprise-grade load-test harness that uses the sizing research spec as the baseline test specimen, discovers breakpoints, captures evidence, and produces launch-spec recommendations.

**Architecture:** Keep load generation outside the runtime products. Add k6 scripts under `tests/load`, PowerShell evidence wrappers under `scripts/load`, CI guardrails under `scripts/ci`, and an operator manual under `docs/testing/load.md`. Use self-hosted or operator-provided k6; do not add npm/cargo dependencies or runtime SDKs.

**Tech Stack:** k6 JavaScript, PowerShell 7, GitHub Actions manual workflow on self-hosted load-test runners, CloudWatch/OTel evidence files, markdown docs.

---

## File Structure

- Create `docs/testing/load.md`: operating manual, safety rules, runbook, result interpretation.
- Create `tests/load/README.md`: scenario inventory and local usage.
- Create `tests/load/scenarios.v1.json`: scenario registry and SLO thresholds.
- Create `tests/load/lib/env.js`: k6 environment parsing and production-target safety.
- Create `tests/load/lib/http.js`: k6 HTTP helpers with response checks and redaction-safe tags.
- Create `tests/load/scenarios/api-read-mix.js`: Listing/panel/Platform Core read mix.
- Create `tests/load/scenarios/map-marker-mix.js`: marker tile/count/filter/mask mix.
- Create `tests/load/scenarios/capacity-stress.js`: staged breakpoint discovery.
- Create `tests/load/scenarios/platform-core-events.js`: signed webhook burst and duplicate replay mix.
- Create `scripts/load/run-k6.ps1`: controlled k6 launcher that writes evidence paths.
- Create `scripts/load/normalize-k6-summary.ps1`: converts k6 JSON output into required evidence files.
- Create `scripts/ci/check-load-test-assets.ps1`: static guardrail for required assets and unsafe targets.
- Create `scripts/ci/check-load-test-assets.tests.ps1`: guardrail tests.
- Create `.github/workflows/load-test-capacity.yml`: manual self-hosted workflow for controlled runs.

## Task 1: Guardrail First

**Files:**
- Create: `scripts/ci/check-load-test-assets.tests.ps1`
- Create: `scripts/ci/check-load-test-assets.ps1`

- [ ] **Step 1: Write the failing guardrail tests**

Create `scripts/ci/check-load-test-assets.tests.ps1`:

```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-load-test-assets.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path (Join-Path $RepoRoot "target\check-load-test-assets-tests") ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File([string] $Root, [string] $RelativePath, [string] $Content) {
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker([string] $Root) {
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
    [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($output -join [Environment]::NewLine) }
}

function Write-MinimalLoadAssets([string] $Root, [switch] $UnsafeProductionTarget) {
    $target = if ($UnsafeProductionTarget) { "https://gongzzang.com" } else { "https://perf.gongzzang.internal" }
    Write-File $Root "docs\testing\load.md" "# Load Testing`n"
    Write-File $Root "tests\load\README.md" "# Load Scenarios`n"
    Write-File $Root "tests\load\scenarios.v1.json" @"
{
  "schemaVersion": "gongzzang.load.scenarios.v1",
  "defaultTargetBaseUrl": "$target",
  "scenarios": [
    {"id":"api-read-mix","file":"tests/load/scenarios/api-read-mix.js","maxSafeRps":50},
    {"id":"map-marker-mix","file":"tests/load/scenarios/map-marker-mix.js","maxSafeRps":50},
    {"id":"capacity-stress","file":"tests/load/scenarios/capacity-stress.js","maxSafeRps":800},
    {"id":"platform-core-events","file":"tests/load/scenarios/platform-core-events.js","maxSafeRps":50}
  ]
}
"@
    foreach ($file in @(
        "tests\load\lib\env.js",
        "tests\load\lib\http.js",
        "tests\load\scenarios\api-read-mix.js",
        "tests\load\scenarios\map-marker-mix.js",
        "tests\load\scenarios\capacity-stress.js",
        "tests\load\scenarios\platform-core-events.js",
        "scripts\load\run-k6.ps1",
        "scripts\load\normalize-k6-summary.ps1",
        ".github\workflows\load-test-capacity.yml"
    )) {
        Write-File $Root $file "asset"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-MinimalLoadAssets $okRoot
    $ok = Invoke-Checker $okRoot
    if ($ok.ExitCode -ne 0) { throw "expected ok fixture to pass: $($ok.Output)" }

    $unsafeRoot = Join-Path $TempRoot "unsafe"
    Write-MinimalLoadAssets $unsafeRoot -UnsafeProductionTarget
    $unsafe = Invoke-Checker $unsafeRoot
    if ($unsafe.ExitCode -eq 0) { throw "expected production target fixture to fail" }
    if (!$unsafe.Output.Contains("defaultTargetBaseUrl must not be production")) {
        throw "expected production target error, got: $($unsafe.Output)"
    }

    Write-Output "check-load-test-assets-tests-ok"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
```

- [ ] **Step 2: Run the tests and confirm they fail because the checker is missing**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.tests.ps1`

Expected: non-zero exit mentioning `check-load-test-assets.ps1` cannot be found.

- [ ] **Step 3: Implement the checker**

Create `scripts/ci/check-load-test-assets.ps1`:

```powershell
param([string] $Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-File([string] $RelativePath) {
    $path = Join-Path $Root $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing required load-test asset: $RelativePath" }
}

function Read-Json([string] $RelativePath) {
    Get-Content -LiteralPath (Join-Path $Root $RelativePath) -Raw | ConvertFrom-Json
}

$requiredFiles = @(
    "docs/testing/load.md",
    "tests/load/README.md",
    "tests/load/scenarios.v1.json",
    "tests/load/lib/env.js",
    "tests/load/lib/http.js",
    "tests/load/scenarios/api-read-mix.js",
    "tests/load/scenarios/map-marker-mix.js",
    "tests/load/scenarios/capacity-stress.js",
    "tests/load/scenarios/platform-core-events.js",
    "scripts/load/run-k6.ps1",
    "scripts/load/normalize-k6-summary.ps1",
    ".github/workflows/load-test-capacity.yml"
)
$requiredFiles | ForEach-Object { Assert-File $_ }

$registry = Read-Json "tests/load/scenarios.v1.json"
if ($registry.schemaVersion -ne "gongzzang.load.scenarios.v1") { throw "scenario registry schemaVersion mismatch" }
if ([string] $registry.defaultTargetBaseUrl -match "gongzzang\.com|api\.gongzzang\.com") {
    throw "defaultTargetBaseUrl must not be production"
}
foreach ($scenario in @($registry.scenarios)) {
    Assert-File ([string] $scenario.file)
    if ([int] $scenario.maxSafeRps -lt 1) { throw "scenario maxSafeRps must be positive: $($scenario.id)" }
}

Write-Output "check-load-test-assets-ok scenarios=$(@($registry.scenarios).Count)"
```

- [ ] **Step 4: Run the guardrail tests and confirm they pass**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.tests.ps1`

Expected: `check-load-test-assets-tests-ok`.

- [ ] **Step 5: Commit**

Run:

```powershell
git add scripts/ci/check-load-test-assets.ps1 scripts/ci/check-load-test-assets.tests.ps1
git commit -m "test: add load-test asset guardrail"
```

## Task 2: Docs And Scenario Registry

**Files:**
- Create: `docs/testing/load.md`
- Create: `tests/load/README.md`
- Create: `tests/load/scenarios.v1.json`

- [ ] **Step 1: Create the operating manual**

Create `docs/testing/load.md` with sections for Purpose, Safety Rules, Enterprise Gates, Run Types, Scenario Matrix, Evidence, and Result Classification. Include these exact safety rules:

```markdown
- Do not run stress, spike, or soak tests against production user traffic paths.
- Do not run tests that consume VWorld or OpenDataPortal quota from Gongzzang.
- Do not test with production PII.
- Do not log Authorization, Cookie, Set-Cookie, Platform Core service tokens, or webhook secrets.
- Do not claim a launch spec without evidence under `target/audit/load-tests`.
```

- [ ] **Step 2: Create the scenario README**

Create `tests/load/README.md`:

```markdown
# Load Scenarios

These k6 scenarios are operator tooling for perf/staging capacity discovery.
They are not imported by `apps/`, `services/`, `crates/`, or `packages/`.

Run through `scripts/load/run-k6.ps1` so every run writes evidence.

Example:

```powershell
./scripts/load/run-k6.ps1 -Scenario api-read-mix -TargetBaseUrl https://perf.gongzzang.internal -Profile smoke
```
```

- [ ] **Step 3: Create the scenario registry**

Create `tests/load/scenarios.v1.json` with:

```json
{
  "schemaVersion": "gongzzang.load.scenarios.v1",
  "defaultTargetBaseUrl": "https://perf.gongzzang.internal",
  "slo": {
    "apiP95Ms": 300,
    "apiP99Ms": 1000,
    "errorRate": 0.01,
    "markerHitP95Ms": 100,
    "markerMissP95Ms": 500,
    "markerP99Ms": 1500
  },
  "scenarios": [
    {"id":"api-read-mix","file":"tests/load/scenarios/api-read-mix.js","maxSafeRps":50},
    {"id":"map-marker-mix","file":"tests/load/scenarios/map-marker-mix.js","maxSafeRps":50},
    {"id":"capacity-stress","file":"tests/load/scenarios/capacity-stress.js","maxSafeRps":800},
    {"id":"platform-core-events","file":"tests/load/scenarios/platform-core-events.js","maxSafeRps":50}
  ]
}
```

- [ ] **Step 4: Run the asset checker**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.ps1`

Expected: fails until Task 3 and Task 4 create the referenced script files.

- [ ] **Step 5: Commit after Task 4 completes**

Commit these docs together with the first passing asset checker run in Task 4.

## Task 3: k6 Shared Libraries

**Files:**
- Create: `tests/load/lib/env.js`
- Create: `tests/load/lib/http.js`

- [ ] **Step 1: Create environment helpers**

Create `tests/load/lib/env.js`:

```javascript
export function requireEnv(name) {
  const value = __ENV[name];
  if (!value || value.trim() === "") {
    throw new Error(`${name} is required`);
  }
  return value;
}

export function targetBaseUrl() {
  const url = requireEnv("TARGET_BASE_URL").replace(/\/+$/, "");
  if (/^https:\/\/(www\.)?gongzzang\.com$/.test(url) || /api\.gongzzang\.com/.test(url)) {
    throw new Error("production targets are forbidden for load tests");
  }
  return url;
}

export function profile() {
  return __ENV.LOAD_PROFILE || "smoke";
}

export function runTags(scenario) {
  return {
    scenario,
    environment: __ENV.LOAD_ENVIRONMENT || "perf",
    git_sha: __ENV.GIT_SHA || "unknown"
  };
}
```

- [ ] **Step 2: Create HTTP helpers**

Create `tests/load/lib/http.js`:

```javascript
import http from "k6/http";
import { check } from "k6";

export function safeGet(url, tags) {
  const response = http.get(url, { tags });
  check(response, {
    "status is 2xx or controlled 4xx": (r) => (r.status >= 200 && r.status < 300) || r.status === 404 || r.status === 429
  });
  return response;
}

export function safePostJson(url, body, tags, headers = {}) {
  const response = http.post(url, JSON.stringify(body), {
    headers: { "Content-Type": "application/json", ...headers },
    tags
  });
  check(response, {
    "status is accepted or controlled rejection": (r) => (r.status >= 200 && r.status < 300) || r.status === 409 || r.status === 429
  });
  return response;
}
```

- [ ] **Step 3: Run static checker**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.ps1`

Expected: still fails until scenario files exist.

## Task 4: k6 Scenarios

**Files:**
- Create: `tests/load/scenarios/api-read-mix.js`
- Create: `tests/load/scenarios/map-marker-mix.js`
- Create: `tests/load/scenarios/capacity-stress.js`
- Create: `tests/load/scenarios/platform-core-events.js`

- [ ] **Step 1: Create API read mix**

Create `tests/load/scenarios/api-read-mix.js` with k6 `constant-arrival-rate`, thresholds `http_req_failed < 0.01`, `p(95) < 300`, `p(99) < 1000`, and weighted calls to `/health`, `/v1/listings`, `/v1/listings/{id}`, `/api/proxy/catalog/v1/parcels/by-pnu/{pnu}`.

- [ ] **Step 2: Create map marker mix**

Create `tests/load/scenarios/map-marker-mix.js` with cache-hit and cache-miss tagged requests for `/v1/listing-marker-tiles/{z}/{x}/{y}.pbf`, `/v1/listing-marker-counts`, `/v1/listing-marker-filters`, and `/v1/listing-marker-masks`. Use fixed PNU/tile fixtures from perf seed data, not public `bbox` request shapes.

- [ ] **Step 3: Create capacity stress**

Create `tests/load/scenarios/capacity-stress.js` with stages for 50, 100, 200, 300, 400, 600, and 800 RPS. Require `ALLOW_STRESS=true`; otherwise throw before traffic starts.

- [ ] **Step 4: Create Platform Core event scenario**

Create `tests/load/scenarios/platform-core-events.js` to POST signed synthetic webhook events to `/platform-core/events`. Read `PLATFORM_CORE_WEBHOOK_SECRET` from environment, generate timestamped HMAC headers, send valid, duplicate, and poison events with separate tags.

- [ ] **Step 5: Run k6 inspect when k6 is available**

Run:

```powershell
k6 inspect tests/load/scenarios/api-read-mix.js
k6 inspect tests/load/scenarios/map-marker-mix.js
k6 inspect tests/load/scenarios/capacity-stress.js
k6 inspect tests/load/scenarios/platform-core-events.js
```

Expected: each command exits 0 and prints scenario options.

## Task 5: Evidence Wrappers

**Files:**
- Create: `scripts/load/run-k6.ps1`
- Create: `scripts/load/normalize-k6-summary.ps1`

- [ ] **Step 1: Create k6 launcher**

Create `scripts/load/run-k6.ps1` with parameters `Scenario`, `TargetBaseUrl`, `Profile`, `Environment`, `OutRoot`, and `AllowStress`. Resolve the scenario from `tests/load/scenarios.v1.json`, reject production targets, create `target/audit/load-tests/YYYY-MM-DD/<environment>/<scenario>/<timestamp>`, run `k6 run --summary-export k6-summary.json`, and write `run.json`, `spec.json`, and `thresholds.json`.

- [ ] **Step 2: Create summary normalizer**

Create `scripts/load/normalize-k6-summary.ps1` to read `k6-summary.json` and write:

```text
bottleneck.md
recommendation.md
baseline-comparison.md
```

The first version should classify results as `healthy`, `latency breakpoint`, or `error breakpoint` using `http_req_duration` and `http_req_failed` metrics.

- [ ] **Step 3: Run wrapper smoke on a local non-production target**

Run:

```powershell
./scripts/load/run-k6.ps1 -Scenario api-read-mix -TargetBaseUrl http://127.0.0.1:3000 -Profile smoke
```

Expected: if the target is unavailable, k6 records connection failures and evidence files are still written. If the target is available, thresholds decide pass/fail.

## Task 6: Manual Workflow And CI Wiring

**Files:**
- Create: `.github/workflows/load-test-capacity.yml`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add manual load-test workflow**

Create `.github/workflows/load-test-capacity.yml` using `workflow_dispatch`, `runs-on: [self-hosted, load-test]`, checkout SHA pin matching existing workflows, inputs for scenario/profile/target/environment, and an artifact upload step for `target/audit/load-tests/**`.

- [ ] **Step 2: Wire the static guardrail into CI**

Modify `.github/workflows/ci.yml` lint-format job after Platform Integration policy guardrail:

```yaml
      - name: Load-test asset guardrail
        shell: pwsh
        run: ./scripts/ci/check-load-test-assets.ps1
```

- [ ] **Step 3: Run local guardrail verification**

Run:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.tests.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-load-test-assets.ps1
pnpm exec markdownlint-cli2 "docs/testing/load.md" "tests/load/README.md"
```

Expected: tests print `check-load-test-assets-tests-ok`, checker prints `check-load-test-assets-ok scenarios=4`, markdownlint reports `0 error(s)`.

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/testing/load.md tests/load scripts/load scripts/ci/check-load-test-assets.ps1 scripts/ci/check-load-test-assets.tests.ps1 .github/workflows/load-test-capacity.yml .github/workflows/ci.yml
git commit -m "feat: add enterprise load-test capacity harness"
```

## Task 7: First Operator Run

**Files:**
- Create result evidence under `target/audit/load-tests/...`
- Create: `docs/research/YYYY-MM-DD-load-test-result.md`

- [ ] **Step 1: Run smoke against perf or local staging**

Run:

```powershell
./scripts/load/run-k6.ps1 -Scenario api-read-mix -TargetBaseUrl https://perf.gongzzang.internal -Profile smoke -Environment perf
```

Expected: evidence directory contains `run.json`, `spec.json`, `thresholds.json`, `k6-summary.json`, `bottleneck.md`, `recommendation.md`, and `baseline-comparison.md`.

- [ ] **Step 2: Write result report**

Create `docs/research/YYYY-MM-DD-load-test-result.md` with sections: Tested Spec, Scenario, Result Classification, Capacity Curve, First Bottleneck, Recommendation, Upgrade Trigger, Evidence Path.

- [ ] **Step 3: Commit report only if the run used a real perf target**

Run:

```powershell
git add docs/research/YYYY-MM-DD-load-test-result.md
git commit -m "docs: record load-test smoke result"
```

## Self-Review Checklist

- Spec coverage: enterprise references, SLO gates, scenario mix, stress, spike, soak, fault, overload safety, evidence, and sizing classification are covered.
- Boundary safety: no VWorld/OpenDataPortal load, no production PII, no direct Platform Core database access.
- Dependency safety: no npm or cargo package addition.
- Traceability: every run writes evidence and a result report before launch sizing claims.
