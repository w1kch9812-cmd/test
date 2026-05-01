# ADR-0005: 인증 IdP — Zitadel

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

5종 사용자 역할 (매수자 / 매도자 / 중개사 / 시행사 / 기업), 사업자등록번호 검증 + 중개사 자격 식별, 향후 multi-tenant (대기업 임직원 관리) 가능성, 한국 시장 ISMS-P 준비. Rust 백엔드와 OIDC/OAuth2 표준 연동 필요.

## 결정

- **IdP**: Zitadel (셀프호스트, 별도 ECS 서비스)
- **프로토콜**: OIDC + OAuth 2.0
- **세션**: Redis 저장
- **2FA**: TOTP + WebAuthn (Phase 2)
- **소셜 로그인 (Phase 단계적)**: Google → Kakao → Naver → Apple
- **본인인증**: NICE 별도 통합 (sub-project 3 결정)
- **Rust 통합**: jsonwebtoken crate로 토큰 검증

## 대안

- **Keycloak**: 12년 검증, CNCF 후원, 한국 사례 다수. 단점: JVM 무거움(1GB+), 운영 부담, multi-tenancy 어색
- **자체 JWT (Rust + argon2)**: 가벼움, 그러나 보안 검증 부담 — SSS 엔터프라이즈에 미달
- **Auth0 / Clerk SaaS**: DX 최고, 비용·록인·한국 소셜 약함
- **Supabase Auth**: 가벼움, 록인 + 한국 카카오/네이버 약함
- **ORY (Kratos+Hydra+Keto)**: 모듈러, 학습 곡선 큼

## 결과

- 긍정: Go 기반 가벼움(~150MB), API-first, multi-tenancy 1급, Rust 백엔드와 일관 철학, 운영 부담 낮음, OIDC 표준 → 미래 전환 가능 (Keycloak 등)
- 부정: 한국 사례 적음(영어권 자료에 의존), ISMS-P 평가관 익숙도 미검증, Keycloakify 같은 UI 커스텀 도구 부재 (자체 React 페이지 필요)
- 영향 영역: `crates/auth/`, `services/api/` (미들웨어), `apps/*/` (NextAuth 클라이언트), 인프라 (별도 ECS)

## 재검토 트리거

- ISMS-P 평가 시 평가관이 Keycloak 강하게 선호하면 전환 검토 (1-2주 작업)
- 운영 6개월 후 Zitadel 한국 트러블슈팅 자료 부족으로 장애 대응 어려움 시
- multi-tenancy 패턴을 결국 안 쓰게 됐을 시 (Keycloak도 충분)
- Zitadel 회사 사업 위기 (장기 안정성 위협)

## 참조

- → @docs/auth/README.md (작성 예정)
- → @docs/auth/zitadel.md
- → @docs/data-sources/nice-identity.md
