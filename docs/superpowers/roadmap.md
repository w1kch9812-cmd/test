# 공짱 Sub-project Roadmap

> **갱신일**: 2026-05-04 (SP4-iii-a 종료 직후)
> **현재 main**: `c210801` (SP4-iii-a T5) 후 SSOT 갱신 commit
> **SSOT**: 본 문서 — 다음 sub-project 결정/진행 시 *먼저* 갱신.

---

## 완료 (2026-05-04 기준)

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

**누적**: 31 crate, ~1232 tests (1130 단위 + 102 통합), 3 CI workflow 그린, CI clippy `--all-targets` 강화.

**SP5 시리즈 완전 종료**: 13 BC 모두 동일 transactional `save(agg, ctx)` 또는 `insert(agg, ctx)` 패턴. 9 BC (Core+Audit+Pipeline+Operations) 의 SP5-iv 완성에 더해 4 BC (Insights — Bookmark/SearchHistory/AnalysisReport/Notification) 도 정합.

**SSS read side 완성**: outbox 약속의 read side 도 채워짐 — Aggregate save → audit_log + outbox_event INSERT (write) → publisher tick → Sink (read) 의 chain 이 양쪽 모두 작동.

---

## 다음 sub-project (사용자 결정)

### A. SP4-iii — data.go.kr + 법제처 + R2 Reader 6 (남은 분해)

**목표**: SP4-iii-d (RawCapture infra) + SP4-iii-a (data.go.kr 건축물대장)
완료. 남은 sub-task:

| Sub | 영역 | 상태 |
|---|---|---|
| 4-iii-d | RawCapture trait + PgRawCapture | ✅ (2026-05-04) |
| 4-iii-a | data.go.kr 건축물대장 + DataGoKrBuildingReader | ✅ (2026-05-04) |
| 4-iii-b | data.go.kr 실거래가 + RealTransactionReader | 미착수 (1-2일) |
| 4-iii-c | 법제처 (도시계획 텍스트) | 미착수 (1-2일) |
| 4-iii-e | R2 PMTiles Reader 6 (Parcel bbox markers, Building footprint, IC, Mfr, ...) + FU 40 | 미착수 (2-3일) |

**미해소 follow-up (SP4-ii 잔여)**:
- FU 26: `clippy::disallowed_types` reqwest::Client 직접 호출 차단
- FU 28: Redis 캐시 (TTL 24h)
- FU 29: Sentry alert on Breaker open
- FU 30: `fetch_markers_in_bbox` PMTiles 또는 V-World BBOX WFS (SP4-iii-e)

**SP4-iii-a 발견 follow-up**:
- FU 40: `Building.geom` 정확한 footprint (V-World AL_D194 또는 R2 PMTiles)
- FU 41: 한글 라벨 매핑표 확장 (28+ 케이스)
- FU 42: `BuildingReader::fetch_by_id` (mgmBldrgstPk endpoint)
- FU 43: 캐시 정책 (`expires_at = fetched_at + 30 days`)
- FU 44: 토지대장 endpoint

### B. FU 일괄 정리 (작은 빚 닫기)

FU 4/6/8/12/13/14/15/16/17/18/26/27/28/29/30/31/32/33/34 — production 전 필수. 특히 FU 34 (workspace `--all-targets` clippy 가 잡는 기존 부채: shared-kernel `float_cmp`, user-domain `redundant_clone`, `redundant_closure_for_method_calls` 등) 는 SP4-iii 시작 전 권장.

### C. SP6 — Frontend (Next.js + React 19)

**목표**: 인증/매물/북마크/알림 핸들러가 SP5-* 의 PgRepository 를 사용하는 첫 사용자 화면.

**작업**: Next.js 16 + React 19 + Naver Maps SDK + Zitadel OIDC 클라이언트. `services/api` 의 핸들러 추가 + `apps/web` 화면. `MutationContext::new_user_action(...)` helper (`services/api`) 도입.

**추정**: 분해 필요 (SP6-i 인증 / SP6-ii 매물 검색 / SP6-iii 북마크 / SP6-iv 알림 등 4-7일).
**Spec status**: 미작성.

### C. 누적 FU 일괄 정리 (작은 빚 닫기)

FU 4/6/8/12/13/14/15/16/17/18 — 9건 미해소. production 전 필수.

---

## 추천 순서

```
B (FU 일괄, 0.5-1일)
  ↓ 기존 부채 정리 (특히 FU 34: --all-targets 강화)
A (SP4-iii 분해, 3-5일)
  ↓ 나머지 외부 API + R2 Reader 6 + raw_response DB 저장
C (SP6 분해, 4-7일)
  ↓ 첫 사용자 화면 — SP5-* 의 모든 Repository 가 활용됨
SP7 (관측성 — Grafana / Tempo / Sentry — Outbox publisher metrics + circuit breaker open alert)
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
- FU 12 (제안): listing_photo prefix `ph_` (spec) ↔ `lph_` (code) 일관화

### SP5-iii 새 발견
- **FU 13**: AuditLog spec § 4.3 mock SQL ↔ 실제 schema 정렬 (`metadata` → `before_state`/`after_state`/`ip_address`/`created_at`) — plan 에서 정정해 따랐으나 spec 문서 자체 갱신 필요
- **FU 14**: BVQ/LRQ entity 의 `updated_at` 필드 ↔ DB 컬럼 미존재. PgImpl 가 `reviewed_at.unwrap_or(submitted_at)` 으로 합성. 추가 마이그 또는 entity 정리 필요
- **FU 15**: `Repository.save(aggregate, ctx)` OCC API 가 caller 의 read-시점 version 을 묵시 의존. `expected_version` 명시 인자 추가가 더 명확 (도메인 메서드가 `version += 1` 하므로)
- **FU 16**: LRQ `find_by_listing` 의 silent shadow 위험 — `UNIQUE INDEX listing_review_queue(listing_id) WHERE decision IS NULL` 추가 검토
- **FU 17**: Trait doc stale 다수
  - `AuditLogRepository::find_by_resource(limit)` `find_by_actor(since)` — spec 보다 실제 trait 가 풍부
  - `OperationsMetaRepository::find_unacknowledged_alerts` — doc 은 "created_at ASC" 이나 spec/impl/index 는 "severity DESC + created_at DESC"
  - 정리: 코드가 SSOT, spec 갱신 필요
- **FU 18**: AuthCrate clippy 빚 — `crates/auth/src/verifier.rs` 의 pre-existing `clippy::panic` + `clippy::manual_let_else` (SP3 잔재). `cargo clippy --all-targets` 만 잡고 CI 의 `--all-features` 만 으로는 통과. 정리 필요

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
- **CI 3 workflow**: CI (7 jobs) / db-migrations / walking-skeleton (mock JWT mode + integration tests + DB reset)
- **마지막 commit**: `c210801` (SP4-iii-a T5 — wiremock integration tests) +
  로컬 SSOT 갱신 commit (push 대기)
- **다음 commit 시 항상**: 본 문서 갱신 → SP 진행 상태 SSOT 유지
