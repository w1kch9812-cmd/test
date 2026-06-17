# Bazel-Owned Supply Chain Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Promote supply-chain evidence to Bazel-owned graph outputs so CI only consumes declared artifacts and performs GitHub-bound attestation.

**Architecture:** Bazel builds release candidates, CycloneDX release SBOMs, and an evidence manifest as declared outputs. The platform-integration policy names those Bazel targets and output paths as the SSOT. GitHub Actions remains responsible only for OIDC-backed artifact attestations because signing belongs to the GitHub permission boundary.

**Tech Stack:** Bazel genrules, shell contract tests, PowerShell policy guardrails, GitHub Actions artifact attestations.

---

## File Structure

- Modify: `BUILD.bazel`
  - Add `web_release_sbom`, `api_release_sbom`, `supply_chain_evidence_manifest`, `supply_chain_evidence_artifacts`, and `verify_supply_chain` targets.
- Modify: `tools/bazel/BUILD.bazel`
  - Export shell helpers used by Bazel evidence targets.
- Create: `tools/bazel/generate_release_file_sbom.sh`
  - Generate deterministic CycloneDX JSON from release artifact file contents.
- Create: `tools/bazel/generate_supply_chain_evidence_manifest.sh`
  - Generate a deterministic evidence manifest listing release subjects and SBOM outputs.
- Create: `tools/bazel/check_supply_chain_evidence.sh`
  - Validate Bazel-built SBOMs and evidence manifest.
- Modify: `docs/architecture/platform-integration/supply-chain-policy.v1.json`
  - Add Bazel targets and `bazel-bin` output paths for SBOM/evidence outputs.
- Modify: `scripts/ci/check-platform-integration-policy.ps1`
  - Enforce Bazel-owned SBOM/evidence output paths and CI wiring.
- Modify: `scripts/ci/check-platform-integration-policy.tests.ps1`
  - Add RED coverage for non-Bazel SBOM outputs and missing evidence target wiring.
- Modify: `.github/workflows/ci.yml`
  - Build `//:supply_chain_evidence_artifacts` and upload/attest Bazel-generated SBOMs.
- Modify: `docs/runbooks/supply-chain-provenance-and-deploy-gate.md`
  - Document Bazel-owned evidence output paths.

## Tasks

- [x] **Task 1: Write failing policy test**
  - Add a negative fixture that leaves SBOM `output_file` under `target/supply-chain`.
  - Expected RED: checker test fails because the current checker still permits CI-side SBOM output paths.

- [x] **Task 2: Add Bazel evidence targets**
  - Generate deterministic CycloneDX release SBOM files from Bazel release outputs.
  - Generate an evidence manifest that lists subject paths and SBOM paths.
  - Add a Bazel contract test that validates all evidence outputs.

- [x] **Task 3: Enforce policy SSOT**
  - Require each SBOM artifact to declare `bazel_target`, `output_file`, and `subject_path`.
  - Require `output_file` to start with `bazel-bin/`.
  - Require CI and deployment policy to reference the Bazel evidence targets and output paths.

- [x] **Task 4: Rewire CI**
  - Replace CI-side SBOM generation steps with a Bazel build of evidence artifacts.
  - Keep GitHub `actions/attest` only for provenance and SBOM attestation.

- [ ] **Task 5: Verify, commit, push**
  - Run targeted policy tests, Bazel evidence tests, guardrails, and full `bazel test //...`.
  - Commit and push only after hooks pass and the worktree is clean.
