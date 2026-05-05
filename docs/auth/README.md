# auth/

인증·인가·세션 SSOT.

## 책임 영역
- Zitadel (OIDC/OAuth2 IdP)
- 소셜 로그인 (Google/Kakao/Naver/Apple — 단계적)
- NICE 본인인증
- WebAuthn / TOTP 2FA
- RBAC 권한 모델 (5종 사용자 역할)
- 사업자등록번호 검증 (홈택스 진위확인)
- 공인중개사 자격 식별 (사업자 업종 코드)
- JWT 검증 미들웨어 (Rust)
- 세션 (Redis)

## 작성 예정 문서 (sub-project 3)
- `zitadel.md` — Zitadel 운영, 커스텀 페이지
- `social-providers.md` — Google/Kakao/Naver/Apple 통합
- `nice-identity.md` — NICE 본인인증 흐름 + 비용
- `webauthn.md` — Passkey
- `rbac.md` — 권한 매트릭스 (5종 역할)
- `business-verification.md` — 사업자/중개사 검증
- `session.md` — Redis 세션 + 무효화

## Frontend 통합

- [SP6-i Frontend Integration](./frontend-integration.md) — 로컬 개발 / 디버깅 / 장애 대응

## 관련 ADR
- → @docs/adr/0005-auth-zitadel.md

## 관련 컨벤션
- → @docs/conventions/error-format.md
