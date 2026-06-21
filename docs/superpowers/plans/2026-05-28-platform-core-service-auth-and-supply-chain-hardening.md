# Platform Core Service Auth And Supply Chain Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Gongzzang's Platform Core integration harder to misuse by adding supply-chain gates and a service-to-service authentication contract.

**Architecture:** Keep Platform Core as the Catalog SSOT and Gongzzang as a consumer. Fix dependency advisories through pinned workspace overrides, then make the Gongzzang boundary manifest, env examples, and CI guardrails require service authentication for Platform Core HTTP and webhook paths.

**Tech Stack:** pnpm workspaces, GitHub Actions, repo guardrails (`repo-guard` Rust binary / `scripts/lefthook/*.sh`; the original PowerShell CI guardrails were removed per ADR-0044), Next.js route handler, Rust reqwest adapters, gitleaks, cargo-deny.

**Execution note, 2026-05-29:** The Gongzzang caller side is now policy-driven
and Platform Core has a matching inbound default-deny receiver policy. Platform
Core verifies bearer token, policy ID, source service, target service, and
allowed-call ID for Gongzzang parcel-by-PNU service reads. The remaining
identity hardening direction is replacing the transitional shared-token fallback
with workload identity or mTLS in the deployment platform. Browser/API traffic
policy now also keeps authenticated and privileged BFF route-handler rate
profiles in the traffic/auth registry instead of hardcoding per-route limits.
The same registry generates Rust API direct ingress rate policies for public
marker routes and protected backend routes, so BFF and backend enforcement share
one policy source while Redis stores only runtime counters. The Rust API also
enforces generated backend role policies for privileged routes, so BFF-side role
checks are not the only protection against direct API calls. Gongzzang now also
generates a provider-neutral edge/ingress projection from the same registry for
future CloudFront, AWS WAFv2, ALB, or service mesh IaC consumers. That edge
projection reduces public ingress drift, but it does not replace
service-to-service identity: private Platform Core reads still require workload
identity or mTLS-capable service authentication at the protected service
boundary. AWS WAFv2/Pulumi attachment and production deploy admission remain
deferred production-promotion work, not part of the current Platform Core
consumer integration PR. Do not treat local production-deploy, Pulumi, or load
evidence files as current completion evidence unless the release owner explicitly
opens the production-promotion workstream.

---

## File Structure

- Modify: `package.json`
  - Owns root pnpm overrides for vulnerable transitive dependencies.
- Modify: `pnpm-lock.yaml`
  - Generated dependency resolution lockfile.
- Modify: `.github/workflows/ci.yml`
  - Adds an explicit npm SCA gate with `pnpm audit --audit-level moderate`.
- Modify: `docs/architecture/platform-core-boundary.v1.json`
  - Adds the Platform Core service-auth environment contract.
- Modify: `.env.example`
  - Documents only placeholder names for service-auth configuration.
- Modify: `scripts/ci/check-platform-core-boundary`
  - Blocks Platform Core integration from dropping service-auth env contract.
- Modify: `scripts/ci/check-platform-core-boundary.tests`
  - Proves the guard fails when service-auth contract is missing.
- Modify: `docs/ssot-matrix.md`
  - Records service-auth SSOT and enforcement.

## Task 1: Close Known npm Vulnerabilities

- [x] Add root `pnpm.overrides` for patched Vite, PostCSS, and brace-expansion versions.
- [x] Regenerate `pnpm-lock.yaml` with pnpm.
- [x] Run `pnpm audit --audit-level moderate`; expected result is no moderate-or-higher advisories.
- [x] Run the focused web unit tests and typecheck to catch dependency breakage.

## Task 2: Make npm SCA A CI Gate

- [x] Add a CI step after `pnpm install --frozen-lockfile` in `.github/workflows/ci.yml`.
- [x] The command is `pnpm audit --audit-level moderate`.
- [x] This makes future vulnerable dependency drift fail before merge.

## Task 3: Add Platform Core Service-Auth Contract

- [x] Add server-only env contract names for outbound Platform Core service auth and inbound webhook signature validation.
- [x] Add the same names to `.env.example` as non-secret placeholders.
- [x] Update the architecture policy docs so the service-auth SSOT is explicit.

## Task 4: Enforce The Contract Automatically

- [x] Add guardrail test fixtures that fail if the boundary manifest lacks service-auth env names.
- [x] Update `check-platform-core-boundary` to enforce the service-auth env contract.
- [x] Run the guardrail tests and the real guardrail against the repo root.

## Task 4.5: Enforce Platform Core Receiver Side

- [x] Add Platform Core inbound traffic/auth policy for Gongzzang service reads.
- [x] Add Platform Core receiver middleware that defaults to deny for protected Gongzzang service routes.
- [x] Verify both reject and allow paths at router/middleware level.
- [x] Add cross-repo drift checking against Gongzzang's allowed-call matrix when the sibling repo is present.

## Task 5: Verify

- [x] Run `pnpm audit --audit-level moderate`.
- [x] Run `pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts`.
- [x] Run `pnpm --filter @gongzzang/web typecheck`.
- [x] Run `cargo-deny check`.
- [x] Run a scoped gitleaks source scan.
- [x] Run Platform Core boundary/dependency/contract guardrails.
- [x] Run `git diff --check` for touched Gongzzang files.
- [x] Run Platform Core Rust compile/test/clippy checks for the receiver slice.

## Self-Review

- Spec coverage: Covers supply-chain known advisories and the service-auth policy gap without moving Platform Core ownership into Gongzzang.
- Placeholder scan: No implementation placeholder remains; secret values stay out of source.
- Type consistency: Env names and CI command names are fixed across plan, docs, and guardrails.
