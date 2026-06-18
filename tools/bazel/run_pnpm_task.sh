#!/usr/bin/env bash
set -euo pipefail

task="${1:-}"
if [ -z "$task" ]; then
  printf 'run-pnpm-task: task argument is required\n' >&2
  exit 2
fi

script_path="${BASH_SOURCE[0]}"
if command -v realpath >/dev/null 2>&1; then
  script_path="$(realpath "$script_path")"
fi
repo_root="$(cd "$(dirname "$script_path")/../.." && pwd)"

cd "$repo_root"

if ! command -v node >/dev/null 2>&1; then
  printf 'run-pnpm-task: node is required to run frontend Bazel target: %s\n' "$task" >&2
  exit 127
fi

if ! command -v pnpm >/dev/null 2>&1; then
  printf 'run-pnpm-task: pnpm is required to run frontend Bazel target: %s\n' "$task" >&2
  exit 127
fi

if [ ! -d "node_modules/.pnpm" ]; then
  printf 'run-pnpm-task: node_modules are missing; run `pnpm install --frozen-lockfile` first\n' >&2
  exit 127
fi

export CI="${CI:-true}"

case "$task" in
  lint)
    exec pnpm lint
    ;;
  build)
    export NEXT_PUBLIC_API_BASE_URL="${NEXT_PUBLIC_API_BASE_URL:-https://api.gongzzang.example}"
    export NEXT_PUBLIC_PLATFORM_CORE_BASE_URL="${NEXT_PUBLIC_PLATFORM_CORE_BASE_URL:-https://platform-core.gongzzang.example}"
    export NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID="${NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID:-ci-build-naver-client}"
    export ZITADEL_ISSUER="${ZITADEL_ISSUER:-https://auth.gongzzang.example}"
    export ZITADEL_CLIENT_ID="${ZITADEL_CLIENT_ID:-ci-build-client}"
    export ZITADEL_AUDIENCE="${ZITADEL_AUDIENCE:-ci-build-audience}"
    export ZITADEL_REDIRECT_URI="${ZITADEL_REDIRECT_URI:-https://gongzzang.example/api/auth/callback}"
    export REDIS_URL="${REDIS_URL:-rediss://redis.gongzzang.example:6379}"
    export SESSION_SECRET="${SESSION_SECRET:-ci-build-session-secret-32-bytes-valid}"
    export INTERNAL_AUTH_SECRET="${INTERNAL_AUTH_SECRET:-ci-build-internal-auth-secret-32-valid}"
    export PLATFORM_CORE_WEBHOOK_SECRET="${PLATFORM_CORE_WEBHOOK_SECRET:-ci-build-platform-core-webhook-secret-32-valid}"
    exec pnpm build
    ;;
  bundle)
    exec pnpm --filter=@gongzzang/web test:bundle
    ;;
  e2e)
    export NEXT_PUBLIC_API_BASE_URL="${NEXT_PUBLIC_API_BASE_URL:-http://localhost:8080}"
    export NEXT_PUBLIC_PLATFORM_CORE_BASE_URL="${NEXT_PUBLIC_PLATFORM_CORE_BASE_URL:-http://localhost:18082}"
    export NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID="${NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID:-ci-e2e-naver-client}"
    export ZITADEL_ISSUER="${ZITADEL_ISSUER:-http://localhost:8443}"
    export ZITADEL_CLIENT_ID="${ZITADEL_CLIENT_ID:-ci-e2e-client}"
    export ZITADEL_AUDIENCE="${ZITADEL_AUDIENCE:-ci-e2e-audience}"
    export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"
    export SESSION_SECRET="${SESSION_SECRET:-ci-test-secret-32-bytes-or-more-padding-padding}"
    export INTERNAL_AUTH_SECRET="${INTERNAL_AUTH_SECRET:-ci-e2e-internal-auth-secret-32-valid}"
    export PLATFORM_CORE_WEBHOOK_SECRET="${PLATFORM_CORE_WEBHOOK_SECRET:-ci-e2e-platform-core-webhook-secret-32}"
    exec pnpm --filter=@gongzzang/web test:e2e
    ;;
  *)
    printf 'run-pnpm-task: unknown task: %s\n' "$task" >&2
    exit 2
    ;;
esac
