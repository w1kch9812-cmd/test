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
crates/data-clients/
├── raw-capture/           RawCapture trait + NoOpRawCapture (SP4-iii-d)
├── vworld/                V-World 외부 API 클라이언트 (SP4-ii)
└── data-go-kr/            data.go.kr 건축물대장 + DataGoKrBuildingReader (SP4-iii-a)
crates/circuit-breaker/    Policy + Breaker + execute (SP4-ii)
crates/db/                 8+ PgRepository + PgRawCapture (SP4-iii-d)
services/api/              Axum HTTP server (Walking Skeleton, 3 endpoint)
services/outbox-publisher/ Outbox publisher binary (SP4-i)
```

총 **31 crate, ~1232 tests (1130 단위 + 102 통합), Rust 1.88.**

## CI 상태

- SP2 종료 (`51647a5`)까지 3 workflow 그린
- SP3 T1-T8 (`51b4b50`-`30b9c47`) 진행 중 모두 그린 유지 (T7 의 walking-skeleton 일시 빨강 의도 — Zitadel 미통합 상태)
- SP3 T9 first attempt (`9ad70e2`-`1c39b96`) 7 iter 실패 후 GH Actions billing block
- SP3 T9 재작업 (`447d767`) Mock JWT 모드로 푸시 완료, billing block 으로 검증 미완

3 workflow 정의:
- CI (7 jobs): lint, clippy, fmt, cargo-check, cargo-deny, tarpaulin ≥90%, secret scan
- db-migrations: PG17+PostGIS 컨테이너 + V001-V003 마이그 + immutable trigger 검증
- walking-skeleton: HTTP API 빌드 + `AUTH_DEV_MODE=true` 로 mock JWT round-trip 6단계 (T9 재작업 후)

## Rust 툴체인

1.88.0 (변동 없음).

## 다음 단계

- **SP4-iii-b** (실거래가, 1-2일): data.go.kr 실거래가 API + RealTransactionReader.
  같은 패턴 (DataGoKrClient 재사용 + 실거래가 endpoint 추가)
- **SP4-iii-c** (법제처, 1-2일): 법제처 API + 도시계획 텍스트 fetch
- **SP4-iii-e** (R2 Reader 6, 2-3일): PMTiles 정적 ETL + IndustrialComplexReader,
  ManufacturerReader, 정확한 BuildingFootprintReader (FU 40)
- **SP6 분해**: Frontend (Next.js + React 19, 4-7일) — 인증/매물/북마크/알림
  핸들러가 SP5-* 의 PgRepository + V-World ParcelReader 활용
- **SP3 후속 deferred**: 진짜 Zitadel staging 통합 테스트 (`docs/auth/staging-zitadel-integration.md` 사연 기록 — Zitadel v4 PAT opaque + healthz race + billing 비용)
- 7: 관측성 (Grafana, Prometheus, Loki, Tempo, Sentry — Outbox publisher metrics + Breaker open alert)
- 8: IaC (Pulumi RDS/R2/ECS)
- 9-12: 데이터 파이프라인, AI 어시스턴트, 검색, etc.

## 환경 (2026-05-04 SP4-ii 종료 시점)

- **MSVC Build Tools 2022 설치 완료** — winget silent install (사용자 승인).
  `cl.exe` / `link.exe` 위치: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx86\x86\`
- **로컬 `cargo clippy --workspace --all-features -- -D warnings`** 가능 (CI 동일 명령)
  · 단, dev shell 활성화 필요: `& "${vsPath}\Common7\Tools\Launch-VsDevShell.ps1" -Arch amd64 -HostArch amd64 -SkipAutomaticLocation`
- 다음 SP 시작 시 로컬 진단 → 1번 push 로 CI 그린 가능

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
- **GH Actions billing**: 7-iter Zitadel 시도 후 한도 도달. 매 PR 마다 무거운 인프라 부팅 (Zitadel 컨테이너 5 분+) 은 SSS 비용 효율 미달 — staging 환경에 분리하는 게 합리
