# api/

API 계약 SSOT (REST + OpenAPI).

## 책임 영역
- REST API 설계
- OpenAPI 3.1 자동 생성 (utoipa)
- TypeScript SDK 자동 생성 (openapi-typescript + openapi-fetch)
- API 버저닝 (`/v1/`)
- 에러 형식 (RFC 9457)
- Rate Limiting + Quota
- 계약 테스트 (Pact, sub-project 5+)
- API Gateway (Traefik 또는 자체 Axum)

## 작성 예정 문서 (sub-project 5)
- `openapi.md` — utoipa 사용 패턴
- `ts-codegen.md` — openapi-typescript 자동화
- `versioning.md` — URL 버저닝 + 마이그레이션
- `error-format.md` (또는 conventions로 위임)
- `rate-limit.md` — 사용자/IP/API key 별 quota
- `pact.md` — 계약 테스트
- `gateway.md` — Traefik 설정

## 관련 ADR
- → @docs/adr/0006-api-rest-openapi.md

## 관련 컨벤션
- → @docs/conventions/error-format.md
