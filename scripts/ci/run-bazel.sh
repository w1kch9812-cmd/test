#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  printf "run-bazel: bazel arguments are required\n" >&2
  exit 2
fi

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

  local bazel_args=""
  local arg
  local quoted_arg
  for arg in "$@"; do
    printf -v quoted_arg "%q" "$arg"
    bazel_args+="${quoted_arg} "
  done

  MSYS2_ARG_CONV_EXCL="*" exec wsl.exe --cd "${wsl_root}" bash -lc "
    set -euo pipefail
    if command -v bazelisk >/dev/null 2>&1; then
      exec bazelisk ${bazel_args}
    fi
    if [ -x \"\${HOME}/.local/bin/bazelisk\" ]; then
      exec \"\${HOME}/.local/bin/bazelisk\" ${bazel_args}
    fi
    printf "run-bazel: bazelisk not found in WSL\n" >&2
    exit 127
  "
}

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    run_wsl_bazelisk "$@"
    printf "run-bazel: wsl.exe is required on Windows\n" >&2
    exit 127
    ;;
esac

if command -v bazelisk >/dev/null 2>&1; then
  exec bazelisk "$@"
fi

if [ -x "${HOME}/.local/bin/bazelisk" ]; then
  exec "${HOME}/.local/bin/bazelisk" "$@"
fi

printf "run-bazel: bazelisk not found\n" >&2
exit 127
