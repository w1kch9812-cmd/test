# @gongzzang/data-clients

외부 공공 API HTTP 클라이언트 + Anti-Corruption Layer.

## 클라이언트 (계획)

| 클라이언트 | 외부 API | Port 구현 |
|-----------|---------|----------|
| `VWorldClient` | api.vworld.kr | `LandInfoProvider` |
| `KoreanLawClient` | open.law.go.kr | `LegalInfoProvider` |
| `OpenDataClient` | apis.data.go.kr | `OpenDataProvider` |
| `JusoClient` | business.juso.go.kr | `AddressProvider` (추후) |
| `SgisClient` | sgis.kostat.go.kr | `StatsProvider` (추후) |

## 공통 인프라

- **HttpClient** (fetch 기반, 재시도/타임아웃)
- **RateLimiter** (V-World 쿼터 보호)
- **CircuitBreaker** (외부 API 장애 시 빠른 실패)
- **ResponseCache** (Redis 연동, TTL 정책별 다름)
- **AuditLogger** (모든 외부 호출 감사 로그)

## 원칙

- 외부 API의 응답을 **도메인 모델로 변환** 후 반환 (ACL)
- LLM/MCP 의존성 0
- 응답 raw도 보존 (감사용 JSONB)
- "Honest failure" — 에러 마스킹 금지

상세: [docs/architecture/](../../docs/architecture/README.md), [docs/data-sources/](../../docs/data-sources/README.md)
