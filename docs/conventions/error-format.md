# 에러 형식 컨벤션 (RFC 9457 Problem Details)

## 1. 표준

[RFC 9457 — Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457). 모든 에러 응답은 이 형식.

## 2. 응답 본문

```json
{
  "type": "https://gongzzang.com/errors/listing-not-found",
  "title": "매물을 찾을 수 없어요",
  "status": 404,
  "detail": "ID 'lst_01HXY...'에 해당하는 매물이 없어요",
  "instance": "/v1/listings/lst_01HXY...",
  "correlationId": "01HXYZK...",
  "code": "LISTING_NOT_FOUND",
  "errors": []
}
```

## 3. 필드 규칙

| 필드 | 필수 | 형식 | 설명 |
|------|------|------|------|
| `type` | ✅ | URL | `https://gongzzang.com/errors/<kebab-case>` |
| `title` | ✅ | 한국어 (해요체) | 사용자 노출 가능 |
| `status` | ✅ | HTTP 코드 | 4xx / 5xx |
| `detail` | ✅ | 한국어 | 구체 정보 (PII 마스킹) |
| `instance` | ✅ | URI | 요청 경로 |
| `correlationId` | ✅ | ULID | 추적 ID (req → resp 전 체인) |
| `code` | ✅ | `SCREAMING_SNAKE_CASE` | 도메인 에러 enum 값 |
| `errors` | 선택 | 배열 | 검증 에러 (필드별) |

## 4. errors 배열 (검증 에러)

```json
{
  "errors": [
    { "field": "priceKrw", "code": "INVALID_RANGE", "message": "가격은 0보다 커야 해요" },
    { "field": "pnu", "code": "INVALID_FORMAT", "message": "PNU는 19자리여야 해요" }
  ]
}
```

## 5. 에러 코드 매핑 (Rust enum SSOT)

```rust
// crates/api-types/src/error.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 4xx — 클라이언트
    ListingNotFound,
    PnuInvalidFormat,
    BusinessNumberRequired,
    UnauthorizedAccess,
    InsufficientPermission,
    RateLimitExceeded,

    // 5xx — 서버 / 외부
    VworldUnavailable,
    KoreanLawApiUnavailable,
    DatabaseTransient,
    InternalServerError,
}
```

→ utoipa가 자동 OpenAPI spec → openapi-typescript가 자동 TS 타입.

## 6. HTTP 상태 코드 매핑

| 상태 | 의미 | 에러 코드 예시 |
|------|------|--------------|
| 400 | 잘못된 요청 | `INVALID_REQUEST_BODY`, `PNU_INVALID_FORMAT` |
| 401 | 인증 필요 | `UNAUTHORIZED_ACCESS`, `INVALID_TOKEN` |
| 403 | 권한 없음 | `INSUFFICIENT_PERMISSION`, `BROKER_LICENSE_REQUIRED` |
| 404 | 리소스 없음 | `LISTING_NOT_FOUND`, `USER_NOT_FOUND` |
| 409 | 충돌 | `OPTIMISTIC_LOCK_CONFLICT`, `DUPLICATE_BUSINESS_NUMBER` |
| 422 | 검증 실패 | `VALIDATION_FAILED` (errors 배열) |
| 429 | Rate Limit | `RATE_LIMIT_EXCEEDED` |
| 500 | 서버 에러 | `INTERNAL_SERVER_ERROR` |
| 502 | 외부 API 실패 | `VWORLD_UNAVAILABLE`, `KOREAN_LAW_API_UNAVAILABLE` |
| 503 | 일시 불가 | `MAINTENANCE_MODE`, `CIRCUIT_BREAKER_OPEN` |

## 7. 메시지 작성 룰

- **한국어 해요체** ([ui-writing-korean.md](./ui-writing-korean.md))
- **원인 + 대응** ("응답이 늦어요. 잠시 후 다시 시도해 주세요")
- **PII 마스킹** (이메일·주민번호·전화번호·사업자번호 X)
- **기술 용어 회피** (사용자 노출용)

| ❌ 나쁨 | ✅ 좋음 |
|--------|--------|
| "Internal Server Error" | "일시적인 문제가 있어요. 잠시 후 다시 시도해 주세요" |
| "User john@a.com not found" | "사용자를 찾을 수 없어요" (이메일 마스킹) |
| "VWorld 503" | "토지 정보 서비스가 응답하지 않아요. 잠시 후 다시 시도해 주세요" |
| "DB connection failed" | "잠시 후 다시 시도해 주세요" |

## 8. correlation ID 추적

- 모든 요청에 `X-Correlation-Id` 헤더 자동 주입 (없으면 생성)
- 외부 API 호출 시 동일 ID 전파
- audit log + 모든 로그에 포함
- 사용자에게 노출 (에러 화면) → 고객 지원 시 추적

## 9. 자동 강제

- Rust: `crates/api-types/src/error.rs` 단일 SSOT
- Axum 미들웨어: 모든 에러 → Problem Details 변환
- 변환 누락 = utoipa 컴파일 실패 (모든 endpoint에 명시 의무)
- TS: openapi-typescript 자동 타입 → 클라이언트 컴파일 시점 검증
