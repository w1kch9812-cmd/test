# crates/data-clients

외부 공공 API + 상용 API HTTP 클라이언트 + Anti-Corruption Layer.

## 하위 모듈 (sub-project 4+)
- `vworld/` — V-World API
- `data-go-kr/` — 공공데이터포털
- `korean-law/` — 법제처 Open API
- `nice-identity/` — NICE 본인인증 (Phase 3+)
- `naver-maps/` — Naver Maps (서버 사이드 지오코딩)
- `gemini-embedding/` — Google Gemini Embedding (Phase 3+)

## 의존
- `crates/api-types` — 도메인 모델 + 에러 타입
- `crates/circuit-breaker` — 표준 외부 호출 미들웨어
- `crates/observability` — tracing + OTel
- `reqwest` (HTTP), `serde`, `garde` (응답 검증)

## 정책
- 모든 외부 호출에 Circuit Breaker + Retry + Timeout + Audit log
- raw_response *항상* 보존 (DB JSONB 컬럼)
- 응답을 *도메인 모델로 변환* 후 반환 (ACL — 외부 스키마가 도메인에 누출 안 됨)
- "Honest failure" — 5xx는 Mock으로 덮지 말 것
- API 키 = `crates/auth/secrets.rs` 또는 환경변수만

→ ADR-0006, → @docs/data-sources/, → @docs/backend/circuit-breaker.md
