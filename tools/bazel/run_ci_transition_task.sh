#!/usr/bin/env bash
set -euo pipefail

task="${1:-}"
if [ -z "$task" ]; then
  printf 'run-ci-transition-task: task argument is required\n' >&2
  exit 2
fi

script_path="${BASH_SOURCE[0]}"
if command -v realpath >/dev/null 2>&1; then
  script_path="$(realpath "$script_path")"
fi
repo_root="$(cd "$(dirname "$script_path")/../.." && pwd)"

cd "$repo_root"

case "$task" in
  node-audit)
    exec pnpm audit --audit-level moderate
    ;;
  rustfmt-check)
    exec cargo fmt --all -- --check
    ;;
  rust-clippy)
    exec cargo clippy --workspace --all-features --all-targets -- -D warnings
    ;;
  rust-check)
    exec cargo check --workspace --all-features
    ;;
  cargo-deny)
    exec cargo deny check
    ;;
  *)
    printf 'run-ci-transition-task: unknown task: %s\n' "$task" >&2
    exit 2
    ;;
esac
