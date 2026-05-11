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

또한 G5의 "동기 호출 금지" 는 다음 ADR 본문에서 **"중요 쓰기 경로의 외부 동기 호출 최소화"**
로 sharpen 되었음 (Write-Through 패턴 자체는 ADR 0032 에서 허용된 동기 경로). 표는 약어,
본문이 권위.

## 컨텍스트

### 1. 아키텍처 부채 누적 방지

ADR 0030-0032 에서 정의한 아키텍처 원칙은 코드 리뷰 시 개발자가 인지하고 따를 것을
기대한다. 그러나 다음 이유로 자동화 강제가 필수적이다.

- **온보딩 시 지식 격차**: 새 개발자가 모든 ADR 을 숙지하기 전에 원칙 위반 코드를 작성할 수 있다.
- **시간 압박**: 마감 압박 하에서 원칙을 우회하는 코드가 머지될 수 있다.
- **리뷰어 피로**: 아키텍처 원칙 준수 여부를 매번 리뷰에서 확인하면 리뷰어 부담이 증가한다.

### 2. 강제 방법 선택 기준

| 기준 | 설명 |
|---|---|
| 즉시 피드백 | IDE 또는 pre-commit 시점에 피드백 |
| CI 통합 | 모든 PR 에서 자동 실행, 실패 시 머지 차단 |
| 탈출구 | 예외가 정당한 경우 명시적 예외 선언 과정 필요 |
| 낮은 운영 비용 | ESLint, TypeScript, GitHub Actions 등 기존 도구 우선 |

### 3. 기존 위반 현황 (마이그레이션 시 해결 대상)

- **G1 후보**: seanal-sms 의 `industrial-group.repository.ts` 가 Drizzle `db.execute<T>()`
  로 raw SQL 결과를 camelCase 타입으로 캐스팅했으나 실제 반환은 snake_case 였음 — 컴파일
  성공·런타임 `undefined` 누락(2026-04-09 인시던트). 향후 platform-core 가 Catalog 데이터를
  소유하면 seanal-sms 가 직접 SQL 을 쓰지 않고 typed API 만 호출하는 패턴으로 강제 (G1 + G2 결합).
- **G4 후보**: 현재 seanal-sms `event_outbox` 와 gongzzang3 outbox-event 가 별개 스키마를
  사용하고 `schemaVersion` 필드가 명시적이지 않다. platform-core 진입 시 공통
  `event-schemas/` 로 통합하며 schema registry 검증 도입.
- **G6 후보**: seanal-sms 의 `parcelInfo` 테이블이 gongzzang3 의 `parcel` 마스터 데이터를
  부분 사본 형태로 보유 — 외부 데이터 소스(V-World) 접근이 Repository Port 뒤로 격리되어
  있지 않다. platform-core Catalog 가 단일 ETL adapter 를 소유하도록 통합.
- **G7 후보**: 어드민 메뉴 권한이 두 서비스에서 각각 정의되어 동일 직원이 두 시스템에서
  서로 다른 역할을 가질 수 있음. Workforce Context 가 단일 소스가 되면 자연히 해결.

## 7 Guardrails 상세 구현

### Guardrail 1 — 서비스 간 직접 DB 접근 금지

**위반 예시 (실제 사례)**

```typescript
// ❌ 위반: seanal-sms 가 platform-core 가 소유할 Catalog 테이블을 직접 조회
// industry-group.repository.ts
const rows = await db.execute<{ parentId: string | null; name: string }>(
  sql`SELECT parent_id, name FROM industrial_complex WHERE id = ${id}`,
);
// 1) 다른 서비스의 테이블을 직접 SELECT (G1 위반)
// 2) 반환 타입이 snake_case 인데 camelCase 로 캐스팅 (G2 위반 — typed contract 부재)
// → 2026-04-09 인시던트의 근본 원인 클래스
```

```typescript
// ✅ 권장: platform-core API 만 호출, 타입은 published Zod 스키마에서
import { CatalogClient } from "@platform-core/client";
const complex = await catalogClient.getIndustrialComplex({ id });
// 응답은 Zod schema 로 검증된 typed object
```

**강제 방법 — ESLint 커스텀 규칙** (`packages/eslint-config/rules/no-cross-service-sql.js`)

- `db.execute<T>()` 패턴 자체를 금지하거나, 허용 시 같은 파일에 Zod `.parse()` 검증 동반 강제
- 다른 서비스 소유 테이블 이름 (`industrial_complex`, `parcel`, `staff` 등) 을
  raw SQL literal 에서 발견하면 error
- Drizzle 의 typed query builder 만 허용, raw SQL escape 는 명시적 예외 어노테이션 필요

**CI 단계**: `pnpm turbo lint` (PR 머지 조건)

### Guardrail 2 — 서비스 간 호출은 타입된 API 계약으로만

서비스 간 모든 요청/응답은 다음 두 곳의 동기화된 contract 로 정의한다.

- **Rust 측**: `crates/api-types` (gongzzang3 와 platform-core 가 공유)
- **TS 측**: `packages/api-contracts/` Zod 스키마 (seanal-sms 가 소비)

OpenAPI 단일 SSOT 에서 양쪽 코드를 생성한다. `fetch` / `ky` / `reqwest` 응답을 `unknown`
이외 타입으로 받는 것을 금지하고, `.parse()` (TS) 또는 `serde_json::from_str` (Rust) 검증
없이 사용하면 컴파일 오류 또는 lint error.

**위반 예시**

```typescript
// ❌ 위반: 응답을 임의 타입으로 캐스팅
const data = (await res.json()) as { staffId: string; role: string };

// ✅ 권장: Zod published language 통과
const data = StaffSessionResponseSchema.parse(await res.json());
```

**CI 단계**: `pnpm turbo typecheck` + `cargo check --workspace` (둘 다 PR 머지 조건)

### Guardrail 3 — 공유 mutable state 금지

복수 서비스가 같은 mutable 리소스(예: 공유 파일시스템 경로, 공유 Redis key) 를 직접
읽고 쓰는 패턴을 금지한다. 모든 공유 상태는 platform-core 가 owner 가 되어 API 또는
이벤트 형태로 노출한다.

**ESLint immutability 규칙 + Rust 측 `#[deny(...)]`**

- TS: `eslint-plugin-functional` 의 `prefer-readonly-type`, `no-let` (도메인 모델 한정)
- Rust: 도메인 entity 는 `&mut self` 메서드를 노출하지 않고 새 인스턴스를 반환

예외가 필요한 경우 `eslint-disable-next-line` (TS) / `#[allow(...)]` (Rust) 로 명시적
예외 선언 후 PR 리뷰 필수.

**CI 단계**: `pnpm turbo lint` + `cargo clippy -- -D warnings` (PR 머지 조건)

### Guardrail 4 — 이벤트 스키마 버전 관리 필수

모든 도메인 이벤트는 `shared-kernel/events/` (Rust) 와 `packages/event-schemas/` (TS) 양쪽에
`schemaVersion` 리터럴 필드를 포함해야 한다.

```rust
// Rust 측 — Catalog 이벤트
#[derive(Serialize, Deserialize)]
pub struct IndustrialComplexCreated {
    pub schema_version: u32,   // 리터럴 1 — 변경 시 v2 신규 파일
    pub complex_id: ComplexId,
    pub pnu_prefix: String,
    pub created_at: DateTime<Utc>,
}
```

```typescript
// TS 측 — 동일 이벤트 (생성된 Zod)
export const IndustrialComplexCreatedV1 = z.object({
  schemaVersion: z.literal(1),
  complexId: z.string().uuid(),
  pnuPrefix: z.string().length(10),
  createdAt: z.string().datetime(),
});
```

CI 검증 스크립트 (`scripts/validate-event-schemas.mjs` + Rust 측 `cargo-public-api`):

- 모든 이벤트 파일에 `schemaVersion` 필드 존재 여부
- 기존 버전 파일 수정 금지 (파일 해시 비교 + `cargo-public-api` diff)
- Breaking change 는 `.v2.ts` / `_v2.rs` 신규 파일 생성, consumer 2주간 이전 버전 지원

**CI 단계**: 별도 `schema-registry-check` job (PR 머지 조건)

### Guardrail 5 — 중요 쓰기 경로에서 외부 동기 호출 최소화

**중요 쓰기 경로 정의**: 데이터 일관성에 결정적인 쓰기 (산단 등록 commit, Staff 역할
부여, Site 발행 등) 는 트랜잭션 내부에서 외부 동기 HTTP 호출을 하지 않는다. 외부 부수
효과는 Transactional Outbox 로 미뤄 트랜잭션 커밋 후 비동기 처리한다.

Write-Through 패턴 (ADR 0032 기둥 3) 의 명시적 동기 의존은 `@allowSyncCall` 어노테이션
(TS) / `#[allow_sync_call]` macro (Rust) 으로 화이트리스트 처리하고 PR 리뷰에서 사유 확인.

```rust
// ❌ 위반: 산단 등록 트랜잭션 안에서 외부 HTTP 호출
async fn create_complex(&self, tx: &mut Transaction, input: CreateComplexInput) -> Result<...> {
    let complex = self.repo.insert(tx, &input).await?;
    self.email_client.send_admin_notification(&complex).await?;  // ← G5 위반
    Ok(complex)
}

// ✅ 권장: outbox 이벤트로 미루기
async fn create_complex(&self, tx: &mut Transaction, input: CreateComplexInput) -> Result<...> {
    let complex = self.repo.insert(tx, &input).await?;
    self.outbox.publish(tx, IndustrialComplexCreated::from(&complex)).await?;
    Ok(complex)
    // 이메일 알림은 별도 consumer 가 IndustrialComplexCreated 이벤트를 받아 처리
}
```

**CI 단계**: architecture fitness 테스트 — `pnpm turbo test --filter=@seanal/core -- --testPathPattern=arch` 및 `cargo test --workspace --features arch-fitness`

### Guardrail 6 — 외부 사이드 이펙트는 Port/Adapter 뒤로

외부 서비스 목록: V-World OpenAPI, data.go.kr OpenAPI, Zitadel OIDC, AWS SES (이메일),
카카오 알림톡, AWS S3 (도면·이미지 스토리지). 모두 도메인 계층은 trait/interface 만
참조하고, 구체 HTTP 클라이언트는 infrastructure 어댑터로 격리한다.

```rust
// catalog-domain/src/ports.rs
pub trait VWorldClient: Send + Sync {
    async fn fetch_complex_by_pnu(&self, pnu: &Pnu) -> Result<RawComplexDto, VWorldError>;
}

// catalog-infra/src/vworld_adapter.rs
pub struct HttpVWorldClient { http: reqwest::Client, base_url: Url }
impl VWorldClient for HttpVWorldClient { ... }
```

**위반 예시**

```typescript
// ❌ 위반: use case 가 fetch 를 직접 호출
export class CreateComplexUseCase {
  async execute(input) {
    const res = await fetch(`https://api.vworld.kr/...`);   // ← G6 위반
    ...
  }
}

// ✅ 권장: port 주입
export class CreateComplexUseCase {
  constructor(private readonly vworld: VWorldPort) {}
  async execute(input) {
    const raw = await this.vworld.fetchByPnu(input.pnu);
    ...
  }
}
```

**CI 단계**: architecture fitness 테스트 — `import` 경로 정적 분석 (`dependency-cruiser` TS,
`cargo-deny` Rust) 으로 도메인 계층의 외부 네트워크 의존성 차단.

### Guardrail 7 — Bounded Context 소유권 CODEOWNERS 강제

`CODEOWNERS` 파일에서 컨텍스트별 소유자 팀을 명시한다.

```
# platform-core CODEOWNERS
/crates/catalog/         @team-catalog
/crates/workforce/       @team-platform-auth
/crates/shared-kernel/   @team-architecture

# gongzzang3 CODEOWNERS
/crates/domain/core/listing/    @team-marketplace
/crates/data-pipeline-control/  @team-data
/apps/web/                       @team-frontend

# seanal-sms CODEOWNERS
/seanal-sms/apps/web/src/components/blueprint/  @team-blueprint
/seanal-sms/packages/core/                       @team-backend
```

브랜치 보호 규칙:
- `require_code_owner_reviews`: true
- `required_approving_review_count`: 1 (보안 영향 PR 은 2)
- Required Status Checks: `lint`, `typecheck`, `test`, `schema-registry-check`,
  `cargo clippy`, `arch-fitness`

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

PR 머지 차단 조건은 다음 job 매트릭스로 강제한다.

| Guardrail | Job | 도구 | 차단 조건 |
|---|---|---|---|
| G1 | `lint` | ESLint + custom rule `no-cross-service-sql` | error 발생 |
| G2 | `typecheck` | `tsc --noEmit` + `cargo check` | unknown 캐스팅 또는 .parse 누락 |
| G3 | `lint` | `eslint-plugin-functional` + `cargo clippy -D warnings` | mutable shared state |
| G4 | `schema-registry-check` | `validate-event-schemas.mjs` + `cargo-public-api` | schemaVersion 누락 / 기존 버전 hash drift |
| G5 | `arch-fitness` | Vitest arch tests + Rust integration tests | 트랜잭션 내 외부 sync 호출 |
| G6 | `arch-fitness` | `dependency-cruiser` + `cargo-deny` | 도메인 계층의 외부 클라이언트 직접 의존 |
| G7 | GitHub branch protection | CODEOWNERS + required reviewers | 컨텍스트 소유 팀 승인 누락 |

각 job 은 `pnpm turbo run <task>` 와 `cargo` workspace 명령으로 병렬 실행되며 캐시 hit
시 평균 PR 검증 시간은 4분 이내로 유지한다.

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
