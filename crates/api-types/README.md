# crates/api-types

OpenAPI에 노출되는 모든 타입 + 에러 코드 SSOT.

## 책임
- DTO (Request / Response)
- 도메인 모델 → DTO 변환 (TryFrom 구현)
- 에러 enum (`ErrorCode`) — RFC 9457 Problem Details 매핑
- utoipa 매크로로 OpenAPI 자동 생성
- ts-rs 또는 openapi-typescript로 TS 자동

## 의존
- `crates/domain/shared-kernel` (값 객체)
- `crates/domain/*` (도메인 모델 변환)
- `serde`, `utoipa`, `garde` (요청 검증)

## 정책
- *모든* HTTP endpoint의 요청/응답 타입 = 이 crate에 정의
- 에러 enum = 단일 SSOT (`error.rs`)
- 도메인 모델을 *직접* 노출 X (반드시 DTO 거침 — ACL)
- 필드 네이밍: `serde(rename_all = "camelCase")` (TS와 통일)
- 응답에 `correlationId` 자동 포함 (envelope wrapper)

→ ADR-0006, → @docs/conventions/error-format.md
