#!/usr/bin/env bash
# Zitadel CI 셋업 — admin token 으로 project + OIDC app + machine user + JWT 토큰 발급.
# 출력: $GITHUB_OUTPUT 에 issuer, audience, token

set -euo pipefail

ZITADEL_URL="${ZITADEL_URL:-http://localhost:8081}"

# 1) Zitadel 부팅 대기 (debug/healthz 는 v2 부터 보장)
echo "Waiting for Zitadel at $ZITADEL_URL ..."
for i in $(seq 1 180); do
  if curl -sf "$ZITADEL_URL/debug/healthz" >/dev/null 2>&1; then
    echo "Zitadel is up after ${i}s"
    break
  fi
  if [ "$i" = "180" ]; then
    echo "ERROR: Zitadel did not become healthy in 180s" >&2
    exit 1
  fi
  sleep 1
done

# 2) admin PAT — caller(workflow) 가 docker logs 에서 grep 해서 주입
if [ -z "${ZITADEL_ADMIN_PAT:-}" ]; then
  echo "ERROR: ZITADEL_ADMIN_PAT 환경 변수 필요" >&2
  exit 1
fi

AUTH="Authorization: Bearer ${ZITADEL_ADMIN_PAT}"

# admin PAT 검증 — /auth/v1/users/me 호출
echo "Validating admin PAT..."
ME_RESP=$(curl -sf -H "$AUTH" "$ZITADEL_URL/auth/v1/users/me" || true)
if [ -z "$ME_RESP" ]; then
  echo "ERROR: admin PAT validation failed (auth/v1/users/me)" >&2
  exit 1
fi
echo "admin PAT OK"

# 3) Project 생성
echo "Creating project..."
PROJECT_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/projects" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"name":"gongzzang-ci"}')
echo "Project resp: $PROJECT_RESP"
PROJECT_ID=$(echo "$PROJECT_RESP" | jq -r .id)
if [ -z "$PROJECT_ID" ] || [ "$PROJECT_ID" = "null" ]; then
  echo "ERROR: project create failed" >&2
  exit 1
fi
echo "PROJECT_ID=$PROJECT_ID"

# 4) OIDC application — JWT access tokens 발급용 (API + client_credentials)
echo "Creating OIDC application..."
APP_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/projects/$PROJECT_ID/apps/oidc" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "name":"gongzzang-api-ci",
    "redirectUris":["http://localhost:8080/callback"],
    "responseTypes":["OIDC_RESPONSE_TYPE_CODE"],
    "grantTypes":["OIDC_GRANT_TYPE_AUTHORIZATION_CODE","OIDC_GRANT_TYPE_REFRESH_TOKEN","OIDC_GRANT_TYPE_CLIENT_CREDENTIALS"],
    "appType":"OIDC_APP_TYPE_API",
    "authMethodType":"OIDC_AUTH_METHOD_TYPE_BASIC",
    "accessTokenType":"OIDC_TOKEN_TYPE_JWT"
  }')
echo "App resp: $APP_RESP"
CLIENT_ID=$(echo "$APP_RESP" | jq -r .clientId)
CLIENT_SECRET=$(echo "$APP_RESP" | jq -r .clientSecret)
if [ -z "$CLIENT_ID" ] || [ "$CLIENT_ID" = "null" ]; then
  echo "ERROR: OIDC app create failed" >&2
  exit 1
fi
echo "CLIENT_ID=$CLIENT_ID"

# 5) Machine user 생성 (client_credentials grant 의 sub 주체)
echo "Creating machine user..."
SU_RESP=$(curl -sf -X POST "$ZITADEL_URL/management/v1/users/machine" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"userName":"ci-test-user","name":"CI Test","description":"walking-skeleton","accessTokenType":"ACCESS_TOKEN_TYPE_JWT"}')
echo "SU resp: $SU_RESP"
SU_ID=$(echo "$SU_RESP" | jq -r .userId)
if [ -z "$SU_ID" ] || [ "$SU_ID" = "null" ]; then
  echo "ERROR: machine user create failed" >&2
  exit 1
fi
echo "SU_ID=$SU_ID"

# 6) Machine user 에 client_credentials secret 부여
echo "Adding secret to machine user..."
SECRET_RESP=$(curl -sf -X PUT "$ZITADEL_URL/management/v1/users/$SU_ID/secret" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{}')
echo "Machine user secret resp: $SECRET_RESP"
SU_CLIENT_ID=$(echo "$SECRET_RESP" | jq -r .clientId)
SU_CLIENT_SECRET=$(echo "$SECRET_RESP" | jq -r .clientSecret)

# 7) Machine user 에 user grant 추가 (best effort)
echo "Granting project to machine user (best effort)..."
curl -sf -X POST "$ZITADEL_URL/management/v1/users/$SU_ID/grants" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d "{\"projectId\":\"$PROJECT_ID\",\"roleKeys\":[]}" >/dev/null 2>&1 || true

# 8) Token 발급 — 두 경로 시도
SU_TOKEN=""

# 8a) client_credentials with OIDC app credentials, scope 에 audience 강제
echo "Trying client_credentials grant via OIDC app..."
TOKEN_RESP=$(curl -sf -u "$CLIENT_ID:$CLIENT_SECRET" \
  -d "grant_type=client_credentials&scope=openid urn:zitadel:iam:org:project:id:$PROJECT_ID:aud" \
  "$ZITADEL_URL/oauth/v2/token" || true)
echo "Token resp (8a): $TOKEN_RESP"
SU_TOKEN=$(echo "$TOKEN_RESP" | jq -r .access_token 2>/dev/null || true)

# 8b) client_credentials with machine user secret (sub = machine user)
if [ -z "$SU_TOKEN" ] || [ "$SU_TOKEN" = "null" ]; then
  echo "Trying client_credentials grant via machine user secret..."
  TOKEN_RESP=$(curl -sf -u "$SU_CLIENT_ID:$SU_CLIENT_SECRET" \
    -d "grant_type=client_credentials&scope=openid email profile urn:zitadel:iam:org:project:id:$PROJECT_ID:aud" \
    "$ZITADEL_URL/oauth/v2/token" || true)
  echo "Token resp (8b): $TOKEN_RESP"
  SU_TOKEN=$(echo "$TOKEN_RESP" | jq -r .access_token 2>/dev/null || true)
fi

if [ -z "$SU_TOKEN" ] || [ "$SU_TOKEN" = "null" ]; then
  echo "ERROR: failed to obtain access token via client_credentials" >&2
  exit 1
fi

# 9) Token 이 JWT(RS256) 인지 검증
echo "Verifying token is JWT (RS256)..."
HEADER_B64=$(echo "$SU_TOKEN" | cut -d. -f1)
# base64url → base64
HEADER_PAD=$(echo "$HEADER_B64" | tr '_-' '/+' | awk '{l=length($0); p=(4-l%4)%4; printf "%s%s", $0, substr("====",1,p)}')
HEADER_JSON=$(echo "$HEADER_PAD" | base64 -d 2>/dev/null || true)
echo "Token header: $HEADER_JSON"
TOKEN_ALG=$(echo "$HEADER_JSON" | jq -r .alg 2>/dev/null || true)
if [ "$TOKEN_ALG" != "RS256" ]; then
  echo "ERROR: token alg is not RS256 (got: $TOKEN_ALG). JwtVerifier will reject." >&2
  exit 1
fi

PAYLOAD_B64=$(echo "$SU_TOKEN" | cut -d. -f2)
PAYLOAD_PAD=$(echo "$PAYLOAD_B64" | tr '_-' '/+' | awk '{l=length($0); p=(4-l%4)%4; printf "%s%s", $0, substr("====",1,p)}')
PAYLOAD_JSON=$(echo "$PAYLOAD_PAD" | base64 -d 2>/dev/null || true)
echo "Token payload: $PAYLOAD_JSON"
TOKEN_AUD=$(echo "$PAYLOAD_JSON" | jq -r '.aud | if type=="array" then .[0] else . end' 2>/dev/null || true)
TOKEN_ISS=$(echo "$PAYLOAD_JSON" | jq -r .iss 2>/dev/null || true)
TOKEN_SUB=$(echo "$PAYLOAD_JSON" | jq -r .sub 2>/dev/null || true)
TOKEN_EMAIL=$(echo "$PAYLOAD_JSON" | jq -r .email 2>/dev/null || true)
TOKEN_PREF=$(echo "$PAYLOAD_JSON" | jq -r .preferred_username 2>/dev/null || true)
echo "iss=$TOKEN_ISS  aud=$TOKEN_AUD  sub=$TOKEN_SUB  email=$TOKEN_EMAIL  preferred_username=$TOKEN_PREF"

# AUDIENCE 결정 — token 의 aud 가 우리가 가진 client_id 와 일치하는 게 정상
AUDIENCE_FOR_API="$CLIENT_ID"
if [ -n "$TOKEN_AUD" ] && [ "$TOKEN_AUD" != "null" ] && [ "$TOKEN_AUD" != "$CLIENT_ID" ]; then
  echo "WARN: token aud ($TOKEN_AUD) != client_id ($CLIENT_ID). Using token aud for API."
  AUDIENCE_FOR_API="$TOKEN_AUD"
fi

# ISSUER — token 의 iss 를 그대로 사용 (Zitadel 이 외부에서 보는 URL)
ISSUER_FOR_API="$TOKEN_ISS"
if [ -z "$ISSUER_FOR_API" ] || [ "$ISSUER_FOR_API" = "null" ]; then
  ISSUER_FOR_API="$ZITADEL_URL"
fi

# 10) GITHUB_OUTPUT
{
  echo "issuer=$ISSUER_FOR_API"
  echo "audience=$AUDIENCE_FOR_API"
  echo "token=$SU_TOKEN"
  echo "client_id=$CLIENT_ID"
  echo "project_id=$PROJECT_ID"
} >> "$GITHUB_OUTPUT"

echo "Setup OK: issuer=$ISSUER_FOR_API audience=$AUDIENCE_FOR_API token=<masked, len=${#SU_TOKEN}>"
