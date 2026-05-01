# ADR-0006: API — REST + OpenAPI 3.1 (utoipa + openapi-typescript)

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

Rust 백엔드 ↔ Next.js 프론트 (TS) 통신. SSS 핵심 = 타입 동기화 자동화 + 임시방편 0. v2의 `ALLOWED_FOR_FRONTEND_TEMP` 같은 안티패턴 회피. 향후 모바일 네이티브 앱(Phase 2+) + B2B API 판매(Phase 3+, 수익 모델 D)도 같은 spec 사용.

## 결정

- **스타일**: REST
- **스펙**: OpenAPI 3.1
- **버저닝**: URL (`/v1/...`)
- **에러 형식**: RFC 9457 Problem Details
- **Rust 측**: utoipa (코드에서 자동 spec 생성)
- **TS 측**: openapi-typescript + openapi-fetch (자동 타입 생성)
- **린트**: Spectral 또는 Vacuum (CI 차단)
- **계약 테스트**: Pact (sub-project 5+)
- **인증**: Bearer JWT (Zitadel)

## 대안

- **GraphQL**: 클라이언트 유연성, 그러나 큰 페이로드(공간 데이터) 캐싱 어려움, 권한 세밀 제어 복잡
- **gRPC**: 빠름·타입 강함, 그러나 브라우저 직접 호출 어려움(grpc-web 필요), 디버깅 도구 약함
- **tRPC**: 풀 TS만 의미 있음 — Rust 백엔드 부적합
- **Orval (TS 클라이언트 생성기)**: v2에서 사용, 그러나 수동 조정 필요(안티패턴) → openapi-typescript가 더 깔끔

## 결과

- 긍정: 컴파일 시점 타입 불일치 검출 (Rust 변경 → openapi.json → TS 컴파일 실패), 자동 SDK (모바일/B2B 무료), Swagger UI 자동, 캐싱(CDN/ETag) 표준
- 부정: 페이로드 페이지네이션 등 클라이언트 책임, 일부 *연관 데이터*는 N+1 호출 (resolver 패턴 별도)
- 영향 영역: `crates/api-types/`, `services/api/`, `packages/api-client/`, `apps/*/`

## 재검토 트리거

- B2B 고객이 GraphQL 강력 요구 시 (수익 모델 D 진입 후)
- 모바일 네이티브 앱이 작은 페이로드 다수 호출로 배터리 영향 → BFF 추가
- gRPC 내부 마이크로서비스 통신 필요 시 (Phase 4+)

## 참조

- → @docs/api/openapi.md (작성 예정)
- → @docs/conventions/error-format.md
- → @docs/api/utoipa.md
- RFC 9457 (Problem Details): https://www.rfc-editor.org/rfc/rfc9457
