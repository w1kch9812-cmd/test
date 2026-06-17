#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
  printf 'usage: generate_release_file_sbom.sh <source-path> <output-json> <component-name> <subject-path>\n' >&2
  exit 2
fi

source_path="$1"
output_json="$2"
component_name="$3"
subject_path="$4"

if [ ! -e "$source_path" ]; then
  printf 'generate-release-file-sbom: source path does not exist: %s\n' "$source_path" >&2
  exit 1
fi

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

collect_files() {
  if [ -d "$source_path" ]; then
    find "$source_path" -type f -print0 | sort -z
  elif [ -f "$source_path" ]; then
    printf '%s\0' "$source_path"
  else
    printf 'generate-release-file-sbom: source path is not a file or directory: %s\n' "$source_path" >&2
    exit 1
  fi
}

mkdir -p "$(dirname "$output_json")"
tmp_json="$(mktemp)"
trap 'rm -f "$tmp_json"' EXIT

{
  printf '{\n'
  printf '  "bomFormat": "CycloneDX",\n'
  printf '  "specVersion": "1.5",\n'
  printf '  "version": 1,\n'
  printf '  "metadata": {\n'
  printf '    "component": {\n'
  printf '      "type": "application",\n'
  printf '      "name": "%s",\n' "$(json_escape "$component_name")"
  printf '      "version": "release-candidate"\n'
  printf '    },\n'
  printf '    "properties": [\n'
  printf '      {"name": "gongzzang.subject_path", "value": "%s"},\n' "$(json_escape "$subject_path")"
  printf '      {"name": "gongzzang.source_path", "value": "%s"}\n' "$(json_escape "$source_path")"
  printf '    ]\n'
  printf '  },\n'
  printf '  "components": [\n'
} > "$tmp_json"

component_count=0
while IFS= read -r -d '' file_path; do
  if [ -d "$source_path" ]; then
    relative_path="${file_path#"$source_path"/}"
  else
    relative_path="$(basename "$file_path")"
  fi
  hash_value="$(sha256sum "$file_path" | awk '{print $1}')"
  if [ "$component_count" -gt 0 ]; then
    printf ',\n' >> "$tmp_json"
  fi
  {
    printf '    {\n'
    printf '      "type": "file",\n'
    printf '      "name": "%s",\n' "$(json_escape "$relative_path")"
    printf '      "bom-ref": "file:%s",\n' "$(json_escape "$relative_path")"
    printf '      "hashes": [\n'
    printf '        {"alg": "SHA-256", "content": "%s"}\n' "$hash_value"
    printf '      ]\n'
    printf '    }'
  } >> "$tmp_json"
  component_count=$((component_count + 1))
done < <(collect_files)

if [ "$component_count" -eq 0 ]; then
  printf 'generate-release-file-sbom: source path contains no files: %s\n' "$source_path" >&2
  exit 1
fi

{
  printf '\n'
  printf '  ],\n'
  printf '  "properties": [\n'
  printf '    {"name": "gongzzang.component_count", "value": "%s"}\n' "$component_count"
  printf '  ]\n'
  printf '}\n'
} >> "$tmp_json"

mv "$tmp_json" "$output_json"
printf 'release-file-sbom-ok output=%s components=%s\n' "$output_json" "$component_count"
