# ADR 0033 - 7 Guardrails 자동 강제 방법

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0030](./0030-three-service-architecture.md), [ADR 0031](./0031-platform-core-bounded-contexts.md), [ADR 0032](./0032-eventual-consistency-strategy.md) |

## 결정

γ' Three-Service Architecture 의 아키텍처 원칙을 **사람의 코드 리뷰에만 의존하지 않고**
자동화된 도구로 강제(enforce)한다. 총 7개의 Guardrail 을 정의하고, 각각에 대해
CI/CD 파이프라인에서 자동 통과/실패를 결정하는 구체적인 강제 메커니즘을 명시한다.

| # | Guardrail | 강제 방법 |
|---|---|---|
| 1 | 서비스 간 직접 DB 접근 금지 | ESLint 커스텀 규칙 |
| 2 | 서비스 간 호출은 타입된 API 계약으로만 | Zod 스키마 + 타입 검사 |
| 3 | 공유 mutable state 금지 | ESLint immutability 규칙 |
| 4 | 이벤트 스키마 버전 관리 필수 | CI 스키마 레지스트리 검증 |
| 5 | 중요 쓰기 경로에서 동기 호출 금지 | async-only 정책 검사 |
| 6 | 외부 사이드 이펙트는 Port/Adapter 뒤로 | Architecture fitness 테스트 |
| 7 | Bounded Context 소유권 CODEOWNERS 강제 | GitHub 브랜치 보호 규칙 |
## 콘텍스트

### 1. 아키텍처 부체 누적 방지

ADR 0030-0032 에서 정의한 아키텍처 원칙은 코드 리뷰 시 개발자가 인지하고 따를 것을
기대한다. 그러나 다음 이유로 자동화 강제가 필수적이다.

- **온보딩 시 지식 감**: 새 개발자가 모든 ADR 을 숙지하기 전에 원칙 위반 코드를 작성할 수 있다.
- **시간 압박**: 마감 압박 하에서 원칙을 우회하는 코드가 머지될 수 있다.
- **리뷰어 피로**: 아키텍처 원칙 준수 여부를 매번 리뷰에서 확인하면 리뷰어 부담이 증가한다.

### 2. 강제 방법 선택 기준

| 기준 | 설명 |
|---|---|
| 즉시 피드백 | IDE 또는 pre-commit 시점에 피드백 |
| CI 통합 | 모든 PR 에서 자동 실행, 실패 시 머지 차단 |
| 탈출구 | 예외가 정당한 경우 명시적 예외 선언 과정 필요 |
| 낮은 운영 비용 | ESLint, TypeScript, GitHub Actions 등 기존 도구 우선 |

### 3. 기존 위반 현황

- platform-core DB 테이블을 gongzzang3 가 직접 Drizzle 로 조회하는 코드 존재 (G1 위반)
- 서비스 간 이벤트 스키마에 버전 필드가 없어 consumer 코드 변경 시 무선언 breaking change (G4 위반)
- Workforce Context 도메인 객체가 Catalog Context 파일에 직접 import 된 사례 (G6 위반)

## 7 Guardrails 상세 구현

### Guardrail 1 — 서비스 간 직접 DB 접근 금지

**위반 예시**

**강제 방법: ESLint 커스텀 규칙**

**CI 단계**: pnpm turbo lint (PR 머지 조건)

### Guardrail 2 — 서비스 간 호출은 타입된 API 계약으로만

서비스 간 모든 요청/응답은 packages/api-contracts/ 의 Zod 스키마로 정의한다.
unknown 타입으로 받은 응답을 .parse() 없이 사용하면 TypeScript strict 모드에서 컴파일 오류 발생.

**CI 단계**: pnpm turbo typecheck (PR 머지 조건)

### Guardrail 3 — 공유 mutable state 금지

예외가 필요한 경우 eslint-disable-next-line 주석으로 명시적 예외 선언 후 PR 리뷰 필수.

**CI 단계**: pnpm turbo lint (PR 머지 조건)

### Guardrail 4 — 이벤트 스키마 버전 관리 필수

모든 도메인 이벤트는 packages/event-schemas/ 에 schemaVersion 리터럴 필드를 포함해야 한다.

CI 검증 스크립트 (scripts/validate-event-schemas.mjs) 검사 항목:

- 모든 이벤트 파일에 schemaVersion 필드 존재 여부
- 기존 버전 파일 수정 금지 (파일 해시 비교)
- Breaking change 는 .v2.ts 신규 파일 생성, consumer 2주간 이전 버전 지원

**CI 단계**: 별도 schema-registry-check job (PR 머지 조건)

### Guardrail 5 — 중요 쓰기 경로에서 동기 호출 금지

**중요 쓰기 경로 정의**: payment, auth, critical-write 로 태그된 Server Action / Use Case

Write-Through 패턴(ADR 0032 기둥 3) 예외는 @allowSyncCall 어노테이션으로 화이트리스트 처리.

**CI 단계**: pnpm turbo test --filter=@seanal/core -- --testPathPattern=arch

### Guardrail 6 — 외부 사이드 이펙트는 Port/Adapter 뒤로

외부 서비스 목록: 이메일(SES), 결제(PG), SMS, 외부 REST API, 파일 스토리지(S3)
모두 ports/ 인터페이스를 통해서만 호출.

### Guardrail 7 — Bounded Context 소유권 CODEOWNERS 강제

브랜치 보호 규칙:
- require_code_owner_reviews: true
- required_approving_review_count: 1
- Required Status Checks: lint, typecheck, test, schema-registry-check

## 예외 처리 (Escape Hatch) 프로세스

Guardrail 위반이 정당한 경우 다음 절차를 따른다.

1. GitHub Issue 생성 (Guardrail Exception Request 템플릿)
   - 위반하는 Guardrail 번호 명시
   - 기술적 이유 및 대안 검토 결과
   - 예외 적용 범위 및 만료일

2. @team-architecture 리뷰 및 승인

3. 코드에 예외 선언:
   // @guardrail-exception: G1 -- [issue: #123] [expires: 2026-08-01]

4. .guardrail-exceptions.json 에 등록

5. 만료일 도래 시 자동 알림 + 해결 PR 필수

## CI/CD 통합 요약

## 영향

### 긍정적 영향

- 아키텍처 부체 누적 속도 감소
- 개발자 학습 지원: ESLint 오류 메시지에 수정 방법 포함
- 리뷰어 부담 감소: 비즈니스 로직 리뷰에 집중 가능
- 감사 추적: 모든 예외 결정이 Issue 로 기록됨

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| 초기 설정 비용 | Guardrail 별 스캐폴딩 가이드 제공 |
| 오탐(false positive) | 예외 처리 프로세스로 지원 |
| CI 속도 증가 | arch-tests 는 변경 파일 범위만 선택적 실행 |
| 기존 위반 코드 | 신규 파일부터 적용, 기존 파일은 별도 마이그레이션 스프린트 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 아키텍처 원칙이 자동 강제되어 코드베이스 일관성 유지 |
| 2. 보안 | G1, G2 로 인젝션 및 타입 혼동 공격 표면 감소 |
| 3. 신뢰성 | G4 로 consumer 스키마 깨짐 방지 |
| 4. 관찰가능성 | 예외 처리 Issue 로 원칙 위반 히스토리 추적 가능 |
| 5. 성능 | G5 로 중요 경로 동기 블로킹 방지 |
| 6. 운영성 | G7(CODEOWNERS) 로 팀별 책임 명확화 |
| 7. 확장성 | 코드베이스 성장과 함께 자동 강제되어 확장 용이 |

## 재검토 트리거

- Guardrail 예외 처리 요청이 월 5건을 초과하는 경우 (규칙 과도 여부 검토)
- 특정 Guardrail 의 오탐률이 20% 초과하는 경우
- 새로운 서비스 추가 시 (CODEOWNERS 및 cross-service import 규칙 업데이트)

## 참고

- [ADR 0030](./0030-three-service-architecture.md) — γ' Three-Service Architecture 채택
- [ADR 0031](./0031-platform-core-bounded-contexts.md) — Platform-Core Bounded Context 분리
- [ADR 0032](./0032-eventual-consistency-strategy.md) — Eventual Consistency 전략
- [AGENTS.md](../../AGENTS.md) — SSS 7기둥 정의
- Robert Martin, "Clean Architecture" — Ports and Adapters 패턴
