# 공짱 Sub-project Roadmap

> **Current supersession**: 2026-05-22 — listing marker placement and map-marker data flow are
> governed by [ADR 0018](../adr/0018-pnu-first-identity-no-coordinates.md),
> [ADR 0037](../adr/0037-pnu-anchor-pbf-marker-tiles.md), and the
> [Gongzzang-owned listing PBF design spec](./specs/2026-05-22-gongzzang-owned-listing-pbf-marker-tiles-design.md).
> Older roadmap entries mentioning `listing.geom_point`, bbox/bounds marker requests, or
> listing marker placement outside PNU anchors are historical only and must be revalidated against
> those documents before implementation.
>
> **Current gate**: the Gongzzang-owned listing PBF marker implementation slice has local
> verification evidence. This is not a whole-product launch completion claim; re-run the linked
> handoff/audit verification before changing or claiming this slice.
>
> **갱신일**: 2026-05-23 (listing photo upload confirmation and signed download hardening)
> **현재 구현 커밋**: `8946709` (listing photo downloads route through authenticated photo_id paths and R2 signed GET URLs)
> **SSOT**: 본 문서 — 다음 sub-project 결정/진행 시 *먼저* 갱신.

---

## 완료 (2026-05-05 기준)

| SP | 영역 | 주요 산출물 | 상태 |
|---|---|---|---|
| 1 | 헌법 + 모노레포 | 132 파일, lefthook/gitleaks/biome/clippy/cargo-deny 자동 강제 | ✅ |
| 2a | DB + shared-kernel | 18 테이블 V001 + 14 값 객체 | ✅ |
| 2a-fixup | spec 결함 5건 | V003_01/02/03, BusinessNumber checksum, PhoneKr prefix | ✅ |
| Walking Skeleton | API 골격 | User Aggregate + PgUserRepository + Axum 3 endpoint | ✅ |
| 2b-i | Core BC RDS Aggregates | User/Listing/ListingPhoto + 6 값 객체 | ✅ |
| 2b-ii | Core BC R2 Reader port | Parcel/Building/IndustrialComplex/Manufacturer | ✅ |
| 2c | Market+Insights+Audit+Pipeline+Operations | 14 task, 14 신규 crate | ✅ |
| **3** | Auth — Zitadel JWT 핵심 게이트 | `crates/auth` (Verifier enum + JwksCache + middleware), Mock JWT CI mode | ✅ |
| **5-i** | Core BC RDS Repository SQLx | PgListingRepository + PgListingPhotoRepository + PgUserRepository 18 필드 보강 | ✅ |
| **5-iii** | Audit + Pipeline + Operations RDS Repo + 트랜잭션 Outbox | MutationContext + 8 PgRepository + audit_log/outbox transactional 패턴 | ✅ |
| **5-iv** | Core BC `MutationContext` 일원화 | 3 trait 시그니처 + 3 PgImpl tx + auth middleware first_sign_in + 10 신규 통합 테스트 | ✅ |
| **4-i** | Outbox Publisher Worker | `crates/outbox-publisher` (Sink/tick/LoggingSink/CountingSink) + `services/outbox-publisher` daemon + 4 신규 통합 테스트 | ✅ |
| **5-ii** | Insights BC RDS Repository | PgBookmarkRepository (composite PK + polymorphic) + PgSearchHistoryRepository (bulk pseudonymize) + PgAnalysisReportRepository (OCC + target_pnus[]) + PgNotificationRepository (멱등 mark_read) + 22 통합 테스트 | ✅ |
| **4-ii** | V-World 외부 API + Circuit Breaker | `crates/circuit-breaker` (Policy + 3-state Breaker + execute) + `crates/data-clients/vworld` (Client + ParcelReader + ACL parser + RawCapture) + 23 단위 + 6 wiremock 통합 | ✅ |
| **FU 34** | 잠복 lint 부채 일괄 정리 + CI 강화 | shared-kernel/user-domain/listing-domain/data-pipeline-control/auth/db tests 14건 lint fix + workflow `--all-targets` 추가 | ✅ |
| **4-iii-d** | RawCapture trait 분리 + PgRawCapture (FU 27 closed) | `crates/data-clients/raw-capture` 신규 + 마이그 V003_06 (`parcel_external_data` 테이블) + `PgRawCapture` UPSERT + 3 통합 테스트 | ✅ |
| **4-iii-a** | data.go.kr 건축물대장 + DataGoKrBuildingReader | `crates/data-clients/data-go-kr` 신규 + `Policy::data_go_kr_default` + pnu_split + ACL parser (한글→enum 매핑) + V-World geom 합성 + 25 단위 + 6 wiremock 통합 | ✅ |
| **FU-i** | 누적 spec 부채 일괄 정리 (6 FU) | T1 (FU 12 prefix `lph_` + FU 13 spec mock SQL ↔ schema + FU 17 trait rustdoc, ac4036a) / T2 (FU 18 already closed by FU 34) / T3 (FU 26 clippy.toml `disallowed-types reqwest::Client`, 30515ae) / T4 (FU 41 Cd primary 매핑 + endpoint URL drift fix + 숫자/문자열 dual 파싱 + 5 fixture + 25 신규 테스트, bae883c) | ✅ |
| **7-iii** | 정부 API drift 자동 검출 시스템 | crates/operations/api-health (도메인) + crates/db/src/api_health.rs (PgImpl, 8 통합 테스트) + 2 smoke test crate (data.go.kr + V-World, feature-gated `real-api`) + crates/api-health-recorder (octocrab binary, Issue orchestration) + .github/workflows/api-drift-smoke-test.yml (nightly cron 04:00 KST + workflow_dispatch + simulate_failure) + docs/observability/api-drift-smoke-test.md. FU 45/46 closed. SSS 7기둥 모두 ◎ | ✅ |
| **6-foundation** | Frontend 인프라 (Next.js 16 + shadcn + tokens + i18n + UX) | apps/web (Next.js 16.2 + React 19 + Tailwind 4) + packages/ui (shadcn 6 primitives + Pretendard tokens, swap-able) + packages/api-types (utoipa → TS) + 한국어 helper + error/not-found/loading + ky API client + TanStack Query + proxy skeleton + instrumentation.ts (Sentry 자리) + Vitest + Playwright + @axe-core/playwright (WCAG 2.1 AA) + size-limit (bundle < 200KB) + .github/workflows/frontend.yml. SSS 7기둥 모두 ◎ | ✅ |
| **6-iv** | 매물 등록 (broker mutation 화면) | `Listing::update_editable_fields` + `ListingError::ImmutableState` (8 신규 unit tests) / `services/api/src/http/{mutation_ctx,problem}.rs` (`http_user_action` helper + `from_listing_error`/`from_listing_repo_error` 7 매핑) / POST/PATCH/transitions + photo R2 presigned PUT + `POST /listings/:listing_id/photos/:photo_id/confirm` R2 HEAD verification + confirmed-only photo exposure + authenticated `GET /listings/:listing_id/photos/:photo_id` signed R2 download redirect + `LISTING_PHOTO_R2_*` config SSOT + DELETE photo soft-delete / db integration tests for audit, pending-photo non-exposure, and photo_id detail projection / `/listings/new` 폼 (react-hook-form + zod + ProblemDetails toast) / proxy.ts BROKER_PATHS gate / 10 Vitest cross-field unit tests. PhotoUploader/edit page 는 FU 56. | ✅ |
| **4-iii-e** (1차) | R2 PMTiles Reader foundation | `Policy::r2_default` (8s/retry 1/60s cooldown) + `crates/data-clients/r2-public-data` 신규 lib (R2Client get_object_bytes/json + reqwest+Breaker) + PMTiles v3 magic+version 파서 + R2ParcelReader (architecture wire-up + FU 60 honest failure stub). 12 unit tests. **R2BuildingReader (FU 40 close) / BuildingFootprintSource 추상화 / R2IndustrialComplexReader / PMTiles tile_at + MVT decode 는 FU 60 후속** (ETL 빌더 + production fixture 필요). [ADR-0014 보류 — base layer 자체 SSS 부적합](../adr/0014-base-layer-defer-pmtiles.md). | 🟡 (foundation only) |
| **6-iii** | 매물 상세 + 북마크 | `ListingRepository::find_detail_by_id` + `increment_view_count` + `is_bookmarked` JOIN. `Listing.bookmark_count` denormalized 필드 deprecated -- bookmark_listing JOIN COUNT 가 응답 SSOT (FU 70 schema 제거 예정). GET /listings/:id (RBAC: 비공개 상태는 owner only -> 404 leak 차단) + POST/DELETE /listings/:id/bookmark (멱등) + GET /me/bookmarks. Frontend `/listings/[id]` server component + ListingDetail / BookmarkButton (optimistic toggle). 6 db integration test. ADR-0015 V-World ACL 재설계 직전 commit 묶음. | ✅ |
| **6-v** | 알림 (Notifications) | `NotificationKind` 도메인 enum (4 variants + Other forward-compat). PgImpl enum round-trip + `mark_all_read_by_kind` 시그니처 변경. `auth::role_guard::require_one_of_roles` (OR 매칭). admin endpoints (`POST /admin/listings/:id/{approve,reject}` -- Admin/Operator + reason) + `Notification` trigger (best-effort multi-tx). bookmark trigger (owner != bookmarker -> ListingBookmarked 알림). `/me/notifications` 4 endpoints (list / unread-count / read 멱등 / mark-all-read by kind). Frontend `/me/notifications` 페이지 + NotificationBell 헤더 badge (1분 polling). 9 DB integration test (enum round-trip / Other fallback / user isolation / bulk filter). | ✅ |
| **Obs** (1차) | Production 관측성 + Audit chain hardening | SSS § 2/3/4/5 동시 close. **T2** X-Request-Id middleware (Axum + Next.js proxy) -- request_id_layer outermost, span 자동 attach, 응답 echo, ASCII 영숫자 64자 sanitize. **T3** `MutationContextBuilder` extractor -- auth + correlation + ip(XFF) + ua(≤500) 자동, handler 잊을 수 없음. **T4** PgRepository before_state capture (5 repos: User/Listing/ListingPhoto/Bookmark/AnalysisReport) -- `to_jsonb(t.*)` PostGIS-aware (`ST_AsGeoJSON`) + `__metadata__` nesting (FU 90 별도 컬럼). **T5** Sentry init Rust backend (env-driven, silent disabled, panic+error capture, release+env tagging, 10% sample). **T7** Health check liveness/readiness/db (K8s/ECS 분리). T6 (OTLP/Prometheus) 와 frontend Sentry 는 SP-Obs-2 (SP8 IaC 인프라 후). | ✅ (1차) |

**누적**: 34 Rust crate + JS workspace (apps/web + packages/ui + packages/api-types), ~1340 Rust tests (1198 단위 + 142 통합) + 17 frontend unit (Vitest) + 3 e2e (Playwright + axe), 5 CI workflow 그린 (frontend 추가), CI clippy `--all-targets` 강화.

**SP5 시리즈 완전 종료**: 13 BC 모두 동일 transactional `save(agg, ctx)` 또는 `insert(agg, ctx)` 패턴. 9 BC (Core+Audit+Pipeline+Operations) 의 SP5-iv 완성에 더해 4 BC (Insights — Bookmark/SearchHistory/AnalysisReport/Notification) 도 정합.

**SSS read side 완성**: outbox 약속의 read side 도 채워짐 — Aggregate save → audit_log + outbox_event INSERT (write) → publisher tick → Sink (read) 의 chain 이 양쪽 모두 작동.

---

## 다음 sub-project (사용자 결정)

### A. Platform Core 경계 안정화

**목표**: Gongzzang은 Platform Core Catalog/Workforce의 published contract만
소비하고, Catalog source ingestion/raw lineage/public spatial lifecycle은 다시
공짱 작업으로 돌아오지 않게 한다.

**작업**:
- Platform Core boundary, dependency, Catalog API, event receiver gate를 계속 CI
  필수 경로로 유지
- Platform Core pin 파일 변경 시 local consumer adapter만 갱신
- legacy ETL/raw/API health schema drop migration은 사용자 승인 후 별도 작업
- active docs가 Catalog source client, raw capture, public data reader 작업을
  다시 추천하지 않도록 section guard 유지

### B. Gongzzang-owned product work

**목표**: Listing, listing photo, bookmark, notification, broker/admin UX,
listing marker serving처럼 공짱이 영구 owner인 영역만 다음 제품 작업으로 분해한다.

**작업 후보**:
- listing marker serving hardening과 frontend PBF 전환 검증
- listing edit/photo upload UX
- broker/admin review workflow UX
- auth user provisioning hardening

### C. 잔여 FU 일괄 정리

남은 FU는 Gongzzang-owned 여부를 먼저 판정한다. Platform Core Catalog 입력,
raw lineage, public/reference spatial lifecycle에 속하는 FU는 이 repo에서 구현하지
않고 Platform Core issue/ADR로 넘긴다.

---

### SP6 시리즈 (Frontend)

- ✅ SP6-foundation: 인프라 (2026-05-05) — Next.js 16 + shadcn + tokens + i18n + UX + ky/TanStack Query + Vitest/Playwright/axe/size-limit + frontend CI workflow
- ✅ SP6-i: auth flow (Zitadel OIDC + Redis session + cookie + audit + V004 + e2e) — 2026-05-05
- ✅ SP6-ii: 매물 검색 + Naver Maps + ListingCard + FilterBar + Pretendard self-host + e2e — 2026-05-05~06
- 미착수 SP6-iii: 매물 상세 + 북마크 (1-2일)
- ✅ SP6-iv: 매물 등록 (broker POST/PATCH/submit/revise/photos backend + /listings/new 폼 + 10 Vitest + 4 DB integration test) — 2026-05-06
- ✅ SP6-iii: 매물 상세 + 북마크 (find_detail_by_id JOIN COUNT + increment_view_count + bookmark CRUD + /listings/[id] + BookmarkButton + 6 DB integration test) — 2026-05-06
- ✅ SP6-v: 알림 (NotificationKind enum + admin approve/reject + bookmark trigger + /me/notifications 4 endpoints + 헤더 종 badge + 9 DB integration test) — 2026-05-06
- 미착수 FU 56: SP6-iv 후속 — `/listings/[id]/edit` PATCH 화면 + PhotoUploader (R2 통합 후) + e2e 3 (broker 등록 / non-broker 차단 / 폼 cross-field)
- 미착수 FU 70: `listing.bookmark_count` schema 컬럼 제거 (deprecated 후 마이그)
- 미착수 FU 71: 외부 (Parcel/Mfr/IC/CourtAuction) 북마크 UI
- 미착수 FU 72: ListingCard inline bookmark toggle
- 미착수 FU 73: `/me/bookmarks` 프론트엔드 페이지
- 미착수 FU 74: 사진 carousel + lightbox (a11y)
- 미착수 FU 75: `contact_visibility` 별 broker contact 표시 분기
- 미착수 FU 76: bookmark 알림 (SP6-v 묶음)

---

### SP7 시리즈 (관측성)

- ✅ SP7-iii: 정부 API drift 자동 검출 (2026-05-05) — `crates/operations/api-health`, `crates/api-health-recorder`, `.github/workflows/api-drift-smoke-test.yml`
- 미착수 SP7-i: Sentry — 에러 자동 추적 (services/api 통합, 1-2일) — production code panic / breaker open 알림
- 미착수 SP7-ii: Grafana metrics + Outbox publisher metrics (2-3일) — `api_health_check` 시계열 + Outbox lag

---

## 추천 순서

```text
Platform Core boundary verification
  ↓ 경계/의존성/계약/pin 가드 유지
DB cleanup approval
  ↓ legacy ETL/raw/API health schema drop migration은 승인 후 별도
Gongzzang-owned product work
  ↓ listing marker serving, listing edit/photo UX, broker/admin UX
Observability and IaC
  ↓ Gongzzang runtime metrics, Sentry, Pulumi
SP8 (IaC — Pulumi)
```

---

## Spec FU 누적 (production 배포 전 처리)

### 사전 발견 (SP1-SP3 잔재)
- FU 4: BusinessNumber NTS 체크섬 외부 검증 (실제 사업자번호 표본)
- FU 6: BusinessNumber D₃D₄ 사업자 유형 코드 검증
- FU 8: KsicCode 대분류 letter A-U 강제 (KSIC 11차 추적)
- FU 9: ✅ 해소됨 (analysis_report.updated_at, V003_04)
- FU 10: ✅ 해소됨 (outbox_event prefix `evt`)
- FU 11: ✅ 해소됨 (featured_content prefix `fea`)
- ~~FU 12: listing_photo prefix `ph_` (spec) ↔ `lph_` (code) 일관화~~ → ✅ closed by SP-FU-i T1 (`ac4036a`)

### SP5-iii 새 발견
- ~~**FU 13**: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정렬~~ → ✅ closed by SP-FU-i T1 (`ac4036a`)
- **FU 14**: BVQ/LRQ entity 의 `updated_at` 필드 ↔ DB 컬럼 미존재. PgImpl 가 `reviewed_at.unwrap_or(submitted_at)` 으로 합성. 추가 마이그 또는 entity 정리 필요
- **FU 15**: `Repository.save(aggregate, ctx)` OCC API 가 caller 의 read-시점 version 을 묵시 의존. `expected_version` 명시 인자 추가가 더 명확 (도메인 메서드가 `version += 1` 하므로)
- **FU 16**: LRQ `find_by_listing` 의 silent shadow 위험 — `UNIQUE INDEX listing_review_queue(listing_id) WHERE decision IS NULL` 추가 검토
- ~~**FU 17**: Trait doc stale (AuditLog/OperationsMeta)~~ → ✅ closed by SP-FU-i T1 (`ac4036a`)
- ~~**FU 18**: AuthCrate clippy 빚~~ → ✅ already closed by FU 34 (`9f0533a`), confirmed by SP-FU-i T2

### SP-FU-i T4 발견 (2026-05-04)
- ~~**FU 45 (제안)**: 정부 API endpoint URL drift staging-only smoke test~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- ~~**FU 46 (제안)**: 정부 API JSON Number vs String schema drift 모니터링~~ → ✅ closed by SP7-iii (`<T6 commit>`)
- **FU 47 (제안)**: V-World 지오코딩 통합 (주소 → PNU) — dev session 가속

### Production 인프라
- AuditLog full diff capture (`before_state` + `after_state`) — current SP5-iii 는 `before_state = NULL`
- AuditLog `ip_address` / `user_agent` 자동 수집 (Axum middleware 통합) → SP7 관측성과 연관
- Outbox publisher worker 구현 → SP4 또는 별도
- 진짜 Zitadel staging 통합 테스트 (`docs/auth/staging-zitadel-integration.md` 참조)
- Repo private 전환 (production 운영 단계 직전)

---

## 환경 메모

- **로컬 cargo 작동** (MSVC Build Tools 설치 완료, 2026-05-03)
- **Repo public** (`w1kch9812-cmd/test`) — GH Actions 무료
- **CI 5 workflow**: CI (7 jobs) / db-migrations / walking-skeleton (mock JWT mode + integration tests + DB reset) / api-drift-smoke-test (nightly cron 04:00 KST + workflow_dispatch) / frontend (lint/typecheck/test/build/bundle/e2e+a11y, paths-filtered)
- **마지막 commit**: `<T4 commit>` (SP6-foundation T4 — smoke + Playwright + axe + size-limit + frontend CI + docs + roadmap)
- **다음 commit 시 항상**: 본 문서 갱신 → SP 진행 상태 SSOT 유지
> Current status (2026-05-28): This roadmap contains historical sub-project
> entries. Catalog/data-source/ETL/API-drift rows that mention local
> `crates/data-clients/*`, `raw-capture`, `api-health`, or SP9 base-layer assets
> are superseded by ADR 0034 and Platform Core M3.2 physical extraction. The
> current ownership SSOT is `docs/architecture/platform-core-boundary.v1.json`.
