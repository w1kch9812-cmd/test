#!/usr/bin/env bash
set -euo pipefail

runfiles_root="${TEST_SRCDIR:-}/${TEST_WORKSPACE:-}"
web_sbom="${1:-${runfiles_root}/supply-chain/gongzzang-node-workspace-sbom.cdx.json}"
api_sbom="${2:-${runfiles_root}/supply-chain/gongzzang-rust-workspace-sbom.cdx.json}"
manifest="${3:-${runfiles_root}/supply-chain/evidence-manifest.json}"

require_file() {
  local path="$1"
  local label="$2"
  if [ ! -s "$path" ]; then
    printf 'check-supply-chain-evidence: %s is missing or empty: %s\n' "$label" "$path" >&2
    exit 1
  fi
}

require_contains() {
  local path="$1"
  local needle="$2"
  local label="$3"
  if ! grep -Fq "$needle" "$path"; then
    printf 'check-supply-chain-evidence: %s missing %s in %s\n' "$label" "$needle" "$path" >&2
    exit 1
  fi
}

require_file "$web_sbom" "web SBOM"
require_file "$api_sbom" "API SBOM"
require_file "$manifest" "evidence manifest"

for sbom in "$web_sbom" "$api_sbom"; do
  require_contains "$sbom" '"bomFormat": "CycloneDX"' "CycloneDX marker"
  require_contains "$sbom" '"specVersion": "1.5"' "CycloneDX spec version"
  require_contains "$sbom" '"type": "file"' "file component"
  require_contains "$sbom" '"alg": "SHA-256"' "file hash"
  require_contains "$sbom" '"gongzzang.subject_path"' "subject path property"
done

require_contains "$web_sbom" 'bazel-bin/gongzzang-web-next-build.tgz' "web subject"
require_contains "$api_sbom" 'bazel-bin/gongzzang-api-release/api' "API subject"
require_contains "$manifest" '"schema_version": "gongzzang.supply_chain_evidence_manifest.v1"' "manifest schema"
require_contains "$manifest" 'bazel-bin/gongzzang-web-next-build.tgz' "manifest web subject"
require_contains "$manifest" 'bazel-bin/gongzzang-api-release/api' "manifest API subject"
require_contains "$manifest" 'bazel-bin/supply-chain/gongzzang-node-workspace-sbom.cdx.json' "manifest web SBOM"
require_contains "$manifest" 'bazel-bin/supply-chain/gongzzang-rust-workspace-sbom.cdx.json' "manifest API SBOM"

printf 'supply-chain-evidence-ok web_sbom=%s api_sbom=%s manifest=%s\n' "$web_sbom" "$api_sbom" "$manifest"
