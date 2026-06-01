# Sub-project 6-i Auth - Part 06B: Frontend Integration SSOT and Final Verification

Parent index: [Sub-project 6-i Auth - Part 06](./2026-05-05-sub-project-6-i-auth.part-06.md).
## Task 8: docs/auth/frontend-integration.md (운영 SSOT)

**Files:**
- Create: `docs/auth/frontend-integration.md`
- Modify: `docs/auth/README.md` (link 추가)

- [ ] **Step 8.1: docs/auth/frontend-integration.md 작성**

`docs/auth/frontend-integration.md` (full content):

````markdown
# Frontend Auth Integration — 운영 SSOT

> SP6-i 의 운영 가이드. 디버깅·장애 대응·로컬 개발 절차의 단일 출처.

## 1. 로컬 개발 환경

```bash
# 1. Zitadel + Redis dev container 시작
docker compose -f infra/zitadel/docker-compose.yml up -d

# 2. Zitadel 첫 부팅 후 (~30초 대기)
sleep 30

# 3. OIDC app 등록 (idempotent)
bash infra/zitadel/init-zitadel.sh > /tmp/zitadel.out
cat /tmp/zitadel.out  # CLIENT_ID 확인

# 4. apps/web/.env.local 작성 (CLIENT_ID 반영)
cp apps/web/.env.local.example apps/web/.env.local
# CLIENT_ID 수정

# 5. 백엔드 실행 (별도 터미널)
DATABASE_URL=postgres://gongzzang:gongzzang@localhost:5432/gongzzang \
ZITADEL_ISSUER=http://localhost:8443 \
ZITADEL_AUDIENCE=$CLIENT_ID \
REDIS_URL=redis://localhost:6379 \
cargo run -p api

# 6. 프론트엔드 실행
pnpm --filter=@gongzzang/web dev
```

브라우저로 http://localhost:3000/login → admin@zitadel.localhost / Admin123! 로 로그인.

## 2. 인증 흐름

```
사용자 → /login → POST /api/auth/login (PKCE start, tmp cookie 발급)
       → 302 → Zitadel /oauth/v2/authorize
       → 사용자 인증 → 302 → /api/auth/callback?code=&state=
       → state CSRF 검증 → token exchange → Redis session 발급 (sid)
       → Set-Cookie __Host-sid → 302 → returnTo (default /profile)
```

## 3. 디버깅

| 증상 | 원인 후보 | 확인 방법 |
|---|---|---|
| `/login` 누르면 401 state mismatch | tmp cookie 만료 (10분) 또는 SameSite | `__Host-auth-tmp` 쿠키 존재 확인 |
| `/profile` 가 무한 redirect | Redis 연결 실패 → session null | `redis-cli ping`, middleware fail-closed |
| 401 token revoked | logout 후 재사용 시도, 또는 role 변경 직후 | Redis `GET jti:deny:<jti>` 확인 |
| 403 forbidden | role 이 admin/broker 아님 | profile 화면에서 role 확인, backend `users.roles` 확인 |
| 429 rate limit | login 5/min/IP 초과 | Redis `ZRANGE rate:login:<ip> 0 -1 WITHSCORES` |

## 4. 장애 대응

### Zitadel 다운
- 기존 session 은 access_token TTL (5분) 까지 동작
- 만료 후 refresh 시도 → fail → frontend 가 /login redirect → Zitadel 다운 시 503
- 영향: 신규 로그인 + token refresh 차단. 기존 세션 처리는 가용

### Redis 다운
- `getSession` fail → middleware 가 /login redirect (closed-fail)
- JTI denylist check 도 fail → backend Verifier 가 fail-open 정책 (가용성 우선)
- audit_log emit fail → tracing::warn 로깅, 사용자 영향 없음

### Postgres 다운
- frontend 인증은 동작 (Zitadel + Redis 만 의존)
- backend `/me` 등 user 조회 실패 → 502 → frontend RFC 7807 응답

## 5. JTI denylist 운영

```bash
# 특정 jti 무효화 (관리자 수동 — role 변경 시 backend 가 자동 처리)
redis-cli SET jti:deny:<jti> 1 EX 300

# 활성 deny 목록
redis-cli KEYS "jti:deny:*"

# 사용자의 모든 활성 jti (role 변경 직전 조회)
psql -c "SELECT after_state->>'jti' FROM audit_log
         WHERE actor_id = '<user_id>'
           AND action IN ('auth.login', 'auth.refresh.succeeded')
           AND created_at > now() - interval '30 days';"
```

## 6. 모니터링 (SP7-i 통합 후)

| 메트릭 | 임계 | 의미 |
|---|---|---|
| `auth.login.failure_rate` | > 5% | Zitadel 또는 frontend 버그 |
| `auth.refresh.failure_rate` | > 1% | Zitadel down 또는 refresh_token 만료 비율 비정상 |
| `auth.role_guard.denied` | spike | 권한 설정 오류 또는 공격 |
| `redis.session.miss_rate` | > 0.1% | Redis 데이터 손실 또는 TTL 설정 오류 |

## 7. 미래 sub-project 의 자리

- **SP6-CI** (KISA 본인확인): `users.ci` 채움. NICE/Toss SDK 통합 + CI state machine.
- **SP6-Social**: 카카오/네이버/Google federation. `external_account` 가 매 provider 채워짐. 동일인 매칭 = `users.ci` UNIQUE.
- **SP6-org**: organization 분리, JWT `org_id` claim, org switcher UI.
- **SP6-iam-infra**: Zitadel self-host 의 Pulumi 코드화, JWKS rotation, DB backup, alert.

## 8. Spec / Plan 참조

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-i-auth-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-i-auth.md`
- ADR-0005: `docs/adr/0005-auth-zitadel.md`
````

- [ ] **Step 8.2: docs/auth/README.md 에 link 추가**

`docs/auth/README.md` 의 적절한 섹션에:

```markdown
## Frontend 통합

- [SP6-i Frontend Integration](./frontend-integration.md) — 로컬 개발 / 디버깅 / 장애 대응
```

- [ ] **Step 8.3: markdownlint**

```
pnpm markdownlint-cli2 docs/auth/frontend-integration.md
```

Expected: 0 errors.

- [ ] **Step 8.4: Commit**

```bash
git add docs/auth/frontend-integration.md docs/auth/README.md
git commit -m "docs(6i-T8): frontend-integration.md 운영 SSOT (로컬 개발 + 디버깅 + 장애 대응)"
```

---

## 최종 검증 (T7 완료 후)

- [ ] **Step F.1: Push + 5 CI workflow 그린 확인**

```
git push origin main
gh run list --branch main --limit 5 --json status,conclusion,name
```

Expected: 5/5 success (CI / db-migrations / walking-skeleton / api-drift-smoke-test / frontend).

- [ ] **Step F.2: smoke 사용자 검증 (수동)**

브라우저로 production-like 시나리오 1회 (로그인 → /profile → 로그아웃) 실행. 로그에 token 노출 없는지 확인 (`pnpm --filter=@gongzzang/web start` + log inspection).

- [ ] **Step F.3: SP6-i 완료 보고 + 다음 sub-project 의향 묻기**

다음 후보 (사용자가 결정):
- SP6-org: multi-org switcher
- SP6-CI: 본인확인 SDK
- SP6-Social: 카카오/네이버 federation
- SP6-ii: 매물 검색 화면 (auth 가 깔린 후 첫 사용자 가치)
- SP6-iam-infra: Zitadel Pulumi (production 배포 직전)

---

## Spec 커버리지 자가 점검

| Spec § | 요구사항 | 구현 task |
|---|---|---|
| 2.1 Zitadel self-host | dev docker-compose | T1 |
| 2.1 OIDC PKCE oauth4webapi | lib/oidc.ts | T3 |
| 2.1 Redis backed session | lib/session/store.ts | T2 |
| 2.1 __Host- cookie + Partitioned | lib/session/cookie.ts | T2 |
| 2.1 Refresh single-flight | lib/session/single-flight.ts + /api/auth/refresh | T2, T3 |
| 2.1 Back-channel logout | /api/auth/logout | T3 |
| 2.1 Path 분기 RBAC | middleware.ts | T4 |
| 2.1 JTI denylist | crates/auth/jti_denylist.rs + middleware hook | T5 |
| 2.1 Role 즉시반영 | audit_log 의 jti 인덱스 활용 (SP6-iv 가 admin UI 추가) | T5 (자리) |
| 2.1 Rate limit | lib/ratelimit.ts + middleware.ts | T4 |
| 2.1 CSP/HSTS | middleware.ts (CSP) + next.config.ts (HSTS) | T4 |
| 2.1 Log redaction | lib/observability/redact.ts | T4 |
| 2.1 lefthook sqlx prepare check | lefthook.yml | T6 |
| 2.1 Audit emit | crates/auth/audit.rs + /internal/auth/event | T5 |
| 2.1 OTel span | lib/observability/tracer.ts + instrumentation.ts | T4 |
| 2.1 RFC 7807 | lib/http/problem.ts (모든 /api/auth/* 응답) | T2, T3 |
| 2.1 i18n auth.ko.json | messages/auth.ko.json | T3 |
| 2.1 a11y WCAG 2.1 AA | tests/e2e/auth.spec.ts (axe-core) | T7 |
| 2.1 V004 schema 자리 | migrations/30008_user_ci_external_account.sql | T6 |
| 5 V004 SQL | 동일 | T6 |
| 6 디렉토리 구조 | T1-T8 분산 | 전체 |
| 8 SSS 7 기둥 | 일관성/자동강제/추적성/안전성/가시성/SSOT/명확성 모두 강제 코드 | T1-T8 |
| 9 Testing 전략 | unit + integration + e2e + a11y | T2-T7 |
| 10 RFC 7807 error 표 | lib/http/problem.ts + i18n auth.errors.* | T2, T3 |

**미반영 = 0**. 모든 spec 요구사항이 task 1개 이상에 매핑됨.
