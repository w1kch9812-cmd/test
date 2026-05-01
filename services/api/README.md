# services/api

Rust HTTP API 서버 (Axum). 모든 도메인 로직 진입점.

## 의존
- `crates/domain/*` — 비즈니스 규칙
- `crates/data-clients/*` — 외부 API
- `crates/db` — Repository 구현
- `crates/auth` — JWT 검증
- `crates/cache` — moka L1 + Valkey L2
- `crates/observability` — tracing + OTel
- `crates/api-types` — utoipa OpenAPI 타입

## 정책
- 모든 endpoint에 utoipa 매크로 (OpenAPI 자동)
- 모든 외부 호출 Circuit Breaker 적용
- 모든 쓰기에 Idempotency-Key 헤더
- 모든 응답에 `correlationId` 포함
- 에러 = RFC 9457 Problem Details

## 진입점 (sub-project 5+)
- `/v1/listings/*` — 매물
- `/v1/parcels/{pnu}` — 필지
- `/v1/buildings/*` — 건축물
- `/v1/manufacturers/*` — 제조업체
- `/v1/laws/*` — 법령
- `/v1/auth/*` — Zitadel 위임
- `/docs/openapi.json` — 자동 spec
- `/docs` — Swagger UI
