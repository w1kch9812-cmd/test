# Supply Chain Gate Runbook

## Status: provenance + deploy-admission ceremony removed (pre-launch)

The pre-launch provenance, SBOM-attestation, and production deploy-admission
machinery this runbook used to describe was removed. See
[ADR-0044](../adr/0044-bazel-transition-reconciliation.md).

These no longer exist, so do not try to run them:

- CI job `supply-chain-provenance` and the `actions/attest` provenance/SBOM
  attestation steps.
- cosign release signing / provenance steps.
- `.github/workflows/production-deploy-admission.yml`.
- `scripts/ci/verify-production-deploy-candidate(.tests)` and
  `scripts/ci/check-production-edge-admission(.tests)`.

## Surviving supply-chain gate

The supply-chain scan survives, native in `.github/workflows/ci.yml`:

- `cargo-deny` job — `cargo deny check` (config `deny.toml`).
- `secret-scan` job (`Secret scan (gitleaks)`) — `gitleaks/gitleaks-action`
  (config `.gitleaks.toml`). Pre-commit equivalent: `gitleaks protect --staged
  --redact -v`.

Per-PR boundary guards also run, via lefthook and the `guardrails` CI job:

- `scripts/lefthook/catalog-m1-boundary.sh`
- `scripts/lefthook/migration-version-prefixes.sh`

Frontend dependency audit is `pnpm audit`.

Policy SSOT:
[`supply-chain-policy.v1.json`](../architecture/platform-integration/supply-chain-policy.v1.json).

## If you need a deploy/provenance gate later

Re-introducing release provenance (SBOM/SLSA attestation) is a deliberate
post-launch decision, noted as intentionally out of scope in `ci.yml`. Add it
with a fresh ADR and runbook when there is a real launch need — do not resurrect
the removed scripts or admission workflow.
