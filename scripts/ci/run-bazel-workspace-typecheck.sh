#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"

run_wsl_bazelisk() {
  if ! command -v wsl.exe >/dev/null 2>&1; then
    return 1
  fi

  local wsl_root=""
  if [[ "${repo_root}" =~ ^/([a-zA-Z])/(.*)$ ]]; then
    local drive="${BASH_REMATCH[1],,}"
    local path_without_drive="${BASH_REMATCH[2]}"
    wsl_root="/mnt/${drive}/${path_without_drive}"
  else
    wsl_root="$(wsl.exe wslpath -a "${repo_root}" | tr -d '\r')"
  fi

  MSYS2_ARG_CONV_EXCL="*" exec wsl.exe --cd "${wsl_root}" bash -lc '
    set -euo pipefail
    if command -v bazelisk >/dev/null 2>&1; then
      exec bazelisk test //:workspace_typecheck --config=ci --verbose_failures
    fi
    if [ -x "${HOME}/.local/bin/bazelisk" ]; then
      exec "${HOME}/.local/bin/bazelisk" test //:workspace_typecheck --config=ci --verbose_failures
    fi
    printf "run-bazel-workspace-typecheck: bazelisk not found in WSL\n" >&2
    exit 127
  '
}

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    run_wsl_bazelisk
    printf "run-bazel-workspace-typecheck: wsl.exe is required on Windows\n" >&2
    exit 127
    ;;
esac

if command -v bazelisk >/dev/null 2>&1; then
  exec bazelisk test //:workspace_typecheck --config=ci --verbose_failures
fi

if [ -x "${HOME}/.local/bin/bazelisk" ]; then
  exec "${HOME}/.local/bin/bazelisk" test //:workspace_typecheck --config=ci --verbose_failures
fi

printf "run-bazel-workspace-typecheck: bazelisk not found\n" >&2
exit 127
