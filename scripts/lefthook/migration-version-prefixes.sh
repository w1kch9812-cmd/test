#!/usr/bin/env bash
# Thin launcher so git hooks (which don't have ~/.cargo/bin on PATH) can run the
# Rust repo-guard migration-version-prefixes check. ADR-0044 #4 (PowerShell -> Rust).
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
exec cargo run -q -p repo-guard -- migration-version-prefixes
