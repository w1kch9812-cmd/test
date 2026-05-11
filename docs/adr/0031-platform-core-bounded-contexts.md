# ADR 0031 - Platform-Core 내부 Bounded Context 분리

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0030](./0030-three-service-architecture.md) |

## 결정

platform-core 서비스의 내부를 두 개의 명시적 Bounded Context 로 분리한다.

| Bounded Context | 책임 | Ubiquitous Language |
|---|---|---|
| **Catalog Context** | 상품, 서비스, 카테고리, 가격 정책 관리 | Product, ServiceItem, Category, PriceTier |
| **Workforce/Authz Context** | 사용자, 역할, 권한, 조직 계층 관리 | User, Role, Permission, OrgUnit, Member |

두 컨텍스트는 **Anti-Corruption Layer(ACL)** 를 통해서만 상호 참조하며,
직접적인 코드 의존(import) 또는 DB 조인은 금지한다.

## 컨텍스트

### 1. 분리 필요성

ADR 0030 에서 platform-core 를 공유 허브로 채택했지만, 경계 없이 단일 모듈로 구현하면
시간이 지날수록 Catalog 와 Authz 로직이 얽혀 **Big Ball of Mud** 패턴으로 퇴화할 위험이 있다.

두 컨텍스트는 다음과 같이 근본적으로 다른 변경 속도와 팀 소유권을 가진다.

| 관점 | Catalog Context | Workforce/Authz Context |
|---|---|---|
| 변경 빈도 | 상품 기획 변경 시 (월 1-4회) | 권한 정책 변경 시 (분기 1-2회) |
| 위험도 | 비즈니스 영향 (매출) | 보안 영향 (접근 제어) |
| 외부 의존 | 가격 API, 재고 시스템 | SSO, 2FA, HR 시스템 |

### 2. Context Map

```
[Catalog Context] ----ACL----> [Workforce/Authz Context]
       |                                  |
   owns: products                  owns: users
         services                        roles
         categories                      permissions
         pricing                         org hierarchy
         |                                  |
   integrates with:                  integrates with:
     gongzzang3 (Consumer)             gongzzang3 (Consumer)
     seanal-sms (Consumer)             seanal-sms (Consumer)
```

관계 유형: **Customer-Supplier** (Catalog 가 Supplier, gongzzang3/seanal-sms 가 Customer)
통합 패턴: **Published Language** (OpenAPI + Zod 스키마 게시)

### 3. 기존 문제

현재 platform-core 는 단일 모듈로 구현되어 있어 Catalog 엔티티가 User 엔티티를
직접 참조하는 코드가 혼재되어 있다. 이는 다음 문제를 야기한다.

- Catalog 배포 시 Authz 코드도 재배포 필요 → 배포 독립성 저해
- Authz 보안 패치를 빠르게 반영하기 어려움 (Catalog 리그레션 테스트 병행 필요)
- 단위 테스트 작성 시 두 컨텍스트를 모두 목(mock)해야 하는 불편

## 검토한 옵션

### 옵션 A — 단일 모듈 유지

**채택 불가**. 앞서 언급한 퇴화 위험과 배포 독립성 문제를 해결하지 못한다.
단기 편의를 위해 장기 유지보수 비용을 누적시키는 전형적인 기술 부채다.

### 옵션 B — 물리적 서비스 분리 (platform-catalog + platform-authz)

**과도한 분리**. 두 컨텍스트는 운영상 항상 같이 배포되는 경우가 많고(Catalog 의 가격
정책에 역할 기반 접근 제어가 필요), 별개 서비스로 분리하면 네트워크 홉이 추가된다.
팀 규모 대비 운영 오버헤드가 크다.

### 옵션 C — 논리적 Bounded Context 분리 (채택)

같은 platform-core 서비스 안에서 **모듈 경계**(디렉터리 구조 + ESLint import 규칙)를
통해 컨텍스트를 분리한다. 배포 단위는 하나이지만 코드 경계는 엄격히 유지한다.

## 채택 (C) — 구현 세부

### 디렉터리 구조

```
packages/core/src/
  catalog/
    domain/
      entities/     (Product, ServiceItem, Category)
      value-objects/ (PriceTier, ProductCode)
      repositories/ (ProductRepository interface)
      errors.ts
    application/
      use-cases/    (CreateProduct, UpdatePricing, ...)
      ports/        (CatalogEventPublisher interface)
    infrastructure/
      repositories/ (DrizzleProductRepository)
      adapters/     (WorkforceAclAdapter)    <- ACL 진입점
  workforce/
    domain/
      entities/     (User, Role, Permission, OrgUnit)
      value-objects/ (Email, RoleCode)
      repositories/ (UserRepository interface)
      errors.ts
    application/
      use-cases/    (AssignRole, CreateOrgUnit, ...)
      ports/        (AuthzEventPublisher interface)
    infrastructure/
      repositories/ (DrizzleUserRepository)
      adapters/     (CatalogAclAdapter)      <- ACL 진입점
  shared/
    events/         (공유 이벤트 타입 — 컨텍스트 간 공통 계약)
    kernel/         (공통 Value Object: Money, Timestamp, ID)
```

### Anti-Corruption Layer 규칙

ACL 은 다음 두 가지만 허용한다.

1. **단방향 조회**: Catalog 가 특정 상품에 접근 가능한 역할 목록을 조회할 때,
   `WorkforceAclAdapter.getRolesForProduct(productId)` 를 호출한다.
   이 어댑터는 Workforce 도메인 객체를 **Catalog 전용 DTO** 로 변환한다.

2. **이벤트 구독**: `shared/events/` 의 타입으로만 이벤트를 발행/수신한다.
   컨텍스트 내부 도메인 이벤트를 직접 노출하지 않는다.

```typescript
// packages/core/src/catalog/infrastructure/adapters/workforce-acl.adapter.ts
// Workforce 도메인 객체를 Catalog 가 이해하는 DTO 로 변환 — 역방향 의존 차단
export class WorkforceAclAdapter {
  constructor(private readonly userRepo: UserRepository) {}

  async getRolesForProduct(productId: ProductId): Promise<CatalogRoleDto[]> {
    const roles = await this.userRepo.findRolesWithAccess(productId.value);
    return roles.map(toCatalogRoleDto);  // Workforce 엔티티 직접 노출 금지
  }
}
```

### ESLint Import 강제

```jsonc
// eslint.config.mjs (platform-core 전용)
{
  "rules": {
    "no-restricted-imports": ["error", {
      "patterns": [
        // Catalog 코드에서 Workforce 도메인 직접 import 금지
        { "group": ["*/workforce/domain/*"], "importNames": ["*"],
          "message": "Use WorkforceAclAdapter instead of importing Workforce domain directly." },
        // Workforce 코드에서 Catalog 도메인 직접 import 금지
        { "group": ["*/catalog/domain/*"], "importNames": ["*"],
          "message": "Use CatalogAclAdapter instead of importing Catalog domain directly." }
      ]
    }]
  }
}
```

### Ubiquitous Language 사전

**Catalog Context**

| 용어 | 정의 |
|---|---|
| Product | 판매 가능한 실물/디지털 상품 단위 |
| ServiceItem | 구독형 또는 일회성 서비스 단위 |
| Category | 상품/서비스의 분류 트리 노드 |
| PriceTier | 역할/수량 기반 가격 구간 |
| ProductCode | 외부 시스템과의 식별자 (SKU, EAN 등) |

**Workforce/Authz Context**

| 용어 | 정의 |
|---|---|
| User | 인증된 개인 계정 |
| Role | 권한 집합에 붙인 이름 (MASTER_ADMIN, TEAM_MEMBER 등) |
| Permission | 단일 작업 수행 허가 (site:create, post:publish 등) |
| OrgUnit | 조직 계층의 노드 (팀, 부서, 법인) |
| Member | 특정 OrgUnit 에 속한 User-Role 연결 |

## 영향

### 긍정적 영향

- Catalog 와 Authz 독립 테스트 가능: 각 컨텍스트는 상대방을 ACL 인터페이스로만 mock 처리
- 보안 정책 변경(Authz)이 Catalog 리그레션 테스트를 트리거하지 않음
- 새 개발자 온보딩 시 컨텍스트 범위가 명확해 학습 곡선 감소

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| ACL 보일러플레이트 증가 | ACL 생성 스캐폴딩 스크립트(pnpm gen:acl) 제공 |
| 컨텍스트 간 이벤트 스키마 버전 불일치 | ADR 0033 Guardrail #4 (schema registry) 로 강제 |
| 개발자의 경계 무시 유혹 | ESLint import 규칙 + CI 통과 조건으로 강제 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 도메인 언어가 컨텍스트별로 일관, 용어 혼용 방지 |
| 2. 보안 | Authz Context 가 격리되어 보안 취약점 영향 범위 제한 |
| 3. 신뢰성 | 컨텍스트 독립 테스트로 회귀 탐지 속도 향상 |
| 4. 관찰가능성 | 컨텍스트별 독립 메트릭 레이블 부착 가능 |
| 5. 성능 | 각 컨텍스트의 캐시 전략 독립 조정 가능 |
| 6. 운영성 | 컨텍스트별 CODEOWNERS 지정으로 리뷰 책임 명확화 |
| 7. 확장성 | 필요 시 컨텍스트를 독립 서비스로 분리하는 이주 경로 확보 |

## 재검토 트리거

- Catalog Context 와 Workforce/Authz Context 간 ACL 호출이 월 500만 건을 초과하는 경우
  (물리적 서비스 분리 검토)
- 두 컨텍스트가 동일한 DB 테이블을 공유해야 하는 신규 요건이 발생하는 경우
- 보안 감사에서 ACL 경계 우회 취약점이 발견되는 경우

## 참고

- [ADR 0030](./0030-three-service-architecture.md) — γ' Three-Service Architecture 채택
- [ADR 0032](./0032-eventual-consistency-strategy.md) — Eventual Consistency 전략
- [ADR 0033](./0033-seven-guardrails-enforcement.md) — 7 Guardrails 자동 강제 방법
- Eric Evans, "Domain-Driven Design" — Bounded Context 패턴
- Vaughn Vernon, "Implementing Domain-Driven Design" — Context Map 패턴
