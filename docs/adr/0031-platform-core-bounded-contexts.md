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
| **Catalog Context** | 산업단지 마스터 데이터와 그 하위 리소스(산단·필지·건물·제조사·도면·공간 레이어·3D·유치업종) 의 소유 및 외부 데이터 소스(V-World, data.go.kr) ETL | IndustrialComplex, Parcel, Building, Manufacturer, ParcelKind, Blueprint, SpatialLayer, DigitalTwin3d, IndustryGroup, ParcelIndustryAssignment, PNU |
| **Workforce/Authz Context** | 내부 직원(Staff) 의 신원·역할·권한·세션 관리. Zitadel 페더레이션 진입점 | Staff, StaffRole, StaffPermission, StaffSession, ZitadelClaims |

두 컨텍스트는 **Anti-Corruption Layer(ACL)** 를 통해서만 상호 참조하며,
직접적인 코드 의존(import) 또는 DB 조인은 금지한다.

> **중요 — Workforce Context 의 범위**
> Workforce/Authz Context 는 **내부 직원** 만 관리한다. gongzzang.com 의 B2C 일반 사용자
> (`User`, `BookmarkListing`, `SearchHistory` 등) 는 gongzzang 이 직접 소유하며,
> Zitadel 의 별도 organization 으로 격리된다. 직원/소비자 신원을 동일 컨텍스트에 두면
> 권한 모델이 폭증하고 보안 경계가 흐려지므로 명시적으로 분리한다.

## 컨텍스트

### 1. 분리 필요성

ADR 0030 에서 platform-core 를 공유 허브로 채택했지만, 경계 없이 단일 모듈로 구현하면
시간이 지날수록 Catalog 와 Authz 로직이 얽혀 **Big Ball of Mud** 패턴으로 퇴화할 위험이 있다.

두 컨텍스트는 다음과 같이 근본적으로 다른 변경 속도와 팀 소유권을 가진다.

| 관점 | Catalog Context | Workforce/Authz Context |
|---|---|---|
| 변경 빈도 | V-World/data.go.kr 갱신 사이클 (주 1회 ~ 월 1회) | 권한 정책 변경 (분기 1회) |
| 위험도 | 데이터 정확성 (지도·검색 영향) | 보안 영향 (관리자 접근 제어) |
| 외부 의존 | V-World API, data.go.kr OpenAPI, PNU 코드 체계 | Zitadel OIDC, JWKS, JTI denylist |
| 변경 트리거 | 외부 데이터 소스 스키마 변경, 신규 산단 고시 | 신규 역할 추가, 조직 개편 |

### 2. Context Map

```
[Catalog Context] ----ACL----> [Workforce/Authz Context]
       |                                  |
   owns:                              owns:
     IndustrialComplex                  Staff
     Parcel                             StaffRole
     Building                           StaffPermission
     Manufacturer                       StaffSession
     ParcelKind, PNU                    (Zitadel federation)
       |                                  |
   integrates with:                    integrates with:
     gongzzang (Consumer)              gongzzang admin (Consumer)
     dawneer (Consumer)              dawneer (Consumer)
     V-World ETL                        Zitadel OIDC (Upstream)
     data.go.kr ETL
```

관계 유형: **Customer-Supplier** (Catalog/Workforce 가 Supplier, gongzzang·dawneer 가 Customer)
통합 패턴: **Published Language** (OpenAPI + Zod 스키마, `crates/api-types` 공유)

### 3. 기존 분리 문제 — 왜 ACL 이 필요한가

현재 코드베이스에는 두 컨텍스트가 섞일 위험이 이미 존재한다.

- gongzzang 은 `industrial-complex` / `parcel` 엔티티를 자체 소유하지만, dawneer 도
  `industrialComplex` / `parcelInfo` 테이블을 별도 보유해 **마스터 데이터 이중화** 가 발생.
- 어드민 메뉴 권한이 두 서비스에서 각각 정의되어 있어, 동일 직원이 두 시스템에서
  서로 다른 역할을 가질 수 있음 (G7 위반 잠재).

ACL 없이 단일 모듈로 합칠 경우 다음 문제가 누적된다.

- Catalog 배포 시 Workforce 코드도 재배포 필요 → 배포 독립성 저해
- 보안 패치(Workforce)가 Catalog 리그레션 테스트를 trigger
- 단위 테스트 작성 시 두 컨텍스트를 모두 목(mock)해야 하는 불편

## 검토한 옵션

### 옵션 A — 단일 모듈 유지

**채택 불가**. 앞서 언급한 퇴화 위험과 배포 독립성 문제를 해결하지 못한다.
단기 편의를 위해 장기 유지보수 비용을 누적시키는 전형적인 기술 부채다.

### 옵션 B — 물리적 서비스 분리 (platform-catalog + platform-workforce)

**현재는 과도한 분리**. 두 컨텍스트는 운영상 같이 배포되는 경우가 많고 (Workforce
권한이 Catalog 의 어드민 작업에 항상 동반), 별개 서비스로 분리하면 네트워크 홉이 추가된다.
팀 규모 대비 운영 오버헤드가 크다. 단, 미래에 트래픽 또는 팀 규모가 임계점을 넘으면
**옵션 B 로 이주 가능한 경로** 를 옵션 C 가 보존한다.

### 옵션 C — 논리적 Bounded Context 분리 (채택)

같은 platform-core 서비스 안에서 **모듈 경계** (Cargo workspace sub-crate + import 규칙) 를
통해 컨텍스트를 분리한다. 배포 단위는 하나이지만 코드 경계는 엄격히 유지한다.

## 채택 (C) — 구현 세부

### Workspace 구조 (Rust + Cargo)

platform-core 는 Rust 기반 (ADR 0001 기준 gongzzang 과 동일 스택) Cargo workspace 로 구현한다.

```
platform-core/
  crates/
    catalog/
      catalog-domain/         # Pure domain: IndustrialComplex, Parcel, Building, Manufacturer, Blueprint
        src/
          industrial_complex.rs
          parcel.rs            (PNU value object 포함)
          building.rs
          manufacturer.rs
          errors.rs
      catalog-app/            # Use cases + ports
        src/
          register_complex.rs
          update_parcel_kind.rs
          ports.rs             (CatalogRepository, EtlSource trait)
      catalog-infra/          # Drizzle/sqlx repositories + V-World/data.go.kr adapters
        src/
          sqlx_repository.rs
          vworld_etl.rs
          datagokr_etl.rs
          workforce_acl.rs    # ← Catalog → Workforce ACL 진입점

    workforce/
      workforce-domain/       # Pure domain: Staff, StaffRole, StaffPermission, StaffSession
        src/
          staff.rs
          role.rs              (RoleCode value object)
          permission.rs
          session.rs
          errors.rs
      workforce-app/
        src/
          assign_role.rs
          revoke_session.rs
          ports.rs             (StaffRepository, ZitadelClient trait)
      workforce-infra/
        src/
          sqlx_repository.rs
          zitadel_client.rs    (JWKS cache, JTI denylist)
          catalog_acl.rs       # ← Workforce → Catalog ACL 진입점

    shared-kernel/             # 컨텍스트 공통 ValueObject + 이벤트 contract
      src/
        ids.rs                 (ComplexId, ParcelId, StaffId — newtype)
        pnu.rs                 (PNU 검증 — gongzzang 의 shared-kernel 과 동일 규칙)
        events/
          catalog_v1.rs        (IndustrialComplexCreated, ParcelKindChanged, ...)
          workforce_v1.rs      (StaffRoleAssigned, StaffSessionRevoked, ...)
```

이 구조는 gongzzang 의 기존 `crates/domain/core/{industrial-complex,parcel,building,manufacturer}`
배치와 일관성을 유지해 마이그레이션 비용을 낮춘다.

### Anti-Corruption Layer 규칙

ACL 은 다음 두 가지 경로만 허용한다.

1. **단방향 조회 — Workforce → Catalog**
   Workforce 가 "이 Staff 가 특정 산단을 수정할 권한이 있는가?" 를 확인할 때,
   `CatalogAclAdapter` 가 Catalog 도메인 객체를 **Workforce 가 이해하는 DTO** 로 변환한다.

   ```rust
   // crates/workforce/workforce-infra/src/catalog_acl.rs
   // Catalog 도메인 객체를 Workforce 가 이해하는 DTO 로 변환 — 역방향 의존 차단
   pub struct CatalogAclAdapter {
       catalog_repo: Arc<dyn CatalogRepository>,
   }

   impl CatalogAclAdapter {
       /// Staff 가 접근 가능한 산단 ID 목록을 조회한다.
       /// Catalog 엔티티 자체는 노출하지 않고 ID + 최소 식별 정보만 반환.
       pub async fn list_complexes_accessible_by(
           &self,
           staff_id: StaffId,
           role: RoleCode,
       ) -> Result<Vec<ComplexAccessDto>, AclError> {
           let complexes = self.catalog_repo
               .find_by_role_scope(role)
               .await?;
           Ok(complexes.into_iter().map(ComplexAccessDto::from).collect())
       }
   }
   ```

2. **단방향 조회 — Catalog → Workforce**
   Catalog 어드민 작업(예: 신규 산단 등록) 의 actor 정보를 audit log 에 남길 때,
   `WorkforceAclAdapter` 가 Staff 의 식별·역할만 노출하는 DTO 를 반환한다.

   ```rust
   // crates/catalog/catalog-infra/src/workforce_acl.rs
   pub struct WorkforceAclAdapter {
       staff_repo: Arc<dyn StaffRepository>,
   }

   impl WorkforceAclAdapter {
       /// 산단 변경 audit 에 기록할 actor 정보를 조회.
       pub async fn resolve_actor(
           &self,
           staff_id: StaffId,
       ) -> Result<ActorDto, AclError> {
           let staff = self.staff_repo.find_by_id(staff_id).await?;
           Ok(ActorDto::from(staff))   // Staff 엔티티 직접 노출 금지
       }
   }
   ```

3. **이벤트 구독** — `shared-kernel/events/` 의 타입으로만 이벤트를 발행·수신한다.
   컨텍스트 내부 도메인 이벤트(`IndustrialComplexAggregateChanged` 등) 를 직접 노출하지 않는다.

### Cargo Import 강제

Cargo workspace 의 dependency 선언으로 ACL 우회를 컴파일 단계에서 차단한다.

```toml
# crates/catalog/catalog-domain/Cargo.toml
[dependencies]
shared-kernel = { path = "../../shared-kernel" }
# workforce-domain 직접 의존 금지 — Cargo 가 lint 보다 강하게 차단

# crates/catalog/catalog-infra/Cargo.toml — ACL adapter 만 workforce 접근 허용
[dependencies]
catalog-domain     = { path = "../catalog-domain" }
catalog-app        = { path = "../catalog-app" }
workforce-domain   = { path = "../../workforce/workforce-domain", optional = true }
# optional + feature flag 로 ACL 모듈에서만 활성화

[features]
default = []
workforce-acl = ["dep:workforce-domain"]
```

추가로 `cargo deny` 또는 `cargo-machete` 로 의도하지 않은 transitive dependency 를 CI 에서 차단한다.

### Ubiquitous Language 사전

**Catalog Context**

| 용어 | 정의 |
|---|---|
| IndustrialComplex | 산업단지 마스터 레코드. 행정구역·면적·관리 주체 등 ETL 기반 사실 데이터의 집합 |
| Parcel | 산단 내 개별 필지. `PNU` 19자리 식별자, kind(공장용지/지원시설 등), 면적, 폴리곤 좌표를 가짐 |
| Building | 필지 위의 건축물. 용도(`PurposeCode`), 구조(`StructureCode`), 23개 표준 필드 |
| Manufacturer | 입주 제조업체 마스터. KSIC 업종 코드, 사업자번호 (PII 는 별도 보호 레이어) |
| PNU | 19자리 부동산 고유번호 — gongzzang shared-kernel 과 동일 검증 규칙 (ADR 0018) |
| ParcelKind | 필지 종류 enum (공장용지 / 지원시설 / 도로 / 녹지 / 하천 등) |
| Blueprint | 특정 IndustrialComplex 에 붙는 도면/평면/배치 원본. Dawneer 는 이를 참조해 보여줄 뿐 원장을 소유하지 않음 |
| SpatialLayer | complex/parcel/zone geometry, polygon, layer metadata. 산단 공간 표현의 canonical source |
| DigitalTwin3d | complex/parcel/building 에 연결되는 3D 모델 또는 digital twin asset metadata |
| IndustryGroup | 산단의 허용/유치업종 taxonomy |
| ParcelIndustryAssignment | 필지와 유치업종/허용업종의 배정 사실 |

**Workforce/Authz Context**

| 용어 | 정의 |
|---|---|
| Staff | 인증된 내부 직원 계정 (Zitadel staff organization 의 사용자) |
| StaffRole | 권한 집합에 붙인 이름 (MASTER_ADMIN, COMPLEX_EDITOR, BLUEPRINT_VIEWER 등) |
| StaffPermission | 단일 작업 수행 허가 (`complex:update`, `parcel:assign_industry`, `staff:invite` 등) |
| StaffSession | Zitadel ID Token 검증 후 platform-core 가 발급하는 단기 세션 + JTI |
| ZitadelClaims | Zitadel ID Token 의 표준 클레임 + custom role claim — `crates/auth` 의 `Claims` 와 동일 구조 |

**용어 충돌 주의**: gongzzang 에는 B2C 일반 사용자를 가리키는 `User` 엔티티가 있다.
platform-core Workforce 의 `Staff` 와 절대 혼용 금지. 두 개념은 별도 Zitadel
organization 에 속하며 권한 모델이 다르다.

## 영향

### 긍정적 영향

- Catalog 와 Workforce 독립 테스트 가능: 각 컨텍스트는 상대방을 ACL 인터페이스로만 mock
- 보안 정책 변경(Workforce) 이 Catalog 리그레션 테스트를 트리거하지 않음
- 새 개발자 온보딩 시 컨텍스트 범위가 명확해 학습 곡선 감소
- 미래에 트래픽이 증가하면 두 컨텍스트를 옵션 B (물리적 분리) 로 분리 가능 — 코드 경계가
  이미 모듈/crate 단위로 분리되어 있어 추출 비용이 낮다

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| ACL 보일러플레이트 증가 | ACL 생성 스캐폴딩 (`cargo gen acl` 또는 macro_rules) 제공 |
| 컨텍스트 간 이벤트 스키마 버전 드리프트 | ADR 0033 Guardrail #4 (schema registry + `cargo-public-api`) 강제 |
| 개발자의 경계 무시 유혹 | Cargo workspace dependency 선언 자체가 차단, 추가로 `cargo deny` CI |
| Staff 와 User 혼동 가능성 | 타입 시스템 newtype 으로 분리 — `StaffId(Uuid)`, `UserId(Uuid)` 컴파일 단계 충돌 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 도메인 언어가 컨텍스트별로 일관, 산단/직원 용어 혼용 방지 |
| 2. 보안 | Workforce Context 격리 — Zitadel 검증 로직 변경의 영향 범위가 워크포스 crate 에 국한 |
| 3. 신뢰성 | 컨텍스트 독립 테스트로 회귀 탐지 속도 향상 |
| 4. 관찰가능성 | 컨텍스트별 독립 메트릭 레이블 (`bc=catalog` / `bc=workforce`) |
| 5. 성능 | 각 컨텍스트의 캐시 전략 독립 조정 가능 (Catalog 는 TTL 300s, Workforce 는 60s) |
| 6. 운영성 | 컨텍스트별 CODEOWNERS 지정으로 리뷰 책임 명확화 |
| 7. 확장성 | 필요 시 컨텍스트를 독립 서비스로 분리하는 이주 경로 확보 |

## 재검토 트리거

- Catalog Context 와 Workforce/Authz Context 간 ACL 호출이 월 500만 건을 초과하는 경우
  (옵션 B 물리적 서비스 분리 검토)
- 두 컨텍스트가 동일한 DB 테이블을 공유해야 하는 신규 요건이 발생하는 경우 (요건 자체 재검토)
- 보안 감사에서 ACL 경계 우회(예: `workforce-domain` 직접 import) 가 발견되는 경우
- B2C `User` 와 `Staff` 의 인증 모델이 통합되어야 한다는 비즈니스 요건이 등장하는 경우
  (Workforce Context 의 범위 재정의 — 현재 ADR 의 핵심 가정이 깨짐)

## 참고

- [ADR 0030](./0030-three-service-architecture.md) — γ' Three-Service Architecture 채택
- [ADR 0032](./0032-eventual-consistency-strategy.md) — Eventual Consistency 전략
- [ADR 0033](./0033-seven-guardrails-enforcement.md) — 7 Guardrails 자동 강제 방법
- [ADR 0018](./0018-pnu-first-identity-no-coordinates.md) — PNU 식별자 정책 (Catalog Context 에 그대로 적용)
- [ADR 0005](./0005-auth-zitadel.md) — Zitadel 채택 (Workforce Context 의 upstream)
- Eric Evans, *Domain-Driven Design* — Bounded Context 패턴
- Vaughn Vernon, *Implementing Domain-Driven Design* — Context Map 패턴
