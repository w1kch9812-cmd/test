# Platform Integration Enterprise Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Platform Core integration from registry-level guardrails toward an enterprise control plane with explicit service-call authorization, exception governance, replay-safe events, provenance, and deploy gates.

**Architecture:** Keep `docs/architecture/platform-integration/` as the folder-shaped SSOT. Add small policy files for each control-plane concern and make `check-platform-integration-policy` reject drift, missing ownership, and expired exceptions before runtime work is treated as production-ready.

**Tech Stack:** JSON policy SSOT, repo guardrails (`repo-guard` Rust binary / `scripts/lefthook/*.sh`; the original PowerShell guardrails were removed per ADR-0044), Next.js webhook receiver, Rust Platform Core clients, GitHub Actions, lefthook, gitleaks, cargo-deny, pnpm audit, future SBOM/attestation/deploy admission.

---

## File Structure

- Modify: `docs/architecture/platform-integration/index.v1.json`
  - Registers new policy components.
- Create: `docs/architecture/platform-integration/allowed-call-matrix.v1.json`
  - Explicit allow-list for service-to-service and cross-repo calls.
- Create: `docs/architecture/platform-integration/exception-policy.v1.json`
  - Defines owner/reason/expiry/approval requirements for exceptions.
- Modify: `scripts/ci/check-platform-integration-policy`
  - Enforces call matrix and exception governance.
- Modify: `scripts/ci/check-platform-integration-policy.tests`
  - Proves expired exceptions and missing gates fail.

## Phase 1: Policy Control Plane

- [x] Add an explicit `default_decision = deny` allowed-call matrix.
- [x] Declare active Gongzzang to Platform Core catalog read calls.
- [x] Declare active Platform Core to Gongzzang webhook calls.
- [x] Reserve the planned Dawneer to Platform Core catalog read path.
- [x] Declare direct cross-repo database access as prohibited.
- [x] Add exception governance with max TTL, owner, reason, approval, and compensating controls.
- [x] Extend the guardrail to reject missing call matrix entries.
- [x] Extend the guardrail to reject expired active exceptions.

## Phase 2: Runtime Event Safety

- [x] Add an inbox/replay ledger for Platform Core events.
- [x] Store event idempotency keys before applying side effects.
- [x] Add a dead-letter path for poison events.
- [x] Verify event schema compatibility before accepting a new event type.

## Phase 3: Service Identity Upgrade

- [x] Add token metadata: scope, issued-at, expiry, and rotation owner.
- [x] Add rotation runbook and CI check that production examples do not use dev tokens.
- [x] Design SPIFFE/SPIRE or cloud workload identity cutover.
- [x] Add default-deny service authorization policy before replacing bearer tokens.

## Phase 4: Supply Chain Provenance

- [x] Generate SBOMs for Node and Rust artifacts.
- [x] Add artifact attestation for production build outputs.
- [x] Require signed provenance for production deploy candidates.
- [x] Reject deploy artifacts not built by the approved workflow.
- [x] Require production edge admission so deploy candidates cannot skip the
  generated traffic/auth WAF attachment gate.

## Phase 5: Observability And Operations

- [x] Add required trace/span attributes to the integration policy.
- [x] Add SLO policy for Platform Core calls and webhooks.
- [x] Link alert policies and runbooks from the policy SSOT.
- [x] Add load and fault tests for graceful degradation.
