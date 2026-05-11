# ADR 0030 - γ' Three-Service Architecture 채택

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0029](./0029-explicit-environment-separation.md) |

## 결정

**gongzzang3 + seanal-sms + platform-core** 세 서비스로 이루어진 **γ' Three-Service Architecture**를 채택한다.

| 서비스 | 도메인 | 사용자 | 소유 데이터 |
|---|---|---|---|
| **platform-core** | (내부 허브, 외부 노출 없음) | gongzzang3 / seanal-sms (서비스 consumer) | Catalog Context (IndustrialComplex, Parcel, Building, Manufacturer) · Workforce/Authz Context (Staff, Role, Session) · 공통 Outbox 버스 |
| **gongzzang3** | **gongzzang.com (B2C 부동산 플랫폼)** | 일반 소비자 (Zitadel consumer org) | Listing, ListingPhoto, RealTransaction, CourtAuction, BookmarkListing, SearchHistory, AnalysisReport, V-World/data.go.kr ETL 파이프라인 |
| **seanal-sms** | **saenal.com (B2B 산단 관리 어드민)** | 산단 관리 직원 (Zitadel staff org via platform-core) | InteractiveBlueprint, Polygon, IndustryGroup, ParcelIndustryGroup, Site, SitePage, SiteDeployment, SiteCampaign, KakaoNotificationLog |

서비스 간 통신은 **typed API contract (Rust 측 `crates/api-types`, TS 측 Zod 스키마)** 또는
**도메인 이벤트 (Transactional Outbox)** 로만 허용한다.
직접 DB 크로스-접근 및 공유 mutable state 는 명시적으로 금지된다 (ADR 0033 참조).

**Cross-domain SSO**: 두 제품 서비스의 어드민 페이지는 별개 도메인(`gongzzang.com` / `saenal.com`)
에 호스팅되지만, platform-core Workforce/Authz Context 를 통해 **Zitadel staff organization
기반 단일 로그인**을 공유한다. 한 도메인에 로그인한 직원은 다른 도메인 어드민에서도
별도 로그인 없이 (Zitadel session 유효 범위 내) 인증된다.

## 컨텍스트

### 1. 문제 배경

gongzzang3 와 seanal-sms 는 독립적으로 발전해 왔으나, 2026-04 기준 다음 중복 문제가 누적됐다.

- **신원/권한 로직 이중화**: gongzzang3 는 Zitadel JWT 검증을 자체 보유(`crates/auth`),
  seanal-sms 는 Better Auth + 자체 `team` / `permission` / `rolePermission` 테이블 보유.
  동일 직원이 두 시스템에서 서로 다른 역할을 가질 수 있어 권한 일관성이 깨진다.
- **산단 마스터 데이터 이중화**: gongzzang3 의 `crates/domain/core/industrial-complex` 와
  `crates/domain/core/parcel` 가 V-World/data.go.kr 에서 ETL 한 사실 데이터를 소유하는데,
  seanal-sms 는 별도 `industrialComplex` / `parcelInfo` 테이블에 부분 사본을 유지한다.
  동기화 누락이 발생하면 saenal.com 의 도면 에디터가 outdated 정보를 표시한다.
- **운영 복잡도**: 두 서비스의 배포 파이프라인이 완전히 별개여서 공통 정책(SHA pin, secret
  namespace, ADR 0029) 적용 시 중복 작업 발생.

### 2. 아키텍처 옵션 검토

아래 네 가지 후보를 SSS 7기둥 기준으로 평가했다.

| 옵션 | 설명 | 결함 |
|---|---|---|
| A. 모놀리스 통합 | gongzzang3 + seanal-sms 를 단일 코드베이스로 병합 | 배포 독립성 0, 팀 자율성 소멸 |
| B. Two-Service | 두 서비스 유지, 공유 코드를 npm 패키지로 추출 | 버전 드리프트, 패키지 게시 오버헤드 |
| **C. γ' Three-Service (채택)** | platform-core 를 독립 서비스로 분리, 두 제품 서비스가 의존 | 초기 설계 비용 |
| D. 풀 마이크로서비스 | 도메인당 1 서비스 (10+ 서비스) | 운영 복잡도 폭발, 팀 규모 부적합 |

### 3. γ' Three-Service 선택 근거

- **Catalog Context** (IndustrialComplex, Parcel, Building, Manufacturer) 와
  **Workforce/Authz Context** (Staff identity) 는 두 제품 서비스 모두에서 핵심적으로
  사용되는 공유 도메인이다. 산단 마스터 데이터는 V-World/data.go.kr ETL 한 곳에서만
  유지되어야 하고, 직원 신원은 두 도메인에서 동일하게 인식되어야 한다.
- platform-core 를 독립 서비스로 운영하면 **배포 독립성**과 **공유 도메인 로직의 단일 소스(SSOT)**
  를 동시에 달성한다. ETL 파이프라인 변경이 gongzzang3 와 seanal-sms 양쪽에 즉시 반영된다.
- 팀 규모(소규모 ~ 중간) 에서 풀 마이크로서비스보다 운영 부담이 현저히 낮다.

## 검토한 옵션

### 옵션 A — 모놀리스 통합

**채택 불가**. gongzzang3 의 Rust ETL 파이프라인과 seanal-sms 의 Next.js 스택은
언어/런타임 자체가 달라 단일 코드베이스로 병합하면 빌드 복잡도가 기하급수적으로 증가한다.
독립 배포 요건(ADR 0029 에서 확립된 environment 분리 정책)과도 충돌한다.

### 옵션 B — Two-Service + npm 패키지

**부분 채택 불가**. npm 패키지 추출은 단기적으로 유효하지만, platform-core 가 독자적인
**런타임 상태**(이벤트 버스, 캐시 워밍)를 필요로 하게 될 때 패키지 경계를 깰 수밖에 없다.
이 시점에서 서비스 분리를 다시 해야 하므로 선행 투자 대비 효용이 낮다.

### 옵션 C — γ' Three-Service (채택)

platform-core 는 다음 두 Bounded Context 를 소유한다 (상세는 ADR 0031 참조).

gongzzang3 와 seanal-sms 는 platform-core API 를 통해서만 이 컨텍스트에 접근한다.
서비스 간 이벤트는 Transactional Outbox 패턴으로 전달된다 (ADR 0032 참조).

### 옵션 D — 풀 마이크로서비스

**채택 불가**. 현재 팀 구조에서 10개 이상의 서비스를 운영하면 서비스 메시, 분산 트레이싱,
독립 on-call 로테이션이 필요해 운영 비용이 제품 개발 비용을 초과한다.

## 채택 (C) — 구현 세부

### 서비스 경계 다이어그램

```
                          ┌────────────────────────────────┐
                          │        platform-core           │
                          │  ┌─────────────────────────┐   │
                          │  │ Catalog Context         │   │
                          │  │  IndustrialComplex      │   │
                          │  │  Parcel, Building       │   │
                          │  │  Manufacturer + ETL     │   │
                          │  └────────────┬────────────┘   │
                          │               │ Outbox v1      │
                          │  ┌────────────┴────────────┐   │
                          │  │ Workforce/Authz Context │   │
                          │  │  Staff, Role, Session   │   │
                          │  │  (Zitadel federation)   │   │
                          │  └─────────────────────────┘   │
                          └────────▲──────────────▲────────┘
                                   │ REST + OIDC  │ Outbox events
                ┌──────────────────┴────┐   ┌─────┴────────────────────┐
                │      gongzzang3       │   │       seanal-sms         │
                │  gongzzang.com (B2C)  │   │  saenal.com (B2B admin)  │
                │                       │   │                          │
                │ Listing, Bookmark,    │   │ InteractiveBlueprint,    │
                │ RealTransaction,      │   │ IndustryGroup,           │
                │ SearchHistory,        │   │ ParcelIndustryGroup,     │
                │ Zitadel consumer org  │   │ Site, SitePage,          │
                │ + admin (staff org)   │   │ SiteDeployment           │
                └───────────────────────┘   └──────────────────────────┘
```

### API 통신 규칙

1. **동기 요청**: REST + OpenAPI (`crates/api-types` Rust 측 + Zod 스키마 TS 측 — 둘이
   동일 published language 를 다른 언어로 표현).
   서비스가 직접 다른 서비스의 DB 를 쿼리하는 것은 금지 (ADR 0033 G1).
2. **비동기 이벤트**: platform-core 의 Catalog/Workforce Context 가 Transactional Outbox
   에 이벤트 기록 → polling worker 가 broadcast → consumer 가 idempotent 처리.
   이벤트 스키마는 `schemaVersion` 필드 필수 (ADR 0033 G4).
3. **인증 위임**: gongzzang3 admin / seanal-sms 는 Zitadel ID Token 검증을 platform-core
   Workforce 에 위임 — `POST /workforce/sessions/verify` 가 JTI denylist 와 role claim 을
   확인하고 platform-core StaffSession 식별자를 반환. 로컬 role cache TTL = 60s.
   gongzzang3 의 B2C 일반 사용자는 별도 Zitadel consumer organization 으로 격리되고
   gongzzang3 가 직접 검증 (Workforce Context 와 무관).
4. **Cross-domain SSO**: Zitadel staff organization 의 session cookie 가
   `gongzzang.com` 과 `saenal.com` 양쪽 어드민의 OIDC redirect 후 발급되어 한 번 로그인이
   다른 도메인 어드민으로 자동 이어진다. 2FA 는 Zitadel 측 정책에 위임.

### 배포 독립성 보장

각 서비스는 독자적인 CI/CD 파이프라인, 독자적인 DB 인스턴스, 독자적인 secret namespace 를 가진다.
공유 인프라(이벤트 브로커 등)는 platform-core 가 소유하고, 다른 두 서비스는 consumer 로만 참여한다.

## 영향

### 긍정적 영향

| 영역 | 효과 |
|---|---|
| 배포 독립성 | 각 서비스가 독립적으로 릴리스 가능, 상호 블로킹 제거 |
| 도메인 명확성 | Bounded Context 경계가 코드베이스 구조와 일치 |
| 재사용성 | Catalog/Authz 로직이 단일 소스로 관리 |
| 테스트 용이성 | 서비스 경계가 명확해 인테그레이션 테스트 범위 축소 가능 |
| 보안 | secret namespace 분리 (ADR 0029), DB 크로스-접근 금지 (ADR 0033) |

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| 네트워크 지연 (동기 API 호출) | 중요 경로는 Write-Through + 로컬 캐시로 완화 (ADR 0032) |
| Eventual Consistency 복잡도 | Outbox 패턴 + Optimistic UI 로 UX 영향 최소화 (ADR 0032) |
| 초기 설계 비용 | ADR 0031-0033 으로 구현 지침 명문화 |
| platform-core SPOF 위험 | 가용성 목표 99.9% + circuit breaker 패턴 적용 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 서비스 경계가 명확해 각 서비스의 복잡도 독립 관리 가능 |
| 2. 보안 | DB 크로스-접근 금지, secret namespace 분리 유지 |
| 3. 신뢰성 | 서비스 독립 배포로 장애 격리 |
| 4. 관찰가능성 | 서비스별 독립 로깅/트레이싱 파이프라인 |
| 5. 성능 | 공유 캐시 레이어를 platform-core 가 소유해 일관된 TTL 정책 |
| 6. 운영성 | 배포 파이프라인 독립, 팀별 on-call 명확 |
| 7. 확장성 | 각 서비스 독립 스케일링, 공유 도메인만 수직 스케일 |

## 재검토 트리거

- platform-core 의 가용성이 3개월 내 99.9% 미만으로 하락하는 경우
- 팀 규모가 3배 이상 성장하고 platform-core 의 Bounded Context 가 5개 이상으로 분화되는 경우
- 서비스 간 동기 API 호출 P99 레이턴시가 지속적으로 500ms 초과하는 경우

## 참고

- [ADR 0029](./0029-explicit-environment-separation.md) — environment 분리 정책
- [ADR 0031](./0031-platform-core-bounded-contexts.md) — platform-core 내부 Bounded Context 분리
- [ADR 0032](./0032-eventual-consistency-strategy.md) — Eventual Consistency 전략
- [ADR 0033](./0033-seven-guardrails-enforcement.md) — 7 Guardrails 자동 강제 방법
