# Sub-project 2 — DB + Core 도메인 설계

| | |
|---|---|
| **작성일** | 2026-05-02 |
| **상태** | Draft (사용자 검토 대기) |
| **타입** | Foundation Layer 2 (sub-project 1 헌법 위에 첫 코드 + 데이터 모델) |
| **소요 추정** | 2-3주 |
| **결과물** | RDS 마이그레이션 V001 + Rust 도메인 모델 + R2 정적 데이터 어댑터 + 첫 단위 테스트 |
| **선행** | sub-project 1 (charter + monorepo) ✅ 완료 |

---

## 1. 목적 (Why)

Sub-project 1이 *집의 설계도면*이었다면, sub-project 2는 *방의 골조 + 벽 + 배관*입니다.

본 sub-project가 끝나면 다음이 가능:
- 한국 산업용 부동산 도메인의 *모든 데이터 종류*가 코드로 표현됨
- 첫 PostgreSQL 마이그레이션이 가능한 18개 테이블 정의
- R2의 정적 공공 데이터 (필지·건축물·산업단지·실거래·경매·법령)를 읽는 어댑터
- 모든 값 객체 (Pnu, Money, Area, BusinessNumber 등)가 Rust로 구현
- 어드민 운영을 위한 *데이터 모델 + 운영 흐름* 디자인

이 골조 위에 sub-project 3(인증), 4(외부 API 통합), 5(API endpoint), 6(프론트엔드)이 쌓인다.

---

## 2. 범위 (Scope)

### 2.1 포함 (In Scope)

- **Rust 도메인 모델** — 6 Aggregate (User, Listing, Parcel, Building, IndustrialComplex, Manufacturer) + 4 Aggregate (RealTransaction, CourtAuction, Bookmark, Notification 등)
- **값 객체** — Pnu, Money, Area, BusinessNumber, BrokerLicense, Geometry, AdminDivision 등
- **Repository trait** (Port) 모든 Aggregate별
- **18개 RDS 테이블** + 첫 마이그레이션 V001__init.sql
- **DB role 분리** (writer / reader / audit_archiver) — V002__db_roles.sql
- **R2 정적 데이터 어댑터** (R2 reader trait + 구현체)
- **R2 디렉토리 구조** 정의 + presigned URL 생성 헬퍼
- **단위 테스트** 모든 값 객체 + 도메인 로직 (커버리지 90%+)
- **Pipeline schedule + run + steps JSONB** 데이터 모델
- **어드민 운영 데이터 모델** (verification queue, review queue, report, featured, alert, admin_action)
- **공유 위젯 데이터 계약** (AuditLogWidget, MetricsWidget 등이 사용할 쿼리 패턴)

### 2.2 제외 (Out of Scope)

- **API endpoint** 구현 — sub-project 5
- **외부 API 호출** (V-World, data.go.kr 등) 실제 통합 — sub-project 4
- **인증 미들웨어** (Zitadel JWT 검증) — sub-project 3
- **Repository 구현체** (SQLx) — *trait만* 정의, 구현은 sub-project 5
- **PMTiles 생성 워커** — sub-project 9 (data-pipeline)
- **어드민 UI 화면** — sub-project 6
- **관측성 (OTel/Tempo) 통합** — sub-project 7
- **인프라 프로비저닝** (Pulumi RDS/R2 셋업) — sub-project 8

### 2.3 결정 보류

- Outbox publisher 서비스 구현 — sub-project 4
- pgvector 임베딩 컬럼 — sub-project 11 (검색)
- 멀티 테넌시 (B2B 임직원 관리) — Phase 4+
- Read replica — Phase 4+

---

## 3. 핵심 의사결정 (이미 합의됨)

| # | 결정 | 출처 |
|---|------|------|
| 1 | 백엔드 = Rust + Axum + SQLx | ADR-0001 |
| 2 | DB = PostgreSQL 17 + PostGIS | ADR-0004 |
| 3 | 인증 = Zitadel | ADR-0005 |
| 4 | 캐시 = moka L1 + Valkey L2 | ADR-0007 |
| 5 | 옵션 A 데이터 플랫폼 (AI 생성 X) | ADR-0010 |
| 6 | 임베딩 = Gemini + pgvector (Phase 3+) | ADR-0011 |
| 7 | **경매 포함** (CourtAuction Aggregate, Market BC) | sub-project 2 brainstorming |
| 8 | **Index/Detail 분리 안 함** — 단일 테이블 + 두 endpoint | sub-project 2 brainstorming |
| 9 | **Listing 거래유형** = 매매 / 월세 / 전세 (산업용 + 한국 시장) | sub-project 2 brainstorming |
| 10 | **데이터 저장** = 정적(R2) + 동적(RDS) 분리 | sub-project 2 brainstorming |
| 11 | **객체 스토리지** = Cloudflare R2 (S3 호환, egress 무료) | sub-project 2 brainstorming |
| 12 | **단일 schema** (12 schema 분리는 YAGNI) — `public` schema 사용 | sub-project 2 brainstorming |
| 13 | **갱신 주기 어드민 동적 관리** — pipeline_schedule 테이블 | sub-project 2 brainstorming |
| 14 | **변경 감지 + 멱등성** — 해시 비교 + advisory lock + 시도별 shard 단위 | sub-project 2 brainstorming |
| 15 | **사진** = listing_photo 별도 테이블 (R2 key 저장, presigned URL 동적 생성) | sub-project 2 brainstorming |
| 16 | **Bookmark** = 하이브리드 (bookmark_listing FK + bookmark_external polymorphic) | sub-project 2 brainstorming |
| 17 | **Audit retention** = 1년 RDS + 6년 R2 IA archive (총 7년 PIPA + ISMS-P) | sub-project 2 brainstorming |
| 18 | **Audit immutable** = 별도 DB role + R2 Object Lock | sub-project 2 brainstorming |
| 19 | **시각화** = Grafana embed (서비스 맵, 트레이스, 메트릭) + 자체 UI (파이프라인 단계 진행) | sub-project 2 brainstorming |
| 20 | **어드민 UI** = 컨텍스트 중심 9 화면 + 공유 위젯 (기능별 분리 X) | sub-project 2 brainstorming |

---

## Design Parts

Detailed design sections are split by responsibility so this spec remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Domain Classification And Core Tables](./2026-05-02-sub-project-2-db-core-domain-design.part-01-domain-core-tables.md)
- [Part 02 - System, Pipeline, Admin Tables, And DB Roles](./2026-05-02-sub-project-2-db-core-domain-design.part-02-system-admin-db-roles.md)
- [Part 03 - R2 Static Data And Rust Domain Code](./2026-05-02-sub-project-2-db-core-domain-design.part-03-r2-rust-domain.md)
- [Part 04 - Admin Preview, Verification, Risks, And References](./2026-05-02-sub-project-2-db-core-domain-design.part-04-verification-risks.md)
