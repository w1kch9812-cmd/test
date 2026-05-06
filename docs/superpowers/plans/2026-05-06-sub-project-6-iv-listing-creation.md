# Sub-project 6-iv: Broker 매물 등록 — 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Approved |
| 선행 spec | [`2026-05-06-sub-project-6-iv-listing-creation-design.md`](../specs/2026-05-06-sub-project-6-iv-listing-creation-design.md) |
| 추정 | 8 task, 2-3일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp6-iv): spec + plan -- broker 매물 등록 (POST/PATCH + photo presigned + RBAC)`

---

## T2 — `Listing::update_editable_fields` + `ImmutableState` error

**대상**: `crates/domain/core/listing/src/{entity.rs,errors.rs}`

- `ListingError::ImmutableState { current: ListingStatus }` 추가
- `ListingUpdate` 구조체 + `update_editable_fields` 메서드
- 단위 테스트 5+:
  - happy path Draft 수정 → version bump
  - Rejected 수정 OK
  - Active 수정 → ImmutableState
  - transaction_type 동일 유지 + deposit/monthly_rent 변경 → cross-field 검증
  - title 만 변경 → 다른 필드 보존

**commit**: `feat(sp6-iv-t2): Listing::update_editable_fields + ImmutableState error`

---

## T3 — `services/api/src/http/mutation_ctx.rs` + ProblemDetails 매핑 확장

**대상**: `services/api/src/http/{mutation_ctx.rs,problem.rs}`

- `mutation_ctx.rs` 신규: `http_user_action(auth, action, correlation_id) -> MutationContext`
- `problem.rs` 확장: `from_listing_error`, `from_repo_error` helper (또는 `From<...>` impl). 매핑표는 spec § 3.7
- `correlation_id` 자동 ULID 생성 helper

**commit**: `feat(sp6-iv-t3): http_user_action helper + ProblemDetails domain mapping`

---

## T4 — `POST /listings` + `PATCH /listings/:id` 핸들러

**대상**: `services/api/src/routes/listings.rs` (확장)

- `CreateListingRequest` (zod-equivalent struct + validate)
- `create_listing` handler:
  1. `require_role(&auth, UserRole::Broker)`
  2. body → 도메인 값 객체 (`ListingTitle::try_new` 등) 변환
  3. `Listing::try_new_draft(...)` (`actor.user.id` = `owner_id`)
  4. `repo.save(&listing, http_user_action(&auth, "create_listing", &cor_id))`
  5. `201 Created` with `{ id, version }`
- `UpdateListingRequest` + `patch_listing` handler:
  1. RBAC + ownership check (`current.owner_id != auth.user.id` → 403)
  2. `if-match` header parse → version 비교
  3. `listing.update_editable_fields(update, now)`
  4. `repo.save(&listing, ctx)` — DB OCC 가 다시 검증
  5. `200 OK` with `{ id, version }`
- 라우터에 등록 — `main.rs` 의 `listings_router` 갱신

**commit**: `feat(sp6-iv-t4): POST /listings + PATCH /listings/:id with RBAC + OCC if-match`

---

## T5 — 상태 전이 + 사진 presigned URL endpoint

**대상**: `services/api/src/routes/listings.rs` 추가 핸들러

- `submit_for_review_handler` — `POST /listings/:id/submit-for-review`
- `revise_handler` — `POST /listings/:id/revise` (Rejected → Draft)
- `request_photo_upload` — `POST /listings/:id/photos`
  - `ListingPhoto::try_new` → `repo.save(&photo, ctx)`
  - response: `{ photo_id, presigned_put_url: "MOCK://...", expires_at }`
  - tracing log target = `photo.upload.mock`
- `delete_photo` — `DELETE /listings/:id/photos/:photo_id` (soft-delete)
- 모든 핸들러 ownership check
- 라우터 등록

**commit**: `feat(sp6-iv-t5): submit/revise transitions + photo presigned URL (mock) + soft-delete`

---

## T6 — backend 통합 테스트 6 시나리오

**대상**: `services/api/tests/listings_creation_integration.rs` (또는 `crates/db/tests/...`)

- `create_listing_happy_path` — broker 로 생성 → DB 검증 + audit_log row + outbox row
- `create_listing_non_broker_returns_403` — buyer 로 시도 → 403
- `patch_listing_stale_version_returns_409` — `if-match: 1` 으로 보내고 DB 가 v2 → 409
- `patch_listing_cross_tenant_returns_403` — 다른 broker 가 수정 시도 → 403
- `submit_for_review_happy_path` — Draft → PendingReview, audit_log row
- `request_photo_upload_returns_mock_presigned_url` — response shape 검증

**commit**: `feat(sp6-iv-t6): listings_creation integration tests (6 scenarios)`

---

## T7 — Frontend `/listings/new` + `/listings/:id/edit`

**대상**: `apps/web/`

- `app/(authenticated)/listings/new/page.tsx`
- `app/(authenticated)/listings/[id]/edit/page.tsx`
- `components/listings/listing-form.tsx` (react-hook-form + zod resolver)
- `components/listings/photo-uploader.tsx` (drag-drop + progress)
- `lib/listings/mutations.ts` (TanStack Query mutations)
- `lib/listings/schema.ts` 확장 (createListingSchema + cross-field refine)
- `lib/i18n/messages/listings.ko.json` 확장 (form labels + errors)
- `proxy.ts` — `/listings/new` + `/listings/:id/edit` 진입 시 `Broker` 가드 (이미 있는 ADMIN_ROLES 패턴 재사용 또는 별도)
- 5 단위 테스트 (Vitest):
  - schema cross-field refine (sale / jeonse / monthly_rent)
  - form rendering
  - photo uploader optimistic state
- 3 e2e (Playwright + axe):
  - broker 로그인 → 등록 → /listings 에서 새 매물 보임
  - non-broker 가 /listings/new 진입 → /forbidden 리다이렉트
  - 폼 cross-field 에러 메시지 한국어로 표시

**commit**: `feat(sp6-iv-t7): /listings/new + /listings/:id/edit forms + photo uploader + e2e`

---

## T8 — 워크스페이스 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- 로컬 `pnpm turbo run lint typecheck test build` 그린
- 로컬 `pnpm turbo run e2e` 그린 (또는 mock mode)
- push → 5 CI workflow 그린 확인
- SSOT 갱신:
  - `docs/superpowers/roadmap.md` SP6-iv ✅
  - `memory/project_progress.md` SP6-iv 본문 추가
  - `docs/superpowers/next-actions.md` SP6-iv 제거 + SP4-iii-e 1순위 승격
  - `MEMORY.md` index 갱신

**commit**: `docs(sp6-iv-t8): SP6-iv 종료 -- broker 매물 등록 (POST/PATCH/submit/revise/photos)`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| 도메인 | `crates/domain/core/listing/src/{entity,errors}.rs` | `update_editable_fields` + `ImmutableState` |
| HTTP helpers | `services/api/src/http/{mutation_ctx,problem}.rs` | 신규 + 확장 |
| Routes | `services/api/src/routes/listings.rs` | +5 endpoint |
| Routes | `services/api/src/main.rs` | router 갱신 |
| 통합 테스트 | `services/api/tests/listings_creation_integration.rs` | 신규 6 |
| Frontend pages | `apps/web/app/(authenticated)/listings/{new,[id]/edit}/page.tsx` | 신규 |
| Frontend components | `apps/web/components/listings/{listing-form,photo-uploader}.tsx` | 신규 |
| Frontend lib | `apps/web/lib/listings/{mutations,schema}.ts` | 신규/확장 |
| Frontend proxy | `apps/web/proxy.ts` | broker 가드 추가 |
| Frontend i18n | `apps/web/lib/i18n/messages/listings.ko.json` | 확장 |
| Frontend tests | Vitest 5 + Playwright 3 e2e | 신규 |
| docs | spec + plan + roadmap + project_progress + next-actions + MEMORY | 신규/갱신 |

총 ~25-30 파일.

---

## 위험 요소

- **R2 미통합**: photo presigned URL 가 mock — 실 업로드 작동 안 함. SP4-iii-e 까지 받아들임. 프로덕트 가치는 등록 + 검토 흐름 자체.
- **`if-match` header parsing**: HTTP/1.1 spec 의 `If-Match` 가 ETag 형태 (`"v123"`). 우리는 단순 정수 — header 가 quote 포함하면 strip. 미준수 시 silent failure 가능.
- **ownership check 위치**: route handler 레벨 — domain 이 강제하지 않음. RBAC 필터는 항상 핸들러 첫줄에. 누락 시 cross-tenant 가능 → 통합 테스트로 강제.
- **frontend zod refine ↔ backend 도메인 invariant 둘**: 같은 룰 두 번. SSOT = 도메인. utoipa 자동 생성 미구축이라 zod 가 manual fork. 차이 발생 시 backend 가 진실 (FU 55 가 자동화).
- **`broker_verified_at` 무시**: 정책 결정 전까지는 `Broker` role 만 있으면 OK. FU 51 후속.
