# crates/auth

Zitadel JWT 검증 + RBAC 미들웨어 + 세션 관리.

## 책임
- JWT 검증 (Zitadel 발급, JWK 캐시)
- RBAC (5종 역할)
- 사용자 세션 (Redis)
- 사업자등록번호 검증 호출 (홈택스 진위확인 — sub-project 3)
- 공인중개사 자격 식별 (사업자 업종 코드)
- NICE 본인인증 (Phase 3+, sub-project 3)

## 의존
- `crates/cache` — Redis 세션
- `crates/data-clients/nice-identity` — NICE
- `crates/observability`
- `jsonwebtoken`, `argon2` (해싱), `oauth2`

## 정책
- JWT 검증 실패 = 401 + 즉시 차단 (재시도 X)
- RBAC 위반 = 403 (감사 로그)
- 토큰 만료 시 자동 refresh (clientside)
- 세션 무효화 = Redis Pub/Sub 즉시 전파
- 모든 인증 시도/성공/실패 = audit log

→ ADR-0005, → @docs/auth/README.md
