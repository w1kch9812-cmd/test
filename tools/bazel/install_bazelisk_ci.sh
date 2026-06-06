#!/usr/bin/env bash
set -euo pipefail

install_dir="${1:-}"
if [[ -z "${install_dir}" ]]; then
  printf 'install-bazelisk-ci: install directory argument is required\n' >&2
  exit 64
fi

version="1.29.0"
sha256="5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992"
url="https://github.com/bazelbuild/bazelisk/releases/download/v${version}/bazelisk-linux-amd64"

mkdir -p "${install_dir}"
tmp_file="$(mktemp)"
trap 'rm -f "${tmp_file}"' EXIT

curl --fail --location --silent --show-error "${url}" --output "${tmp_file}"
printf '%s  %s\n' "${sha256}" "${tmp_file}" | sha256sum --check --status
install -m 0755 "${tmp_file}" "${install_dir}/bazelisk"
"${install_dir}/bazelisk" version
