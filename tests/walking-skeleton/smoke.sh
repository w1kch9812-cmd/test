#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://localhost:8080}"

echo "== Walking Skeleton smoke test =="
echo "API: $API_URL"

# 1. Wait for /healthz
echo
echo "[1/3] Waiting for $API_URL/healthz ..."
for i in {1..30}; do
    if curl -sf "$API_URL/healthz" >/dev/null 2>&1; then
        echo "    ✓ ready (after ${i}s)"
        break
    fi
    if [ "$i" = "30" ]; then
        echo "    ✗ FAIL: server didn't respond within 30s" >&2
        exit 1
    fi
    sleep 1
done

# 2. POST /users — create
echo
echo "[2/3] POST $API_URL/users"
CREATE_RESPONSE=$(curl -sf -X POST "$API_URL/users" \
    -H 'content-type: application/json' \
    -d '{
        "zitadel_sub":"smoke-test-1",
        "email":"alice@example.com",
        "display_name":"Alice",
        "user_kind":"individual"
    }')

echo "    Response: $CREATE_RESPONSE"

# Extract id (basic JSON grep — keeps test dep-free)
USER_ID=$(echo "$CREATE_RESPONSE" | grep -oE '"id":"[^"]+' | head -1 | cut -d'"' -f4)
if [ -z "$USER_ID" ]; then
    echo "    ✗ FAIL: no 'id' in response" >&2
    exit 1
fi
if [[ ! "$USER_ID" =~ ^usr_[0-9A-Z]{26}$ ]]; then
    echo "    ✗ FAIL: id doesn't match usr_<26 ULID> pattern: $USER_ID" >&2
    exit 1
fi
echo "    ✓ created user_id=$USER_ID"

# Verify version=1
VERSION=$(echo "$CREATE_RESPONSE" | grep -oE '"version":[0-9]+' | cut -d':' -f2)
if [ "$VERSION" != "1" ]; then
    echo "    ✗ FAIL: expected version=1 on first save, got $VERSION" >&2
    exit 1
fi

# 3. GET /users/:id — round-trip
echo
echo "[3/3] GET $API_URL/users/$USER_ID"
GET_RESPONSE=$(curl -sf "$API_URL/users/$USER_ID")
echo "    Response: $GET_RESPONSE"

GET_EMAIL=$(echo "$GET_RESPONSE" | grep -oE '"email":"[^"]+' | head -1 | cut -d'"' -f4)
if [ "$GET_EMAIL" != "alice@example.com" ]; then
    echo "    ✗ FAIL: email mismatch: $GET_EMAIL" >&2
    exit 1
fi

GET_DISPLAY=$(echo "$GET_RESPONSE" | grep -oE '"display_name":"[^"]+' | head -1 | cut -d'"' -f4)
if [ "$GET_DISPLAY" != "Alice" ]; then
    echo "    ✗ FAIL: display_name mismatch: $GET_DISPLAY" >&2
    exit 1
fi

echo
echo "== PASS: round-trip works (id=$USER_ID, email=$GET_EMAIL) =="
echo
echo "Bonus checks:"

# 4. Bonus: POST with invalid email → 400
echo
echo "[bonus 1] POST with invalid email — expect HTTP 400"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/users" \
    -H 'content-type: application/json' \
    -d '{"zitadel_sub":"x","email":"not-an-email","display_name":"X","user_kind":"individual"}')
if [ "$HTTP_CODE" != "400" ]; then
    echo "    ✗ FAIL: expected 400, got $HTTP_CODE" >&2
    exit 1
fi
echo "    ✓ rejected with HTTP 400"

# 5. Bonus: GET nonexistent → 404
echo
echo "[bonus 2] GET nonexistent id — expect HTTP 404"
NONEXISTENT="usr_00000000000000000000000000"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/users/$NONEXISTENT")
if [ "$HTTP_CODE" != "404" ]; then
    echo "    ✗ FAIL: expected 404, got $HTTP_CODE" >&2
    exit 1
fi
echo "    ✓ returned HTTP 404"

# 6. Bonus: GET malformed id → 400
echo
echo "[bonus 3] GET malformed id — expect HTTP 400"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/users/not-a-valid-id")
if [ "$HTTP_CODE" != "400" ]; then
    echo "    ✗ FAIL: expected 400, got $HTTP_CODE" >&2
    exit 1
fi
echo "    ✓ rejected with HTTP 400"

echo
echo "All smoke tests PASS ✓"
exit 0
