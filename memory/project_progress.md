---
name: 프로젝트 진행 현황 (2026-05-02)
description: Sub-project 1, 2a, 2a-fixup, Walking Skeleton, 2b-i 완료 상태 + 누적 산출물
type: project
---

## 완료된 Sub-projects

### Sub-project 1: 헌법 + 모노레포 (완료)
- 132 파일 (헌법 + ADR 11개 + 컨벤션 9개 + 모노레포 골격)
- 자동 강제 도구: lefthook, gitleaks, biome, clippy, cargo-deny, markdownlint

### Sub-project 2a: DB + shared-kernel (완료, 31 task)
- 18 RDS 테이블 (V001 5분할) + V002 (3 role + audit immutable trigger)
- shared-kernel crate, 14 값 객체 (Pnu, Money, Email, BusinessNumber + checksum 등)
- 167 단위 테스트, tarpaulin ≥90% CI 게이트

### Sub-project 2a-fixup: spec 결함 5건 보강 (완료)
- V003_01: listing transaction_type cross-field CHECK
- V003_02: BVQ + LRQ optimistic locking version
- V003_03: featured_content ends_at > starts_at CHECK
- BusinessNumber 000xxxxxxx prefix 거부
- PhoneKr +82 명시적 prefix만 strip

### Walking Skeleton (완료, T1-T5)
- User Aggregate minimal (`crates/domain/core/user`)
- PgUserRepository (SQLx, `crates/db`)
- Axum HTTP server (`services/api`, 3 endpoint)
- CI smoke test workflow (`.github/workflows/walking-skeleton.yml`) — POST /users + GET /users/:id round-trip
- 로컬 시연 검증 (psql 직접 + DB invariant 강제 동작)

### Sub-project 2b-i: Core BC RDS Aggregates (완료, T1-T13)
- 워크스페이스 재구조 (shared-kernel → `crates/domain/core/`)
- 6 신규 값 객체 (ListingType, TransactionType, ListingStatus + 상태 머신, ContactVisibility, ListingTitle, Description)
- User Aggregate full (18 필드, 13 도메인 메서드, soft-delete)
- Listing Aggregate full (20 필드, 9 도메인 메서드, 상태 머신, V003_01 invariant)
- ListingPhoto Aggregate (12 필드, soft-delete + reorder)
- Repository trait 3개 (User/Listing/ListingPhoto), 모두 port-only (구현은 sub-project 5)
- 348 단위 테스트 누적

### Sub-project 2b-ii: Core BC R2 정적 Reader (완료, T1-T8)
- shared-kernel 추가: LandUseType (지목 9), Zoning (용도지역 5), PolygonSrid, BoundingBox, AdminDivision composite
- 4 R2 정적 BC 신규 crate:
  - **Parcel** (10 필드 + ParcelMarker 마커 projection)
  - **Building** (12 필드 + BuildingPurposeCode 10 + BuildingStructureCode 8)
  - **IndustrialComplex** (8 필드 + IndustrialComplexKind 4)
  - **Manufacturer** (9 필드 + EmployeeCountBand 6)
- Reader trait 4개, 모두 read-only port (구현은 sub-project 4)
- 466 단위 테스트 누적, 99% 커버리지

## 워크스페이스 구조 (현재)

```
crates/domain/core/
├── shared-kernel/         24 모듈 (값 객체 ALL — 14 + 6 from 2b-i + 4 from 2b-ii)
├── user/                  User Aggregate (RDS 동적, 46 tests)
├── listing/               Listing Aggregate (RDS 동적, 46 tests)
├── listing-photo/         ListingPhoto Aggregate (RDS 동적, 20 tests)
├── parcel/                Parcel Reader (R2 정적, 10 tests)
├── building/              Building Reader (R2 정적, 28 tests with enums)
├── industrial-complex/    IndustrialComplex Reader (R2 정적, 18 tests)
└── manufacturer/          Manufacturer Reader (R2 정적, 18 tests)
crates/db/                 PgUserRepository (Walking Skeleton, 사용자만)
services/api/              Axum HTTP server (Walking Skeleton, 3 endpoint)
```

총 **10 crate, 466 단위 테스트, 99% 커버리지.**

## CI 상태

3 workflow 모두 그린 (commit `b64741b` 시점):
- CI (7 jobs): lint, clippy, fmt, cargo-check, cargo-deny, tarpaulin ≥90%, secret scan
- db-migrations: PG17+PostGIS 컨테이너 + V001-V003 마이그 + immutable trigger 검증
- walking-skeleton: 실제 HTTP API 빌드 + 백그라운드 실행 + curl POST/GET round-trip

## Rust 툴체인

1.88.0 (1.83 → 1.85 → 1.88 — 두 번 amendment, 모두 transitive deps 강제)
- Sub-project 2a Task 26: 1.83 → 1.85 (edition2024)
- WS T2: 1.85 → 1.88 (sqlx 0.8 + rustls)

## 다음 단계 (Sub-project 2 잔여)

### Plan 2c: Market BC + Insights BC + Operations BC + Pipeline + 도메인 이벤트
- RealTransaction, CourtAuction (Market BC)
- Bookmark, SearchHistory, Notification (Insights BC) — Aggregate
- Admin Operations 6개 Aggregate
- Pipeline schedule + run + steps JSONB
- Outbox event publisher trait

### 이후 Sub-projects
- 3: Auth (Zitadel JWT 미들웨어)
- 4: 외부 API 통합 (V-World, 법제처, data.go.kr)
- 5: Repository SQLx 구현 (3개 BC 모두)
- 6: Frontend (Next.js)
- 7: 관측성 (Grafana, Prometheus, Loki, Tempo, Sentry)
- 8: IaC (Pulumi RDS/R2/ECS)
- 9-12: 데이터 파이프라인, AI 어시스턴트, 검색, etc.

## 알려진 deferred items (production 배포 전 처리)

1. BusinessNumber NTS 체크섬 외부 검증 (실제 사업자번호 표본)
2. BusinessNumber D₃D₄ 사업자 유형 코드 검증
3. KsicCode 대분류 letter A-U 강제 (KSIC 11차 추적)

## 환경 알려진 한계

- Windows 로컬 빌드: MSVC Build Tools 부재로 cargo build/test/clippy 실행 불가. CI Linux가 최후 진실
- Postgres 포트 5432: Windows 예약 포트 범위 (5360-5459) 충돌 가능 — 로컬 시연 시 6432 같은 다른 포트 임시 사용
