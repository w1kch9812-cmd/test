# Sub-project 6-i: Auth Core — Zitadel OIDC + Redis Session + Audit (Spec)

| | |
|---|---|
| 작성일 | 2026-05-05 |
| 상태 | Design (사용자 승인 대기) |
| 선행 | SP3 (Auth — Zitadel JWT Verifier, `crates/auth`), SP6-foundation (Next.js 16 + ky + i18n) |
| 후속 | SP6-org (multi-org switcher), SP6-CI (KISA 본인확인 SDK), SP6-Social (카카오/네이버 federation), SP6-iam-infra (Zitadel Pulumi + backup) |

---

## 1. 개요

공짱 사용자가 **Zitadel hosted login** 을 통해 OIDC PKCE 흐름으로 로그인하고,
**httpOnly cookie + Redis session** 으로 서버측 인증 상태가 보존되며,
**JTI denylist + audit_log** 로 토큰 revocation + 추적성이 보장되는 — 진짜 production 등급 인증 코어를 구축해요.

본 sub-project 는 SP3 의 backend Verifier 위에 **frontend 인증 흐름** 을 얹는 작업이에요.
Backend 의 JWT 검증은 이미 작동 중. SP6-i 는 (a) Zitadel 으로부터 token 을 어떻게 얻고
(b) cookie / session 으로 안전하게 보관하고 (c) /api/proxy 를 통해 backend 에 전달하는지를 다뤄요.

본 spec 은 *인증 코어의 진짜 SSS* 만 다뤄요. multi-org 컨텍스트, KISA 본인확인,
카카오/네이버 federation, Zitadel infra-as-code 는 각각 별도 후속 sub-project.

---

## 2. 범위 (Scope)

### 포함

**Identity / Session**
- Zitadel self-host (dev: docker-compose / prod: SP6-iam-infra 가 Pulumi 화)
- OIDC PKCE 흐름 (`oauth4webapi` 저수준 client, lock-in 0)
- Redis backed session (cookie 는 32-byte sid 만, 토큰 payload 는 Redis 에)
- `__Host-` cookie prefix + Secure + HttpOnly + SameSite=Strict + Partitioned
- Refresh token rotation + single-flight mutex (동시 401 시 1회만 refresh)
- Back-channel logout (Zitadel `end_session_endpoint` + Redis sid 삭제 + JTI denylist 추가)

**Authorization**
- Path 분기 RBAC: `(public)/*`, `(authenticated)/*`, `(authenticated)/admin/*`
- middleware.ts 가 unauth → /login redirect, role mismatch → /forbidden
- JTI denylist (Redis) — backend `crates/auth` 가 verify 단계에서 체크
- Role 변경 시 즉시 token revoke (모든 jti denylist)

**Auto-enforcement (자동 강제)**
- Rate limit (Redis sliding window): `/api/auth/login` 5/min/IP, `/api/auth/callback` 10/min/IP, `/api/auth/refresh` 30/min/sid
- `next.config.js` strict CSP nonce-based + HSTS preload + X-Frame-Options DENY
- Log redaction — pino logger 가 `access_token`, `refresh_token`, `ci` 필드 자동 마스킹
- lefthook pre-push 에 `cargo sqlx prepare --check` 추가 (V004 schema drift 차단)

**Traceability + Observability**
- `crates/auth/audit.rs` — `AuthEvent::{Login, Logout, RefreshSucceeded, RefreshFailed, RoleGuardDenied, RoleChanged}` 이벤트 emit → `audit_log` 테이블 (payload 에 `user_id`, `jti`, `exp` 포함하여 role 변경 시 활성 jti 조회 가능)
- `instrumentation.ts` OpenTelemetry — span: `oidc.authorize`, `oidc.callback`, `token.refresh`, `role.guard`
- 모든 `/api/auth/*` 응답이 RFC 7807 ProblemDetails 포맷 (backend 와 일관)

**UI**
- `/login`, `/profile`, `/logout` 화면 (shadcn primitives + 해요체 i18n)
- 모든 사용자 노출 string 이 `messages/auth.ko.json` (옵션 A 강제, AGENTS.md § 5)
- WCAG 2.1 AA + @axe-core/playwright e2e

**Schema 자리 (NULL 허용 컬럼만 — 미래 SP 가 채움)**
- migration `V004_auth_extension.sql`
  - `users.ci VARCHAR(88) UNIQUE NULL` — SP6-CI 가 채움
  - `external_account` 테이블 — SP6-Social 이 카카오/네이버 채움
  - SP6-i 시점에는 `provider='zitadel'` 한 줄만 first sign-in 시 자동 insert

### 미포함 (후속 SP)

- **SP6-org** — 한 사용자가 여러 organization 소속 가능, JWT `org_id` claim, org switcher UI
- **SP6-CI** — KISA 본인확인 vendor SDK (NICE / Toss / 카카오 본인확인) 통합, CI state machine (`UNVERIFIED → PENDING_CI → VERIFIED`)
- **SP6-Social** — Zitadel external IdP federation (카카오 / 네이버 / Google), `external_account` 실 채움 + multi-provider 동일인 매칭 (CI 기반)
- **SP6-iam-infra** — Zitadel self-host 의 Pulumi 코드화, JWKS rotation 절차, DB backup, alert
- 매물 / 검색 / 지도 / CRM — SP6-ii 이후

### 결정 사항 (이번 brainstorming 에서 확정)

| # | 항목 | 결정 | 사유 |
|---|---|---|---|
| 1 | IdP | **Zitadel self-host** | 사용자 가입 거부, 비용 0, Keycloak 보다 multi-org 모델 우수 |
| 2 | OIDC client | **oauth4webapi** | 저수준, lock-in 0, next-auth heavyweight 회피 |
| 3 | Session storage | **Redis backed** (cookie 는 sid 만) | iron-session sealed cookie 만으로는 token revocation 불가 |
| 4 | Login UX | **Zitadel hosted login redirect** | OIDC 표준, 자체 form 은 Zitadel API 추가 통합 비용 |
| 5 | Logout | **Back-channel** (`end_session_endpoint`) | Zitadel SSO session 도 끊어야 다음 로그인 시 자동 재인증 안 됨 |
| 6 | RBAC 진입점 | **Path 분기** `/(authenticated)/admin/*` | Next.js App Router segment grouping = 단일 layout = 단일 gate |
| 7 | Onboarding | **Signup 후 즉시 매물 검색** | CI 본인확인은 거래 시점 lazy (SP6-CI), 진입 마찰 최소 |
| 8 | Schema | **자리만** (`users.ci`, `external_account`) | 1 sub-project = 1 책임. 실 채움은 SP6-CI / SP6-Social |

---

## 3. 아키텍처

```
┌─────────────────────────────────────────────────────────────────┐
│  Browser                                                        │
│  Cookie: __Host-sid=<32B>; Secure; HttpOnly;                    │
│          SameSite=Strict; Partitioned                           │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│  Next.js 16 (apps/web)                                          │
│                                                                 │
│  middleware.ts                                                  │
│    1. rate limit (sliding window, edge KV)                      │
│    2. CSP nonce 주입                                            │
│    3. auth gate (sid → Redis lookup → user + role)              │
│    4. RBAC: (authenticated)/admin → role ∈ {admin, broker}      │
│                                                                 │
│  /api/auth/login    : oauth4webapi PKCE start                   │
│  /api/auth/callback : code → token exchange + Redis sid 발급    │
│  /api/auth/logout   : end_session + Redis 삭제 + JTI denylist   │
│  /api/proxy/[...path]: sid → access_token → Bearer 헤더 변환    │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│  Redis (dev: docker / prod: SP6-iam-infra)                      │
│  KEY                          VALUE                             │
│  session:<sid>                {sub, jti, role, access_token,    │
│                                refresh_token, exp}              │
│  refresh:lock:<sid>           single-flight mutex (SETNX)       │
│  jti:deny:<jti>               1 (TTL = remaining JWT ttl)       │
│  ratelimit:<route>:<ip>       sliding window counter            │
└────────────────────┬────────────────────────────────────────────┘
                     │
              Authorization: Bearer <Zitadel JWT>
                     │
┌────────────────────▼────────────────────────────────────────────┐
│  services/api (Rust, SP3 Verifier 확장)                         │
│                                                                 │
│  crates/auth                                                    │
│    ├── verifier.rs       (SP3, 변경 없음)                       │
│    ├── jti_denylist.rs   (NEW — Redis check)                    │
│    └── audit.rs          (NEW — AuthEvent emit)                 │
│                                                                 │
│  audit_log 테이블 (SP3 V003) ← AuthEvent INSERT                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.1 Trust 경계

- **Zitadel self-host** = identity SSOT (사용자 인증, JWT 발급)
- **Redis** = active session SSOT (sid → token 매핑, JTI denylist)
- **Postgres `users.role`** = authorization SSOT (role 변경 시 JTI denylist 추가)
- **frontend** = 절대 권한 결정 안 함 (JWT claim 표시만)

이 4 분리가 SSS 의 핵심. Zitadel 다운 시에도 기존 session 은 동작 (Redis 유효),
Postgres 다운 시에도 인증은 동작 (Zitadel + Redis), Redis 다운 시에는 인증 차단 (가용성 trade-off).

---

## 4. 데이터 흐름

### 4.1 로그인 (PKCE + Redis session 발급)

```
1. User → /(public)/login → "로그인" 버튼 클릭
2. POST /api/auth/login
   a. code_verifier = random(32B), code_challenge = S256(code_verifier)
   b. state = random(32B), nonce = random(32B)
   c. 임시 cookie 발급 (10분 TTL): {code_verifier, state, nonce}
   d. 302 → ZITADEL_URL/oauth/v2/authorize
            ?response_type=code
            &client_id=...
            &redirect_uri=APP_URL/api/auth/callback
            &scope=openid profile email offline_access
            &code_challenge=...&code_challenge_method=S256
            &state=...&nonce=...

3. Zitadel hosted login UI → 사용자 인증 → 302 → /api/auth/callback?code=...&state=...

4. GET /api/auth/callback
   a. cookie state == query state 비교 (CSRF) — 다르면 401 + ProblemDetails
   b. oauth4webapi.authorizationCodeGrantRequest({code, code_verifier})
   c. id_token 검증 (signature + iss + aud + nonce)
   d. AuthEvent::Login 을 backend `/internal/auth/event` 로 emit (audit_log)
   e. sid = random(32B)
   f. Redis SET session:<sid> {sub, jti, role, access_token, refresh_token, exp} EX <access_ttl>
   g. Set-Cookie: __Host-sid=<sid>; Secure; HttpOnly; SameSite=Strict; Partitioned; Path=/; Max-Age=<refresh_ttl>
   h. 302 → returnTo (default: /listings)
```

### 4.2 인증된 요청 (proxy 변환)

```
1. Browser → GET /(authenticated)/profile
2. middleware.ts:
   a. cookie __Host-sid 추출
   b. Redis GET session:<sid> → {sub, jti, role, access_token, exp}
   c. exp 가 임박 (60s 이내) → silent renew (4.3)
   d. role mismatch → /forbidden, 정상 → 통과
3. Server Component 가 /api/proxy/users/me fetch
4. /api/proxy/[...path]:
   a. sid 검증 (middleware 와 동일)
   b. Authorization: Bearer <access_token> 헤더 추가
   c. backend services/api/users/me 호출
5. crates/auth Verifier:
   a. JWKS lookup (kid)
   b. signature + iss + aud + exp 검증
   c. JTI denylist (Redis) 체크 — denied 면 401
   d. 통과 → AuthenticatedUser extractor 로 핸들러 진입
```

### 4.3 Refresh (single-flight mutex)

**Token TTL 정책** (Zitadel project 설정):
- access_token: **5분** (짧게 유지 → revocation 부담 ↓)
- refresh_token: **30일** (rotation 마다 갱신)
- session cookie Max-Age: refresh_token TTL 와 동일

**Refresh trigger**: middleware / proxy 가 `now > exp - 60s` 감지 시.

```
동시 401 시나리오: 5 개 fetch 가 동시에 401 → 5 번 refresh 호출 → token race

Single-flight 해결:
1. middleware 또는 proxy 가 exp - 60s 임박 감지
2. Redis SETNX refresh:lock:<sid> 1 EX 10
   - 성공 (락 획득) → token refresh 수행 → Redis session 갱신 → 락 삭제
   - 실패 (이미 락 있음) → 100ms backoff (max 3회) + 다시 session GET (이미 갱신됨)
3. token refresh:
   a. AuthEvent::RefreshSucceeded / RefreshFailed emit (이전/새 jti 모두 포함)
   b. 이전 jti 를 jti:deny:<jti> 추가 (rotation, TTL = 이전 access_token 잔여 ttl)
   c. Redis session:<sid> overwrite + EX 갱신
```

### 4.4 로그아웃 (back-channel)

```
1. User → /profile → "로그아웃" 클릭
2. POST /api/auth/logout
   a. Redis GET session:<sid> → {jti, id_token, ...}
   b. Redis SET jti:deny:<jti> 1 EX <remaining_ttl>
   c. Redis DEL session:<sid>
   d. AuthEvent::Logout emit
   e. Set-Cookie: __Host-sid=; Max-Age=0
   f. 302 → ZITADEL_URL/oidc/v1/end_session?id_token_hint=<id_token>&post_logout_redirect_uri=APP_URL/
3. Zitadel 가 자체 SSO session 종료 → 302 → /
```

### 4.5 Role 변경 시 즉시 반영

**전제**: `AuthEvent::Login` / `RefreshSucceeded` 가 emit 시 `(user_id, jti, exp)` 를 audit_log payload 에 기록. → role 변경 핸들러가 user_id 로 활성 jti 조회 가능.

```
1. Admin 이 user X 의 role 을 broker → admin 으로 변경 (SP6-iv 또는 admin UI)
2. Backend 가 users.role UPDATE 후 (같은 트랜잭션 내):
   a. audit_log SELECT WHERE user_id=X AND event_type IN (Login, RefreshSucceeded)
      AND now() < exp ORDER BY ts DESC
      → 활성 jti 목록
   b. 각 jti 를 Redis jti:deny:<jti> 추가 (TTL = exp - now)
   c. AuthEvent::RoleChanged emit (이전/새 role + 무효화한 jti 개수)
3. user X 의 다음 요청 → backend Verifier 가 jti:deny hit → 401
4. frontend 가 /login redirect → user X 재로그인 → 새 JWT 에 새 role claim
```

---

## 5. Schema (V004 — 자리만)

```sql
-- migrations/V004_auth_extension.sql

-- SP6-CI 가 채울 자리
ALTER TABLE users ADD COLUMN ci VARCHAR(88) UNIQUE NULL;
COMMENT ON COLUMN users.ci IS
  'KISA Connecting Information (88-char hash). Populated by SP6-CI via NICE/Toss/PASS SDK on transaction-time verification.';

-- SP6-Social 이 카카오/네이버 채움. SP6-i 는 zitadel 한 줄만.
CREATE TABLE external_account (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  provider     VARCHAR(32) NOT NULL,  -- 'zitadel' | 'kakao' | 'naver' | 'google'
  external_id  VARCHAR(255) NOT NULL,
  linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (provider, external_id)
);
CREATE INDEX idx_external_account_user_id ON external_account(user_id);
COMMENT ON TABLE external_account IS
  'Multi-IdP linking. SP6-i populates only zitadel rows on first sign-in. SP6-Social populates kakao/naver/google via federation; same-person matching via users.ci UNIQUE constraint.';
```

**자동 강제**: lefthook pre-push 에 `cargo sqlx prepare --check` 추가 → migration 적용 후 sqlx prepared queries regenerate 안 했으면 push 차단.

---

## 6. 디렉토리 구조

```
apps/web/
├── middleware.ts                       # rate limit + CSP nonce + auth gate
├── next.config.js                      # strict CSP + HSTS + X-Frame-Options
├── instrumentation.ts                  # OTel + Sentry hookup (SP7-i 가 Sentry 채움)
├── app/
│   ├── (public)/
│   │   ├── login/page.tsx              # "로그인" 버튼 → /api/auth/login
│   │   └── forbidden/page.tsx
│   ├── (authenticated)/
│   │   ├── layout.tsx                  # Server Component, getSession() 강제
│   │   └── profile/page.tsx            # /me 표시 + 로그아웃 버튼
│   └── api/
│       ├── auth/
│       │   ├── login/route.ts          # PKCE start
│       │   ├── callback/route.ts       # code → token → Redis sid
│       │   ├── logout/route.ts         # back-channel + JTI denylist
│       │   └── refresh/route.ts        # silent renew (single-flight)
│       └── proxy/[...path]/route.ts    # sid → Bearer 변환 (SP6-foundation 확장)
├── lib/
│   ├── oidc.ts                         # oauth4webapi 래퍼 (discovery + exchange)
│   ├── session/
│   │   ├── redis.ts                    # ioredis client (singleton)
│   │   ├── store.ts                    # session CRUD
│   │   ├── single-flight.ts            # refresh mutex
│   │   └── cookie.ts                   # __Host- cookie helpers
│   ├── ratelimit.ts                    # edge sliding window
│   ├── http/problem.ts                 # RFC 7807 ProblemDetails 헬퍼
│   ├── observability/
│   │   ├── tracer.ts                   # OTel span helper
│   │   └── redact.ts                   # log 마스킹 (token, ci)
│   └── i18n/messages/auth.ko.json      # 모든 auth string i18n
└── tests/
    ├── unit/oidc.test.ts
    ├── unit/single-flight.test.ts
    └── e2e/auth.spec.ts                # Zitadel real container

services/api/
└── routes/internal/auth_event.rs       # NEW — frontend 가 AuthEvent emit

crates/auth/
├── verifier.rs                         # SP3, 변경 없음
├── jti_denylist.rs                     # NEW — Redis check trait + impl
└── audit.rs                            # NEW — AuthEvent enum + writer

migrations/
└── V004_auth_extension.sql             # users.ci + external_account 자리

docs/auth/
└── frontend-integration.md             # 운영 SSOT
```

---

## 7. Task 분해 (writing-plans 에서 상세화)

| Task | 내용 | 파일 | 추정 |
|---|---|---|---|
| **T1** | Zitadel self-host + Redis docker-compose (dev) + Zitadel project / redirect URI 등록 + zod env schema | `infra/zitadel/docker-compose.yml`, `infra/zitadel/init.sh`, `apps/web/lib/env.ts` | 0.5d |
| **T2** | Redis session store + `__Host-` cookie + single-flight mutex + RFC 7807 ProblemDetails | `lib/session/*`, `lib/http/problem.ts` | 0.5d |
| **T3** | `oauth4webapi` PKCE + `/api/auth/{login,callback,logout,refresh}` Route Handlers + i18n `auth.ko.json` | `lib/oidc.ts`, `app/api/auth/*` | 0.75d |
| **T4** | `middleware.ts` (rate limit + CSP nonce + auth gate) + `next.config.js` strict headers + log redaction | `middleware.ts`, `next.config.js`, `lib/observability/*`, `lib/ratelimit.ts` | 0.5d |
| **T5** | `crates/auth` JTI denylist (Redis trait + impl) + audit_log emit (`AuthEvent`) + OTel span | `crates/auth/jti_denylist.rs`, `crates/auth/audit.rs`, `services/api/routes/internal/auth_event.rs` | 0.5d |
| **T6** | V004 migration (`users.ci`, `external_account` 자리) + `cargo sqlx prepare --check` lefthook hook + first-sign-in `external_account` insert | `migrations/V004_*`, `lefthook.yml`, `crates/auth/first_sign_in.rs` | 0.25d |
| **T7** | `/login`, `/profile` 화면 (shadcn primitives + 해요체 i18n) + e2e (Zitadel real container) + a11y (@axe-core) | `app/(public)/login`, `app/(authenticated)/profile`, `tests/e2e/auth.spec.ts` | 0.5d |
| **T8** | `docs/auth/frontend-integration.md` 운영 SSOT | docs | 0.25d |

총 **3.75d** (≈ 4일).

---

## 8. SSS 7 기둥 매핑

| 기둥 | SP6-i 의 구체 강제 |
|---|---|
| **일관성** | 모든 protected route = same `(authenticated)/layout.tsx`. 모든 auth string = `auth.ko.json` (직접 작성 0). 모든 auth error = RFC 7807 ProblemDetails (backend 와 일관) |
| **자동 강제** | middleware (rate limit + CSP + gate), `next.config` (HSTS + CSP), lefthook (sqlx-prepare-check), edge ratelimit (코드 0 line bypass 불가) |
| **추적성** | `crates/auth/audit.rs` 가 6종 `AuthEvent` 자동 emit (Login / Logout / RefreshSucceeded / RefreshFailed / RoleGuardDenied / RoleChanged) → audit_log INSERT, payload 에 `(user_id, jti, exp)` 포함 |
| **안전성** | iron-session sealed cookie + Zitadel JWT 검증 (compile-time `crates/auth`) + zod env + `__Host-` prefix + JTI denylist (logout 후 token 즉시 무효) |
| **가시성** | `instrumentation.ts` OpenTelemetry — `oidc.authorize`, `oidc.callback`, `token.refresh`, `role.guard` span. SP7-i Sentry 가 connector 채움 |
| **SSOT** | Zitadel = identity / Redis = active session / Postgres `users.role` = authz / `users.ci` (NULL, SP6-CI) = 동일인 SSOT. 4 분리, 사본 0 |
| **명확성** | `docs/auth/frontend-integration.md` 운영 SSOT, RFC 7807 일관 error format, i18n 강제, glossary 의 `User`/`Role` 용어 사용 |

---

## 9. Testing 전략

| Layer | Tool | What |
|---|---|---|
| Unit | Vitest | `lib/oidc.ts` PKCE generation, `lib/session/single-flight.ts` mutex, `lib/observability/redact.ts` 마스킹, `lib/ratelimit.ts` sliding window |
| Integration | Vitest + msw | callback 의 token exchange (Zitadel mock), JTI denylist hit |
| Backend | cargo test | `crates/auth/jti_denylist.rs` Redis trait, `audit.rs` AuthEvent emit |
| E2E | Playwright + Zitadel container | login → /profile → logout 풀 흐름. role guard 차단 확인. CI 에서 docker compose up zitadel + redis |
| A11y | @axe-core/playwright | `/login`, `/forbidden`, `/profile` 화면 WCAG 2.1 AA |
| Security | manual + checklist | OWASP ASVS L2 — cookie flags, CSP, CSRF (state), redirect URI allowlist, redaction, rate limit |

**Coverage 목표**: 기존 90% threshold 유지 (`tarpaulin.toml` 에 `crates/auth/jti_denylist.rs`, `audit.rs` 포함).

---

## 10. Error handling (RFC 7807)

모든 `/api/auth/*` 응답이 다음 포맷:

```json
{
  "type": "https://gongzzang.com/errors/auth/state-mismatch",
  "title": "로그인 검증에 실패했어요",
  "status": 401,
  "detail": "보안을 위해 다시 로그인해 주세요.",
  "instance": "/api/auth/callback?state=..."
}
```

| 시나리오 | type | status | i18n key |
|---|---|---|---|
| Zitadel down | `auth/idp-unavailable` | 503 | `auth.errors.idp_unavailable` |
| state mismatch (CSRF) | `auth/state-mismatch` | 401 | `auth.errors.state_mismatch` |
| code_verifier cookie 만료 | `auth/session-expired` | 401 | `auth.errors.session_expired` |
| JTI denied (revoked) | `auth/token-revoked` | 401 | `auth.errors.token_revoked` |
| Rate limit | `auth/too-many-requests` | 429 | `auth.errors.rate_limit` |
| Role mismatch | `auth/insufficient-role` | 403 | `auth.errors.insufficient_role` |

---

## 11. Open questions

없음. 모든 결정은 § 2 의 결정 표에서 확정.

미래 SP 가 결정해야 할 항목 (SP6-i 시점에는 자리만):

- SP6-CI: 본인확인 vendor 선택 (NICE / Toss / PASS / 카카오 본인확인) — 비용 + UX + 약관 비교
- SP6-Social: federation 우선순위 (카카오 → 네이버 → Google) + 동일인 매칭 trigger (가입 시 즉시 vs 거래 시점)
- SP6-org: organization 모델 (broker 사무소 vs 기업 vs 개인) + 직원 invite 흐름
- SP6-iam-infra: Zitadel HA 토폴로지 (single-node vs 3-node) + DB backup 빈도 + JWKS rotation 주기

---

## 12. Reference

- [ADR-0005: Auth — Zitadel](../../adr/0005-auth-zitadel.md) (SP3 의 기초)
- [SP3 spec](./2026-05-03-sub-project-3-auth-zitadel-jwt-design.md) (backend Verifier)
- [SP6-foundation spec](./2026-05-05-sub-project-6-foundation-design.md) (Next.js + ky + i18n)
- [oauth4webapi 문서](https://github.com/panva/oauth4webapi)
- [Zitadel OIDC docs](https://zitadel.com/docs/apis/openidoauth/endpoints)
- [RFC 7807 — Problem Details for HTTP APIs](https://datatracker.ietf.org/doc/html/rfc7807)
- [OWASP ASVS L2](https://owasp.org/www-project-application-security-verification-standard/)
- [`__Host-` cookie prefix (RFC 6265bis)](https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis)

---

**다음 단계**: 사용자 review → spec 승인 → `writing-plans` skill 로 implementation plan 작성 → `subagent-driven-development` 로 T1-T8 실행.
