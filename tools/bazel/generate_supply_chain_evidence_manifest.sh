#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 5 ]; then
  printf 'usage: generate_supply_chain_evidence_manifest.sh <output-json> <web-subject> <api-subject> <web-sbom> <api-sbom>\n' >&2
  exit 2
fi

output_json="$1"
web_subject="$2"
api_subject="$3"
web_sbom="$4"
api_sbom="$5"

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

mkdir -p "$(dirname "$output_json")"
cat > "$output_json" <<JSON
{
  "schema_version": "gongzzang.supply_chain_evidence_manifest.v1",
  "release_subjects": [
    {
      "id": "web_next_build_archive",
      "ecosystem": "node",
      "subject_path": "$(json_escape "$web_subject")",
      "sbom_path": "$(json_escape "$web_sbom")"
    },
    {
      "id": "api_binary",
      "ecosystem": "rust",
      "subject_path": "$(json_escape "$api_subject")",
      "sbom_path": "$(json_escape "$api_sbom")"
    }
  ]
}
JSON

printf 'supply-chain-evidence-manifest-ok output=%s\n' "$output_json"
