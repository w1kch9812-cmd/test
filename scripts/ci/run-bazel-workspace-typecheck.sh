#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "${script_dir}/run-bazel.sh" test //:workspace_typecheck --config=ci --verbose_failures
