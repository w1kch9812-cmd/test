# ADR 0032 - Eventual Consistency 전략

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0030](./0030-three-service-architecture.md), [ADR 0031](./0031-platform-core-bounded-contexts.md) |

## 결정

γ' Three-Service Architecture 에서 서비스 간 데이터 일관성을 보장하기 위해
**네 가지 기둥(Four-Pillar)** 으로 구성된 Eventual Consistency 전략을 채택한다.

| 기둥 | 적용 시나리오 | 일관성 보장 |
|---|---|---|
| 1. Read-Through Cache | 조회 빈도 높은 공유 데이터 | Eventual (TTL 기반) |
| 2. Transactional Outbox | 서비스 간 도메인 이벤트 전달 | At-least-once |
| 3. Write-Through | 결제, 인증 등 중요 경로 | Strong (동기 확인) |
| 4. Optimistic UI + Rollback | 사용자 인터랙션 응답성 | Eventual (충돌 시 보상) |

## 컨텍스트

### 1. 분산 시스템 일관성 문제

ADR 0030 에서 세 서비스로 분리함에 따라, 단일 DB 트랜잭션으로 여러 서비스의 상태를
원자적으로 변경하는 것이 불가능해졌다. 실제 발생하는 시나리오:

- **catalog ETL → consumer 캐시**: platform-core 가 V-World/data.go.kr 에서 산단 마스터
  데이터를 갱신(`IndustrialComplex.area_m2` 변경 등) 하면, gongzzang.com 의 지도와
  Dawneer workbench 가 변경된 사실을 반영해야 한다.
- **parcel kind 변경 → 산단 운영**: platform-core 에서 한 필지의 `kind` 가
  `공장용지` → `지원시설` 로 갱신되면, dawneer 의 `ParcelIndustryGroup` 매핑이 영향을
  받고, gongzzang 의 listing 검색 인덱스도 재계산되어야 한다.
- **Staff 역할 변경 → 권한 캐시**: platform-core Workforce 에서 직원 역할이 변경되면,
  gongzzang admin 과 dawneer 의 로컬 role cache 가 60초 이내에 무효화되어야 한다.
- **B2B 산단 컨텐츠 → B2C 노출**: dawneer 에서 운영자가 `Site` (산단 공식 사이트) 의
  메타데이터를 발행하면, gongzzang 의 산단 상세 페이지에 해당 링크가 표시되어야 한다.

### 2. 일관성 보장 수준 요건

모든 데이터가 Strong Consistency 를 요구하지는 않는다.

| 데이터 종류 | 허용 지연 | 채택 기둥 |
|---|---|---|
| 산단/필지 마스터 데이터 (읽기) | 최대 5분 | Read-Through Cache |
| Staff 역할/권한 (읽기) | 최대 60초 | Read-Through Cache |
| 도메인 이벤트 (산단 갱신, parcel kind 변경, 도면 발행) | 수 초 이내 | Transactional Outbox |
| 산단 생성·아카이브 commit | 0초 (즉시) | Write-Through |
| Zitadel ID Token 검증 | 0초 (즉시) | Write-Through |
| 사용자 UI 인터랙션 (도면 편집, IndustryGroup CRUD) | 즉시 응답 (충돌 허용) | Optimistic UI |

### 3. 기존 접근 방식의 문제

현재 서비스 간 데이터 전달은 **직접 DB 조회** 또는 **동기 API 폴링** 에 의존하고 있어
다음 문제가 발생한다.

- platform-core 장애 시 gongzzang / dawneer 기능 전체 중단
- N+1 쿼리 패턴으로 인한 성능 저하
- 롤백 없는 부분 업데이트로 인한 데이터 불일치 상태 누적

## 검토한 옵션

### 옵션 A — 2PC (Two-Phase Commit)

**채택 불가**. 분산 트랜잭션은 코디네이터 장애 시 무한 블로킹 위험이 있으며,
현재 기술 스택(PostgreSQL + Next.js)에서 2PC 구현은 복잡도 대비 효용이 낮다.
CAP 정리상 가용성(A)보다 일관성(C)을 선택하는 트레이드오프로, 이 시스템의 가용성 목표에 부합하지 않는다.

### 옵션 B — SAGA 패턴 (Choreography)

**부분 채택**. SAGA 의 이벤트 기반 보상 트랜잭션 아이디어를 Transactional Outbox 기둥에
녹여 채택한다. 순수 SAGA 오케스트레이터는 운영 복잡도가 높아 현재 팀 규모에 적합하지 않다.

### 옵션 C — Four-Pillar Eventual Consistency (채택)

각 데이터 종류의 일관성 요건에 맞는 기둥을 선택적으로 적용한다.
단일 패턴을 모든 케이스에 강요하는 대신, 요건에 맞는 최소한의 복잡도를 사용한다.

## 채택 (C) — 네 기둥 상세 구현

### 기둥 1 — Read-Through Cache

**대상**: Catalog `IndustrialComplex` / `Parcel` 마스터 데이터, Workforce `Staff` 역할/권한 목록

```
Consumer (gongzzang / dawneer)
  |
  +--> 로컬 인메모리 캐시 (TTL: Authz=60s, Catalog=300s)
          |-- hit  --> 반환
          |-- miss --> platform-core API 조회 --> 캐시 갱신 --> 반환
```

**구현 규칙**

- 캐시 키: `{context}:{entityType}:{id}:{version}` 예) `catalog:industrial_complex:KR1101:42`
  (버전 포함으로 stale 탐지)
- TTL 만료 후 백그라운드 revalidation (stale-while-revalidate 패턴)
- platform-core 에서 이벤트 발행 시 consumer 캐시 강제 무효화 (push invalidation):
  - `IndustrialComplexUpdated` → `catalog:industrial_complex:{pnu}:*` invalidate
  - `ParcelKindChanged` → `catalog:parcel:{pnu}:*` invalidate
  - `StaffRoleAssigned` → `workforce:staff:{staff_id}:*` invalidate

**장애 모드**

| 장애 | 대응 |
|---|---|
| platform-core 응답 지연 | 캐시 stale 데이터 서빙 + 백그라운드 retry |
| 캐시 서버 장애 | platform-core 직접 조회 (fallback) |
| TTL 동안 데이터 변경 | 이벤트 기반 즉시 무효화 (Guardrail #4 위반 방지) |

### 기둥 2 — Transactional Outbox

**대상**: 도메인 이벤트
- platform-core Catalog: `IndustrialComplexCreated`, `IndustrialComplexUpdated`,
  `ParcelKindChanged`, `BuildingUpserted`, `ManufacturerLinked`
- platform-core Workforce: `StaffInvited`, `StaffRoleAssigned`, `StaffSessionRevoked`
- dawneer: `SitePublished`, `SiteDeploymentCompleted` (gongzzang 이 산단 상세에서 노출)
- gongzzang: `ListingPublished`, `RealTransactionIngested` (관리·분석 용도)

```
서비스 A (Producer)                         서비스 B (Consumer)
  |                                                |
  +--> DB 트랜잭션 시작                              |
  |     +-- 비즈니스 상태 변경                       |
  |     +-- outbox 테이블 이벤트 삽입               |
  +--> DB 트랜잭션 커밋                              |
  |                                                |
  +--> Outbox Relay Worker (주기적 폴링)             |
          +-- outbox 미처리 이벤트 조회              |
          +-- 메시지 브로커 (또는 HTTP POST) 발행 --->+
          +-- outbox 이벤트 processed 마킹           |
                                             이벤트 처리
                                             (idempotent)
```

**보장**

- **At-least-once**: DB 커밋과 이벤트 삽입이 원자적이므로 발행 누락 없음
- **Idempotent Consumer**: 중복 수신 대비 이벤트 ID 기반 멱등성 처리 필수
- **순서 보장**: 동일 aggregate 의 이벤트는 sequence 번호 기반 순서 처리

**재시도 전략**

```
초기 지연: 1초
최대 재시도: 5회
백오프: 지수 (1s, 2s, 4s, 8s, 16s)
DLQ(Dead Letter Queue): 5회 실패 후 이동, 알림 발송
```

**장애 모드**

| 장애 | 대응 |
|---|---|
| Relay Worker 다운 | 다음 재시작 시 미처리 이벤트 자동 재발행 |
| Consumer 처리 실패 | 지수 백오프 재시도 -> DLQ |
| 이벤트 스키마 불일치 | ADR 0033 Guardrail #4 로 사전 차단 |

### 기둥 3 — Write-Through

**대상**: Zitadel ID Token 검증, 산단 신규 등록, Staff 역할 부여 — 즉시 일관성이
필요한 쓰기 경로. 결제 흐름은 현재 두 서비스 모두에 없음 (재검토 트리거 §재검토 참조).

```
Client --> dawneer (요청: 신규 산단 등록)
             |
             +--> platform-core /catalog/complexes (동기 API, 타임아웃 3초)
             |         |
             |    응답: 201 + ComplexId (성공) | 409 (중복 PNU) | 5xx (실패)
             |
             +--> dawneer 로컬 DB 의 InteractiveBlueprint 레코드를 ComplexId 로 연결
             |
             +--> Client 응답
```

**규칙**

- 동기 호출 타임아웃: 3초 (초과 시 실패 처리, 재시도 없음)
- Circuit Breaker: 5초 내 실패율 50% 초과 시 open (30초 후 half-open 전환)
- Write-Through 경로의 실패는 **전체 트랜잭션 롤백** (부분 성공 금지)

**장애 모드**

| 장애 | 대응 |
|---|---|
| platform-core 타임아웃 | 요청 실패 반환, 클라이언트 재시도 유도 |
| Circuit Breaker open | 빠른 실패(fail-fast) + 사용자에게 일시적 오류 안내 |
| 부분 커밋 상태 | 불가능: 트랜잭션 원자성 보장 |

### 기둥 4 — Optimistic UI + Rollback

**대상**: 사용자 인터랙션 (도면 폴리곤 편집, IndustryGroup 트리 재구성, ParcelIndustryGroup
할당, Site 메타데이터 편집, KakaoNotification 템플릿 저장 등)

```
User Action
  |
  +--> 즉시 UI 업데이트 (낙관적)
  |
  +--> 백그라운드 서버 요청
          |-- 성공 --> UI 상태 확정 (변경 없음)
          |-- 충돌 --> UI 롤백 + 충돌 해결 안내 토스트
          |-- 실패 --> UI 롤백 + 오류 토스트
```

**충돌 감지**

- Optimistic Locking: DB 레코드에 `version` 필드 유지
- 요청 시 클라이언트가 알고 있는 version 을 함께 전송
- 서버에서 version 불일치 시 `409 Conflict` 반환
- 클라이언트는 최신 서버 상태로 UI 재동기화

**구현 예시**

```typescript
// dawneer IndustryGroup 트리 노드 이름 편집 — Optimistic UI + 409 충돌 처리
function useOptimisticIndustryGroupRename(group: IndustryGroupRef) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (updated: IndustryGroupRef) =>
      renameIndustryGroup({
        groupId: updated.id,
        name: updated.name,
        ifMatchVersion: updated.version,  // ← optimistic locking 키
      }),
    onMutate: async (updated) => {
      await queryClient.cancelQueries({ queryKey: ['industry-group', updated.complexId] });
      const previous = queryClient.getQueryData(['industry-group', updated.complexId]);
      queryClient.setQueryData(['industry-group', updated.complexId], (old: IndustryGroupTree | undefined) =>
        old ? renameNodeInTree(old, updated.id, updated.name) : old,
      );
      return { previous };
    },
    onError: (err, updated, context) => {
      queryClient.setQueryData(['industry-group', updated.complexId], context?.previous);
      if (err instanceof ConflictError) {
        // 다른 운영자가 먼저 수정 — 최신 트리 강제 새로고침
        queryClient.invalidateQueries({ queryKey: ['industry-group', updated.complexId] });
        toast.error('다른 운영자가 먼저 수정했어요. 최신 상태를 불러왔어요.');
      } else {
        toast.error('이름 변경에 실패했어요. 다시 시도해 주세요.');
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['industry-group'] });
    },
  });
}
```

## 일관성 보장 매트릭스

| 시나리오 | 기둥 | 최대 불일치 시간 | 사용자 체감 |
|---|---|---|---|
| 산단/필지 카탈로그 조회 | Cache | 5분 (push invalidation 시 즉시) | 비가시적 |
| Staff 역할 변경 후 접근 | Cache + Outbox | 60초 | 비가시적 |
| 신규 산단 등록 commit | Write-Through | 0초 | 즉각 반영 |
| IndustryGroup 트리 편집 | Optimistic UI | 즉각 (충돌 시 롤백) | 즉각 응답 |
| Site 발행 → gongzzang 노출 | Outbox | 수 초 | 비가시적 |
| ParcelKindChanged → 검색 인덱스 | Outbox | 수 초 | 비가시적 |

## 모니터링 요건

| 메트릭 | 임계값 | 알림 |
|---|---|---|
| Cache hit rate | < 80% | P2 알림 |
| Outbox 미처리 이벤트 수 | > 100건 | P1 알림 |
| Outbox 재시도 실패율 | > 5% | P1 알림 |
| Write-Through 타임아웃율 | > 1% | P0 알림 |
| Optimistic conflict rate | > 2% | P2 알림 |

## 영향

### 긍정적 영향

- 서비스 장애 격리: platform-core 장애 시에도 캐시 데이터로 읽기 서비스 유지
- 사용자 경험: Optimistic UI 로 인터랙션 응답성 향상
- 운영 투명성: Outbox 테이블이 이벤트 발행 감사 로그 역할

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| 캐시 무효화 복잡도 | 이벤트 기반 push invalidation 으로 자동화 |
| Outbox 테이블 성장 | 처리 완료된 이벤트 주기적 정리 크론잡 (ADR 0028 참조) |
| Optimistic conflict 사용자 혼란 | 명확한 충돌 해결 UI 가이드라인 제공 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 기둥별 명확한 구현 패턴으로 일관된 코드베이스 |
| 2. 보안 | Write-Through 경로에서 인증/결제 데이터 즉각 일관성 보장 |
| 3. 신뢰성 | Outbox At-least-once 보장, Circuit Breaker 로 장애 격리 |
| 4. 관찰가능성 | 기둥별 메트릭 및 알림 임계값 정의 |
| 5. 성능 | Cache 로 읽기 부하 감소, Optimistic UI 로 인지 지연 0 |
| 6. 운영성 | Outbox DLQ 알림으로 무결 실패 탐지 |
| 7. 확장성 | 기둥별 독립 스케일링 (캐시 크기, Relay Worker 수 등) |

## 재검토 트리거

- Outbox 이벤트 P99 처리 지연이 30초를 초과하는 경우 (polling worker → 메시지 브로커 도입 검토)
- Cache invalidation 버그로 인한 데이터 불일치 인시던트 발생 시 (TTL 전략 재검토)
- Optimistic conflict rate 가 5% 를 초과하는 경우 (낙관적 잠금 전략 재검토)
- 결제·정산 흐름이 도입되어 PG / 정산 시스템이 추가될 경우 (Write-Through 대상 확장 + Saga 패턴 도입 검토)
- V-World/data.go.kr ETL 빈도가 일 1회를 초과해 캐시 TTL 5분이 너무 길어지는 경우

## 참고

- [ADR 0030](./0030-three-service-architecture.md) — γ' Three-Service Architecture 채택
- [ADR 0031](./0031-platform-core-bounded-contexts.md) — Platform-Core Bounded Context 분리
- [ADR 0033](./0033-seven-guardrails-enforcement.md) — 7 Guardrails 자동 강제 방법
- Martin Fowler, "Patterns of Enterprise Application Architecture" — Optimistic Locking
- Chris Richardson, "Microservices Patterns" — Transactional Outbox, Saga
