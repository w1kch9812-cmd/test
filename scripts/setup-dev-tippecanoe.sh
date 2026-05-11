#!/usr/bin/env bash
# Round 5+ — dev tooling SSOT (ADR 0028 + 0029 후속).
#
# **목적**: local dev WSL 의 tippecanoe / tile-join binary 를 SSOT (`crates/sp9-base-layer-config`)
# 의 `TIPPECANOE_GIT_SHA` 와 *정확히 동일* SHA 빌드 강제. production CI 가 빌드하는
# 동일 binary 를 local 도 사용 → capability detection / version drift trick 제거.
#
# **사용**: WSL Ubuntu (또는 Linux dev 환경) 에서:
#   ./scripts/setup-dev-tippecanoe.sh
#
# 결과:
#   /usr/local/bin/tippecanoe  (SSOT SHA)
#   /usr/local/bin/tile-join   (SSOT SHA)
#   /usr/local/bin/.sp9-tippecanoe-sha  (실제 SHA 박제 — 후속 verification)
#
# **검증 모드** (Rust `tippecanoe::check_available` 가 호출):
#   ./scripts/setup-dev-tippecanoe.sh --verify  → exit 0 if SHA matches, 1 otherwise

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERIFY_ONLY=false
if [[ "${1:-}" == "--verify" ]]; then
    VERIFY_ONLY=true
fi

# SSOT 의 SHA 추출 — `cargo run -p sp9-base-layer-config -- key tippecanoe_git_sha`.
# fallback: lib.rs 에서 grep (cargo 빌드 안 됐을 때).
SSOT_SHA=""
if command -v cargo >/dev/null 2>&1; then
    SSOT_SHA="$(cargo run -q -p sp9-base-layer-config --bin sp9-config-print --frozen -- key tippecanoe_git_sha 2>/dev/null || true)"
fi
if [[ -z "$SSOT_SHA" ]]; then
    # fallback — direct grep (cargo 빌드 실패 시).
    SSOT_SHA="$(grep -oE '"[a-f0-9]{40}"' crates/sp9-base-layer-config/src/lib.rs | head -1 | tr -d '"')"
fi
if [[ -z "$SSOT_SHA" || ${#SSOT_SHA} -ne 40 ]]; then
    echo "::error::SSOT 의 TIPPECANOE_GIT_SHA 추출 실패. crates/sp9-base-layer-config/src/lib.rs 확인."
    exit 1
fi

INSTALLED_SHA_FILE="/usr/local/bin/.sp9-tippecanoe-sha"

# --verify 모드: 현재 installed SHA 와 SSOT SHA 비교.
if $VERIFY_ONLY; then
    if [[ ! -f "$INSTALLED_SHA_FILE" ]]; then
        echo "::error::tippecanoe SHA 박제 파일 없음 ($INSTALLED_SHA_FILE)."
        echo "  실행: ./scripts/setup-dev-tippecanoe.sh"
        exit 1
    fi
    INSTALLED_SHA="$(cat "$INSTALLED_SHA_FILE")"
    if [[ "$INSTALLED_SHA" != "$SSOT_SHA" ]]; then
        echo "::error::tippecanoe SHA mismatch."
        echo "  SSOT (sp9-base-layer-config): $SSOT_SHA"
        echo "  Installed:                    $INSTALLED_SHA"
        echo "  실행: ./scripts/setup-dev-tippecanoe.sh (재빌드)"
        exit 1
    fi
    echo "✓ tippecanoe SHA 일치: $SSOT_SHA"
    exit 0
fi

# 빌드 모드 — apt deps + git clone + checkout + make.
echo "Installing dev tippecanoe binary at SSOT SHA: $SSOT_SHA"

# WSL / Linux 만 지원.
if [[ "$(uname -s)" != "Linux" ]]; then
    echo "::error::본 script 는 Linux/WSL 만 지원. macOS 는 별도 brew tap 필요."
    exit 1
fi

# 빌드 deps (Ubuntu/Debian 가정 — apt).
if ! command -v apt-get >/dev/null 2>&1; then
    echo "::warning::apt-get 미감지. 다른 배포판이면 수동 deps 설치 필요."
fi

if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -q
    sudo apt-get install -y --no-install-recommends \
        build-essential libsqlite3-dev zlib1g-dev git
fi

# 빌드 디렉터리 (idempotent — 같은 SHA 재실행 시 skip).
BUILD_DIR="/tmp/sp9-tippecanoe-build"
if [[ -f "$INSTALLED_SHA_FILE" && "$(cat "$INSTALLED_SHA_FILE")" == "$SSOT_SHA" ]]; then
    echo "✓ tippecanoe 이미 SSOT SHA 와 일치 — skip rebuild."
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
    echo "::error::tippecanoe SHA mismatch — expected $SSOT_SHA, got $ACTUAL_SHA"
    exit 1
fi

make -j"$(nproc)"
sudo make install

# 박제 — 후속 --verify 가 검증.
echo "$SSOT_SHA" | sudo tee "$INSTALLED_SHA_FILE" > /dev/null

echo ""
echo "✓ tippecanoe / tile-join installed at SSOT SHA: $SSOT_SHA"
echo "  binary: $(which tippecanoe)"
echo "  version: $(tippecanoe --version 2>&1 | head -1)"
echo ""
echo "검증: ./scripts/setup-dev-tippecanoe.sh --verify"
