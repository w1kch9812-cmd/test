# 공짱 Sub-project Roadmap

> **갱신일**: 2026-05-04 (SP5-iii 종료 직후)
> **현재 main**: `215826a`
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

**누적**: 25 crate, ~1120 tests (1058 단위 + 62 통합), 3 CI workflow 그린.

---

## 다음 sub-project (사용자 결정)

### A. SP5-iv — SP5-i refactor (가장 작음, 가시적)

**목표**: SP5-i 의 `User`/`Listing`/`ListingPhoto` PgRepository `save()` 에 `MutationContext` 추가 — SSS 약속 ("모든 mutation audit") 완전 닫음.

**작업**:
1. 3 도메인 trait 시그니처 변경: `save(aggregate, ctx)` 형태로
2. PgUserRepository / PgListingRepository / PgListingPhotoRepository 의 `save()` 가 SP5-iii 의 transactional 패턴 따름 (audit_log + outbox INSERT 같은 tx)
3. `services/api` 의 AuthMiddleware first-sign-in 자동 생성 시 `MutationContext::new_system_action(...)` 구성
4. 통합 테스트 갱신 (audit_log row 검증 추가)

**추정**: 5-8 task, 1-2일.
**Spec status**: 미작성 — brainstorming 필요.

### B. SP5-ii — Insights BC RDS Repository (패턴 반복)

**목표**: Bookmark / SearchHistory / AnalysisReport / Notification 4 repo 의 PgImpl.

**작업**: SP5-i + SP5-iii 패턴 답습. 새 도메인 4개. Bookmark 는 2 aggregate (BookmarkListing + BookmarkExternal).

**추정**: 8-10 task, 2-3일.
**Spec status**: 미작성.

### C. SP4 — 외부 API ingestion + R2 Reader + Outbox publisher worker (가장 큼)

**목표**: V-World / data.go.kr / 법제처 API 통합 + 6 R2 Reader 구현체 + Outbox row 를 외부로 발행하는 워커.

**작업**: 새 기술 다수 — Circuit Breaker, retry, raw_response JSONB 보존, R2 PMTiles 파싱, AWS S3 client (R2), Outbox publisher 워커.

**추정**: 20-30 task, 4-7일. 사실상 sub-sub-project 분해 필요 (SP4-i / SP4-ii / ...).
**Spec status**: 미작성, 분해 결정 필요.

---

## 추천 순서

```
A (SP5-iv, 1-2일)
  ↓ "모든 mutation audit" 진짜 완성
B (SP5-ii, 2-3일)
  ↓ RDS Repo 패턴 마무리 (SP5 시리즈 종료)
C (SP4 분해, 4-7일)
  ↓ 실제 데이터 흐름 시작
SP6 (Frontend Next.js)
SP7 (관측성 — Grafana / Tempo / Sentry)
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
- **마지막 commit**: `215826a` (SP5-iii 종료)
- **다음 commit 시 항상**: 본 문서 갱신 → SP 진행 상태 SSOT 유지
