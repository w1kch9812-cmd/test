# docs/

도메인별 SSOT 문서 트리. **한 폴더 = 한 도메인**, 각 폴더의 README가 인덱스.

## 학습 순서 (새로 합류한 분 기준)

| # | 문서 | 내용 |
|---|------|------|
| 1 | [sss-charter.md](./sss-charter.md) | 7 기둥 SSS 헌법 — *모든 작업의 측정 자* |
| 2 | [glossary.md](./glossary.md) | 한·영 도메인 용어 사전 |
| 3 | [ssot-matrix.md](./ssot-matrix.md) | 정보별 SSOT + 위반 자동 차단 룰 |
| 4 | [conventions/](./conventions/README.md) | 코드 스타일 + 네이밍 + 에러 형식 |
| 5 | [data-sources/](./data-sources/README.md) | 외부 공공 API 카탈로그 |
| 6 | [adr/](./adr/README.md) | 모든 기술·아키텍처 결정 이력 |

## 도메인 카테고리

| 카테고리 | 책임 | 작성 시점 |
|---------|------|---------|
| [infrastructure/](./infrastructure/README.md) | IaC (Pulumi), Kubernetes, GitOps, CI/CD, 배포 | sub-project 8 |
| [auth/](./auth/README.md) | Zitadel, OIDC/OAuth2, RBAC, NICE 본인인증, WebAuthn | sub-project 3 |
| [data/](./data/README.md) | Postgres + PostGIS, 마이그레이션, CDC, 데이터 카탈로그, retention | sub-project 2 |
| [cache-messaging/](./cache-messaging/README.md) | moka L1, Valkey L2, SQS, Outbox 패턴 | sub-project 4 |
| [backend/](./backend/README.md) | Axum, SQLx, DDD, Circuit Breaker, Idempotency | sub-project 5 |
| [api/](./api/README.md) | OpenAPI, utoipa, ts-codegen, 버저닝, Pact | sub-project 5 |
| [observability/](./observability/README.md) | OTel + Sentry + Grafana + Loki + Tempo + SLO | sub-project 7 |
| [security/](./security/README.md) | OWASP ASVS, PIPA, ISMS-P, PII 마스킹, 암호화, SAST/DAST | 전반 |
| [testing/](./testing/README.md) | 단위/통합/E2E/property/mutation/load/chaos/contract | 전반 |
| [frontend/](./frontend/README.md) | Next.js, shadcn/Radix, TanStack Query, Naver Maps, PWA, a11y | sub-project 6 |
| [governance/](./governance/README.md) | ADR, CODEOWNERS, Changesets, Renovate, DORA, C4 | 전반 |
| [compliance/](./compliance/README.md) | PIPA, ISMS-P, SOC 2, audit log retention, 공공데이터 라이선스 | Phase 3+ |
| [cost/](./cost/README.md) | Phase별 AWS 비용 추정, RI/Spot 전략 | 전반 |

## SSOT 원칙

- 한 정보는 *한 폴더*에만 작성
- 다른 곳에서 필요하면 `→ @docs/<domain>/<topic>.md` 링크
- 중복 검출 = CI 차단 (lefthook + markdownlint + 자체 lint)

## superpowers/

본 프로젝트의 brainstorming → spec → plan → 실행 산출물:

- [superpowers/specs/](./superpowers/specs/) — 각 sub-project의 디자인 spec
- [superpowers/plans/](./superpowers/plans/) — 각 sub-project의 implementation plan

## 작성 규칙

1. 모든 .md ≤500줄. 초과 시 폴더로 분해.
2. 모든 도메인 폴더에 `README.md` 필수.
3. 다른 문서 참조는 명시적 Markdown 링크 (`[text](path.md)` 또는 `@AGENTS.md` 자동 import)
4. 한국어 본문 + 영어 코드 식별자 (glossary 매핑 강제)
