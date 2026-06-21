# Platform Integration Policy SSOT Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a folder-shaped Platform Integration policy SSOT that ties route exposure, traffic budgets, service auth, webhook auth, and supply-chain gates into one drift-checkable control plane.

**Architecture:** Keep the existing narrow registries as authoritative and add a higher-level index under `docs/architecture/platform-integration/`. A guardrail validates cross-file consistency and CI/pre-push wiring. (The original `scripts/ci/check-platform-integration-policy` guardrail was a PowerShell check; it was removed per ADR-0044, which eliminated the verification meta-machine and ports surviving repo-specific guards into the `repo-guard` Rust binary and `scripts/lefthook/*.sh`.)

**Tech Stack:** JSON policy files, repo guardrails (`repo-guard` Rust binary / `scripts/lefthook/*.sh`), GitHub Actions, lefthook, Next.js webhook receiver, Rust reqwest Platform Core adapters, pnpm audit, cargo-deny, gitleaks.

---

## File Structure

- Create: `docs/architecture/platform-integration/index.v1.json`
  - Lists governed policy components and guardrails.
- Create: `docs/architecture/platform-integration/route-exposure-policy.v1.json`
  - Declares public, proxy, webhook, service, and diagnostic surfaces.
- Create: `docs/architecture/platform-integration/service-auth-policy.v1.json`
  - Declares outbound Platform Core service identity requirements.
- Create: `docs/architecture/platform-integration/webhook-policy.v1.json`
  - Declares inbound Platform Core webhook signature requirements.
- Create: `docs/architecture/platform-integration/supply-chain-policy.v1.json`
  - Declares npm/Rust/secret scanning policy gates.
- Create: `scripts/ci/check-platform-integration-policy`
  - Validates policy inventory and runtime/CI drift.
- Create: `scripts/ci/check-platform-integration-policy.tests`
  - Proves the guardrail fails for missing supply-chain or wiring requirements.
- Modify: `.github/workflows/ci.yml`
  - Runs the platform integration guardrail.
- Modify: `lefthook.yml`
  - Runs the platform integration guardrail before push.
- Modify: `docs/ssot-matrix.md`
  - Records the new control-plane SSOT.

## Task 1: Add Platform Integration Policy Files

- [x] Add the five JSON policy files under `docs/architecture/platform-integration/`.
- [x] Keep `traffic-auth-policy-registry.v1.json` and `platform-core-boundary.v1.json` as narrow SSOTs; reference them from the new index instead of copying their full contents.
- [x] Confirm every policy file has a schema version and owner.

## Task 2: Add Drift Checker

- [x] Add `scripts/ci/check-platform-integration-policy`.
- [x] Check policy schema versions, component inventory, service auth, webhook signature enforcement, supply-chain gates, and CI/pre-push wiring.
- [x] Add `scripts/ci/check-platform-integration-policy.tests` with success and failure fixtures.

## Task 3: Wire Enforcement

- [x] Add the checker to `.github/workflows/ci.yml` next to the other architecture guardrails.
- [x] Add the checker to `lefthook.yml` pre-push.
- [x] Add the Platform Integration Policy row to `docs/ssot-matrix.md`.

## Task 4: Verify

- [x] Run the platform-integration policy guardrail (originally the PowerShell `scripts/ci/check-platform-integration-policy` check; removed per ADR-0044). The surviving boundary guard is `bash scripts/lefthook/catalog-m1-boundary.sh`.
- [x] Run the existing traffic-auth, Platform Core boundary, webhook, catalog API, and dependency guardrails.
- [x] Run `pnpm exec biome check` on touched JSON/TS files.
- [x] Run `git diff --check` on touched files.
