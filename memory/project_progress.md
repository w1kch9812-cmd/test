---
name: 프로젝트 진행 현황 (2026-05-03)
description: Sub-project 1 + 2 (Core + Market + Insights + Operations + Pipeline + Audit + Outbox) 완료, Sub-project 3 진입 대기
type: project
---

## 완료된 Sub-projects

### Sub-project 1: 헌법 + 모노레포 (완료)
- 132 파일 (헌법 + ADR 11개 + 컨벤션 9개 + 모노레포 골격)
- 자동 강제 도구: lefthook, gitleaks, biome, clippy, cargo-deny, markdownlint

### Sub-project 2a: DB + shared-kernel (완료, 31 task)
- 18 RDS 테이블 (V001 5분할) + V002 (3 role + audit immutable trigger)
- shared-kernel crate, 14 값 객체 (Pnu, Money, Email, BusinessNumber + checksum 등)
- tarpaulin ≥90% CI 게이트

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
- CI smoke test workflow — POST /users + GET /users/:id round-trip

### Sub-project 2b-i: Core BC RDS Aggregates (완료, T1-T13)
- 워크스페이스 재구조 (shared-kernel → `crates/domain/core/`)
- 6 신규 값 객체 (ListingType, TransactionType, ListingStatus + 상태 머신, ContactVisibility, ListingTitle, Description)
- User Aggregate full (18 필드, 13 도메인 메서드, soft-delete)
- Listing Aggregate full (20 필드, 9 도메인 메서드, 상태 머신, V003_01 invariant)
- ListingPhoto Aggregate (12 필드, soft-delete + reorder)
- Repository trait 3개 (User/Listing/ListingPhoto), 모두 port-only

### Sub-project 2b-ii: Core BC R2 정적 Reader (완료, T1-T8)
- shared-kernel 추가: LandUseType, Zoning, PolygonSrid, BoundingBox, AdminDivision composite
- 4 R2 정적 BC 신규 crate: Parcel, Building, IndustrialComplex, Manufacturer
- Reader trait 4개, 모두 read-only port (구현은 sub-project 4)

### Sub-project 2c: Market + Insights + Audit + Pipeline + Operations BC (완료, 14 task — T1-T18)
- T1 RealTransaction Aggregate (Market BC)
- T2 CourtAuction Aggregate (Market BC)
- T3 Bookmark Aggregate (Insights BC)
- T4 SearchHistory Aggregate (Insights BC)
- T5 AnalysisReport Aggregate (Insights BC)
- T6 Notification Aggregate (Insights BC)
- T7 shared-kernel `DomainEvent` trait + ULID id 표준 (4 tests 추가)
- T8 AuditLog (Audit BC, immutable)
- T9-T10 OutboxEvent + Outbox 패턴 (Audit BC)
- T11-T12 PipelineSchedule + PipelineRun + steps JSONB (data-pipeline-control)
- T13 AdminAction (Operations BC)
- T14 BusinessVerificationQueue (Operations BC, optimistic locking)
- T15 ListingReviewQueue (Operations BC, optimistic locking)
- T16 ListingReport (Operations BC)
- T17 OperationsMeta (FeaturedContent + AlertHistory, 단일 crate)
- T18 통합 검증 + memory 갱신 (현재)

**누적**: 14 신규 crate (Market 2 + Insights 4 + Audit 2 + Pipeline 1 + Operations 5),
1017 단위 테스트, Rust 1.88, 24 workspace member.

## 워크스페이스 구조 (현재)

```
crates/domain/core/
├── shared-kernel/         24 모듈 + DomainEvent trait (값 객체 ALL, 298 tests)
├── user/                  User Aggregate (RDS 동적, 46 tests)
├── listing/               Listing Aggregate (RDS 동적, 46 tests)
├── listing-photo/         ListingPhoto Aggregate (RDS 동적, 20 tests)
├── parcel/                Parcel Reader (R2 정적, 10 tests)
├── building/              Building Reader (R2 정적, 28 tests)
├── industrial-complex/    IndustrialComplex Reader (R2 정적, 18 tests)
└── manufacturer/          Manufacturer Reader (R2 정적, 18 tests)
crates/domain/market/
├── real-transaction/      RealTransaction Aggregate (16 tests)
└── court-auction/         CourtAuction Aggregate (26 tests)
crates/domain/insights/
├── bookmark/              Bookmark Aggregate (20 tests)
├── search-history/        SearchHistory Aggregate (17 tests)
├── analysis-report/       AnalysisReport Aggregate (21 tests)
└── notification/          Notification Aggregate (16 tests)
crates/domain/audit/
├── audit-log/             AuditLog (35 tests, immutable)
└── outbox-event/          OutboxEvent (25 tests, Outbox 패턴)
crates/data-pipeline-control/  PipelineSchedule + PipelineRun (84 tests)
crates/operations/
├── admin-action/          AdminAction (33 tests)
├── business-verification-queue/  BVQ (49 tests, optimistic locking)
├── listing-review-queue/  LRQ (46 tests, optimistic locking)
├── listing-report/        ListingReport (59 tests)
└── operations-meta/       FeaturedContent + AlertHistory (86 tests)
crates/db/                 PgUserRepository (Walking Skeleton)
services/api/              Axum HTTP server (Walking Skeleton, 3 endpoint)
```

총 **24 crate, 1017 단위 테스트, Rust 1.88.**

## CI 상태

3 workflow 모두 그린 (commit `51647a5` 시점):
- CI (7 jobs): lint, clippy, fmt, cargo-check, cargo-deny, tarpaulin ≥90%, secret scan
- db-migrations: PG17+PostGIS 컨테이너 + V001-V003 마이그 + immutable trigger 검증
- walking-skeleton: 실제 HTTP API 빌드 + 백그라운드 실행 + curl POST/GET round-trip

## Rust 툴체인

1.88.0 (변동 없음).

## 다음 단계

- **Sub-project 3 (Auth)**: Zitadel JWT 미들웨어 (Axum tower) + User Aggregate 연결
- **Sub-project 4 (외부 API)**: Reader trait 구현체 (V-World, 법제처, data.go.kr)
- **Sub-project 5 (Repository SQLx 구현)**: 도메인 → DB 통합 (3개 BC 모두)
- 6: Frontend (Next.js)
- 7: 관측성 (Grafana, Prometheus, Loki, Tempo, Sentry)
- 8: IaC (Pulumi RDS/R2/ECS)
- 9-12: 데이터 파이프라인, AI 어시스턴트, 검색, etc.

## 알려진 deferred items (production 배포 전 처리)

1. BusinessNumber NTS 체크섬 외부 검증 (실제 사업자번호 표본)
2. BusinessNumber D₃D₄ 사업자 유형 코드 검증
3. KsicCode 대분류 letter A-U 강제 (KSIC 11차 추적)
4. **Spec FU 9** — `analysis_report.updated_at` 컬럼 추가 마이그 V003_04 (도메인 코드는 이미 반영, 스키마만 후속)
5. **Spec FU 10** — `outbox_event` prefix 표기 일관화 (plan body 표기 통일)
6. **Spec FU 11** — `featured_content` prefix `fc` → `fea` 표기 일관화 (spec § 5.5 inline)

## 환경 알려진 한계

- Windows 로컬 빌드: MSVC Build Tools 부재로 cargo build/test/clippy 실행 불가. CI Linux가 최후 진실
- Postgres 포트 5432: Windows 예약 포트 범위 (5360-5459) 충돌 가능 — 로컬 시연 시 6432 같은 다른 포트 임시 사용
