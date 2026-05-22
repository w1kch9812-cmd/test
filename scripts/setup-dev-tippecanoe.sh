#!/usr/bin/env bash
# Local Linux/WSL helper for installing tippecanoe at the repository SSOT SHA.
#
# Source of truth:
#   crates/sp9-base-layer-config -> tippecanoe_git_sha
#
# Usage:
#   ./scripts/setup-dev-tippecanoe.sh
#   ./scripts/setup-dev-tippecanoe.sh --verify

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERIFY_ONLY=false
if [[ "${1:-}" == "--verify" ]]; then
    VERIFY_ONLY=true
fi

SSOT_SHA=""
if command -v cargo >/dev/null 2>&1; then
    if SSOT_SHA="$(cargo run -q -p sp9-base-layer-config --bin sp9-config-print --frozen -- key tippecanoe_git_sha 2>/dev/null)"; then
        :
    else
        SSOT_SHA=""
    fi
fi

if [[ -z "$SSOT_SHA" ]]; then
    if grep_output="$(grep -oE '"[a-f0-9]{40}"' crates/sp9-base-layer-config/src/lib.rs)"; then
        SSOT_SHA="$(printf '%s\n' "$grep_output" | head -1 | tr -d '"')"
    else
        SSOT_SHA=""
    fi
fi

if [[ -z "$SSOT_SHA" || ${#SSOT_SHA} -ne 40 ]]; then
    echo "::error::failed to resolve TIPPECANOE_GIT_SHA from crates/sp9-base-layer-config/src/lib.rs"
    exit 1
fi

INSTALLED_SHA_FILE="/usr/local/bin/.sp9-tippecanoe-sha"

if $VERIFY_ONLY; then
    if [[ ! -f "$INSTALLED_SHA_FILE" ]]; then
        echo "::error::missing installed tippecanoe SHA file: $INSTALLED_SHA_FILE"
        echo "  run: ./scripts/setup-dev-tippecanoe.sh"
        exit 1
    fi

    INSTALLED_SHA="$(cat "$INSTALLED_SHA_FILE")"
    if [[ "$INSTALLED_SHA" != "$SSOT_SHA" ]]; then
        echo "::error::tippecanoe SHA mismatch"
        echo "  SSOT:      $SSOT_SHA"
        echo "  installed: $INSTALLED_SHA"
        echo "  run: ./scripts/setup-dev-tippecanoe.sh"
        exit 1
    fi

    echo "tippecanoe SHA verified: $SSOT_SHA"
    exit 0
fi

echo "Installing dev tippecanoe binary at SSOT SHA: $SSOT_SHA"

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "::error::this helper supports Linux/WSL only"
    exit 1
fi

if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -q
    sudo apt-get install -y --no-install-recommends \
        build-essential libsqlite3-dev zlib1g-dev git
else
    echo "::warning::apt-get not found; install build-essential, sqlite3, zlib, and git manually"
fi

BUILD_DIR="/tmp/sp9-tippecanoe-build"
if [[ -f "$INSTALLED_SHA_FILE" && "$(cat "$INSTALLED_SHA_FILE")" == "$SSOT_SHA" ]]; then
    echo "tippecanoe already matches SSOT SHA; skipping rebuild"
    exit 0
fi

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

git init -q
git remote add origin https://github.com/felt/tippecanoe.git
git fetch --depth 1 origin "$SSOT_SHA"
git checkout FETCH_HEAD

ACTUAL_SHA="$(git rev-parse HEAD)"
if [[ "$ACTUAL_SHA" != "$SSOT_SHA" ]]; then
    echo "::error::tippecanoe SHA mismatch: expected $SSOT_SHA, got $ACTUAL_SHA"
    exit 1
fi

make -j"$(nproc)"
sudo make install

echo "$SSOT_SHA" | sudo tee "$INSTALLED_SHA_FILE" > /dev/null

echo ""
echo "tippecanoe and tile-join installed at SSOT SHA: $SSOT_SHA"
echo "  binary: $(which tippecanoe)"
echo "  version: $(tippecanoe --version 2>&1 | head -1)"
echo ""
echo "verify: ./scripts/setup-dev-tippecanoe.sh --verify"
