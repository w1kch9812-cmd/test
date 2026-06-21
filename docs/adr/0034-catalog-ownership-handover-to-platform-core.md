# ADR 0034 - Catalog 도메인 소유권의 platform-core 이양

| | |
|---|---|
| 작성일 | 2026-05-11 |
| 상태 | Accepted |
| 선행 | [ADR 0030](./0030-three-service-architecture.md), [ADR 0031](./0031-platform-core-bounded-contexts.md), [ADR 0032](./0032-eventual-consistency-strategy.md) |

## 구현 상태 (2026-05-28)

M3.2 physical extraction 이 gongzzang workspace 에 강제되었다.
`industrial-complex`, `parcel`, `building`, `manufacturer`, `vworld`, `data-go-kr`,
`raw-capture`, `data-pipeline-control` crate 는 gongzzang workspace 에 존재하면 안 된다.
`docs/architecture/platform-core-boundary.v1.json` 이 현재 phase 와 path ownership 의 SSOT 이며,
`scripts/lefthook/catalog-m1-boundary.sh` 와 그 boundary contract 가 재도입을 차단한다.
`docs/architecture/platform-core-catalog-api-contract.v1.pin.json` pins the Gongzzang
Catalog API consumer surface, kept consistent with Platform Core OpenAPI
(`../platform-core/docs/openapi/catalog.v1.yaml`) when the sibling repo is present.
The `parcel-lookup` crate is only the Gongzzang port/projection crate; Platform Core
HTTP adapters live in `services/api` and must use `Policy::platform_core_default()`
through `circuit_breaker::execute`.
Platform Core-owned ETL service scaffolds (`services/data-pipeline`, `services/scraper-py`)
are also extracted from Gongzzang and guarded by the same boundary SSOT.
Public/reference vector tile ETL configuration, workflows, Docker/tooling, and
Catalog public API drift observability are also Platform Core-owned. Therefore
`crates/sp9-base-layer-config`, SP9 GitHub workflows, tippecanoe setup tooling,
`services/etl-base-layer/Dockerfile.etl`, `services/etl-base-layer/scripts`,
`.github/workflows/api-drift-smoke-test.yml`, `crates/operations/api-health`,
`crates/api-health-recorder`, `crates/db/src/api_health.rs`, and
`docs/observability/api-drift-smoke-test.md` must not exist in Gongzzang.
Boundary enforcement also scans GitHub workflow YAML for Catalog source API
tokens, so the same jobs cannot be reintroduced under a new filename.
DB migration history 에 남은 `parcel_external_data`, `api_health_check`,
`pipeline_schedule`, `pipeline_run` 계열은 `allowed_legacy_schema_tokens` ledger 로만
허용된다. 사용자 승인 후 추가된 Gongzzang DB cleanup migration
`migrations/30015_drop_platform_core_legacy_schema.sql` 이 runtime table 을 drop 하며,
Platform Core DB 는 건드리지 않는다.
Gongzzang-owned `db-migrations.yml` must keep running `tests/migrations/test_v001_full.sh`
against disposable PostGIS, and the boundary gate verifies that smoke expects the
legacy Platform Core tables to be absent after the cleanup migration.
`forbidden_canonical_catalog_tables` 는 `industrial_complex`, `parcel`, `building`,
`manufacturer` 의 직접 SQL DDL/DML/query 사용을 차단한다. Gongzzang 의 bookmark,
featured content 같은 product-owned 기능은 이들을 외부 target kind 로 참조할 수 있지만,
canonical Catalog table 을 로컬에 만들거나 읽거나 쓰면 안 된다.
Boundary enforcement blocks both unqualified and schema-qualified table references
such as `building` and `catalog.parcel`. It also rejects direct Platform Core
database connection aliases such as `PLATFORM_CORE_DB_URL`; Gongzzang must use
published HTTP/event/artifact contracts only.
Root env examples must document only HTTP Platform Core contracts
(`PLATFORM_CORE_API_BASE_URL`, `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL`) and must not
expose Platform Core-owned Catalog source, ETL, or generic raw-data R2 settings.
Local Gongzzang Postgres defaults to host port `15432`; `5500` is not used because
it can be reserved by Windows excluded port ranges.
M1 시절 Gongzzang shared-kernel 에 남아 있던 Catalog event schema module 도 제거되었고,
재도입 시 boundary gate 가 차단한다. Catalog event schema 의 owner 는 Platform Core 이며,
Gongzzang 은 webhook receiver/pinned contract copy 만 보유한다.

## 결정

gongzzang 의 `crates/domain/core/{industrial-complex, parcel, building, manufacturer}` 와
관련 ETL 파이프라인 (`crates/data-clients/{vworld, data-go-kr}`, `crates/data-pipeline-control`) 의
**소유권은 platform-core 로 이양** 한다. gongzzang 은 마이그레이션 phase 가 끝나면 이 영역의
consumer 가 되며, 동일 데이터를 자체 DB 에 보유하지 않는다.

Catalog 의 범위는 산업단지 자체와 그 하위 리소스 전체다. 도면, 공간 레이어, polygon,
3D/digital twin, 유치업종/허용업종, 필지별 업종 배정처럼 IndustrialComplex 에 종속되는
canonical fact 또는 operational subobject 는 platform-core 가 소유한다. Dawneer 는 이를
참조해 사이트별 표시 설정과 마케팅 override 만 소유한다.

이양 전·후의 의존 방향:

```
[이양 전]                              [이양 후]
gongzzang ─── owns ──> Catalog        gongzzang ── consumes ──> platform-core/Catalog
                                                                       │
                                                                       owns Catalog
```

## 컨텍스트

ADR 0030 에서 γ' Three-Service 를 채택했고, ADR 0031 에서 Catalog Context 가
platform-core 에 위치한다고 결정했지만, **언제·어떻게 gongzzang 의 기존 Catalog
crate 가 사라지는가** 가 명시되지 않았다. 이 ADR 이 그 공백을 메운다.

### 1. 현재 보유 자산

gongzzang 안에 다음이 존재한다 (2026-05-11 기준 main).

| 자산 | 위치 | 역할 |
|---|---|---|
| `IndustrialComplex` entity | `crates/domain/core/industrial-complex` | 산단 마스터 |
| `Parcel` entity (PNU 검증 포함) | `crates/domain/core/parcel` | 필지 마스터 |
| `Building` entity (23 필드) | `crates/domain/core/building` | 건축물 |
| `Manufacturer` entity | `crates/domain/core/manufacturer` | 제조사 |
| `shared-kernel::Pnu` | `crates/domain/core/shared-kernel` | PNU 19자리 value object |
| V-World 클라이언트 | `crates/data-clients/vworld` | 외부 ETL 소스 |
| data.go.kr 클라이언트 | `crates/data-clients/data-go-kr` | 외부 ETL 소스 |
| ETL 오케스트레이션 | `crates/data-pipeline-control` | 주기적 fetch + 정규화 |
| Postgres 테이블 | `industrial_complex`, `parcel`, `building`, `manufacturer` | DB 사실 데이터 |
| 산단 하위 리소스 | blueprint, spatial layer, polygon, 3D asset, industry assignment | IndustrialComplex 에 종속되는 catalog 사실 또는 운영 리소스 |

이 자산이 platform-core 의 Catalog Context 와 1:1 으로 겹친다. 두 곳에 동일 사실
데이터가 존재하면 ADR 0033 G1 (서비스 간 직접 DB 접근 금지) 와 G3 (공유 mutable state 금지)
의 invariant 가 깨진다.

### 2. 이양의 두 가지 측면

**소유권 이양** (이 ADR) ≠ **코드 삭제**. 정확한 sequencing 은 다음과 같다.

| Phase | gongzzang 측 상태 | platform-core 측 상태 |
|---|---|---|
| M1 | owner (단일) | 빈 상태, shadow read 가능 |
| M2 | owner (계속) | shadow read consumer (drift 감지) |
| M3.2 시작 | dual-write (gongzzang + platform-core) | dual-write |
| M3.2 cutover | read 전환 (platform-core API consumer) | sole owner |
| M3.4 종료 | crate 와 DB 컬럼 deprecation | sole owner |
| Post-M3 | crate 삭제 또는 read model cache only | sole owner |

이 ADR 은 **최종 상태가 무엇이며 어느 ADR 이 그 sequencing 을 정의하는가** 를 선언한다.
구체적 sequencing 은 [`platform-core/docs/migration/2026-05-11-platform-core-extraction.md`](https://github.com/perfectoryinc/platform-core/blob/main/docs/migration/2026-05-11-platform-core-extraction.md)
가 단일 소스다.

## 검토한 옵션

### 옵션 A — 양쪽 영구 보유

**채택 불가**. ADR 0030 의 결정 자체를 무효화한다. 산단 마스터 데이터의 이중 보유는
ADR 0033 G1·G3 invariant 의 항구적 위반.

### 옵션 B — gongzzang 이 owner, platform-core 는 read-only mirror

**채택 불가**. ADR 0031 의 "Catalog Context 가 platform-core 에 위치" 결정과 충돌.
ETL 책임을 platform-core 로 이양하지 못하면 dawneer 가 다시 별도 ETL 을 가져야 하고
3개 ETL 이 같은 외부 API 를 두드리는 anti-pattern 으로 회귀한다.

### 옵션 C — platform-core 가 owner, gongzzang 은 consumer (채택)

ADR 0030~0031 의 자연스러운 귀결. 단방향 의존 (`gongzzang → platform-core`) 으로
일관성과 캐시 무효화 경로가 명확해진다.

## 채택 (C) — 구현 세부

### gongzzang 측 변경 시점

| 변경 | M1 | M2 | M3.2 | M3.4 | Post-M3 |
|---|---|---|---|---|---|
| `industrial-complex` crate 코드 | 유지 | 유지 | dual-write 추가 | read 경로 비활성 | 삭제 또는 cache-only |
| Postgres `industrial_complex` 테이블 | 유지 | 유지 | 유지 (write 양쪽) | read-only mark | DROP 또는 cache view |
| V-World ETL 호출자 | 유지 | 유지 | platform-core 와 분리 시작 | 비활성 | 삭제 |
| `data-pipeline-control` orchestrator | 유지 | 유지 | platform-core 로 이관 시작 | 비활성 | 삭제 |
| 어드민 UI 가 catalog API 호출 | 자체 | 자체 | dual | platform-core API | platform-core API |

### B2C 일반 사용자 (`User`) 는 별개

이 ADR 은 **Catalog 영역만** 이양 대상이다. gongzzang.com 의 B2C 사용자 (`User`,
`BookmarkListing`, `SearchHistory`, `AnalysisReport` 등 insights 도메인) 는 gongzzang 이
영구 owner 다. ADR 0031 의 "Workforce Context = 내부 Staff 만" 가정과 일관.

### `shared-kernel::Pnu` 처리

PNU 19자리 검증 규칙은 두 repo 가 동일해야 한다 (ADR 0018 의 invariant).

- M1~M2: gongzzang 의 `shared-kernel::Pnu` 와 platform-core 의 `platform-shared-kernel::Pnu` 가 동일 규칙으로 **병렬 존재**. CI 가 양쪽에 동일 property test 적용.
- M3 종료 후: gongzzang 이 platform-core 의 published API DTO (PNU 가 검증된 String 으로 옴) 만 받으므로 자체 검증은 inbound boundary 한 곳에만 유지.

규칙이 drift 하면 ADR 0033 G2 (typed contract) 위반으로 CI 가 차단.

## 영향

### 긍정적 영향

- ETL 일원화: V-World / data.go.kr API rate limit 가 한 곳에서만 소비됨
- 산단 마스터 데이터 single source: Dawneer 도면/3D/사이트 표시와 gongzzang 지도가 같은 fact 를 본다
- Read model 자유: gongzzang 은 listing 검색 인덱스 등 자기만의 read model 을 자유롭게 derive (Outbox 이벤트 구독)

### 부정적 영향 및 완화책

| 위험 | 완화 |
|---|---|
| ETL 이전 중 빈 catalog | M1 의 shadow phase 가 7일 drift < 0.01% 검증 통과 후에만 M2 진입 |
| gongzzang 의 ETL crate 가 platform-core 로 이전 시 PR 거대화 | crate 단위 분할 PR (vworld, data-go-kr, pipeline-control 각각 별도) |
| 검색·매트릭 read model 재계산 필요 | `ParcelKindChanged.v1` 등 Outbox 이벤트로 점진 재계산 |
| 운영 인시던트 시 gongzzang 만으로 응급 catalog 수정 못함 | M3 종료까지 dual-write 윈도우 유지, 응급 break-glass 절차 |

## SSS 7기둥 매핑

| 기둥 | 적용 |
|---|---|
| 1. 코드 품질 | 도메인 ownership 가 단일 — gongzzang 이 cache/read model 만 가질 때 그 책임 경계가 명확 |
| 2. 보안 | ETL 자격증명 / PII (사업자번호) 가 한 서비스에만 존재 |
| 3. 신뢰성 | 단일 source 이므로 dual-write drift 가 영구 invariant 가 아닌 limited window (M3.2 만) |
| 4. 관찰가능성 | Outbox lag 모니터링이 한 곳 (platform-core) 에 집중 |
| 5. 성능 | gongzzang 이 join 으로 catalog 데이터를 가져오는 경로가 캐시 hit 으로 단순화 |
| 6. 운영성 | ETL 장애 시 영향 범위가 명확 (gongzzang 의 listing 검색 정도에 한정) |
| 7. 확장성 | dawneer 가 자체 ETL 없이 동일 catalog 사용 |

## 재검토 트리거

- M3.2 cutover 후 gongzzang 의 catalog 의존 latency P99 가 200ms 초과 → caching 전략 재검토
- platform-core 가 6개월 내 99.9% uptime 미달성 → Catalog Context 만 별도 서비스로 빼는 옵션 B 재검토 (ADR 0031 옵션 B)
- B2C 사용자가 catalog 데이터 mutating 권한이 필요한 신규 요구 → 이 ADR 의 "B2C User 는 별개" 가정 재검토

## 참고

- [ADR 0030](./0030-three-service-architecture.md) — γ' Three-Service 채택
- [ADR 0031](./0031-platform-core-bounded-contexts.md) — Catalog/Workforce Context 정의
- [ADR 0032](./0032-eventual-consistency-strategy.md) — dual-write window 의 일관성 전략
- [ADR 0033](./0033-seven-guardrails-enforcement.md) — G1, G3, G6 위반 차단
- [ADR 0018](./0018-pnu-first-identity-no-coordinates.md) — PNU 검증 규칙 (양 repo 동일 유지)
- `platform-core/docs/migration/2026-05-11-platform-core-extraction.md` — sequencing SSOT
