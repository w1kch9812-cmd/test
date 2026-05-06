# Sub-project 6-v: 알림 (Notifications) — 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Approved |
| 선행 spec | [`2026-05-06-sub-project-6-v-notifications-design.md`](../specs/2026-05-06-sub-project-6-v-notifications-design.md) |
| 추정 | 8 task, 1.5-2일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp6-v): spec + plan -- 알림 (NotificationKind enum + admin approve/reject + bookmark trigger + /me/notifications 4 endpoint)`

---

## T2 — `NotificationKind` 도메인 enum + Notification 시그니처 변경

**대상**: `crates/domain/insights/notification/src/{kind.rs,entity.rs,errors.rs,lib.rs}`

- `kind.rs` (신규): `NotificationKind` enum (4 variants: ListingApproved /
  ListingRejected / ListingBookmarked / Other), `as_str` / `from_str` /
  `Display` impl
- `entity.rs`: `Notification.kind: String` → `NotificationKind`. `try_new`
  시그니처 갱신 (이전 `kind: &str` → `kind: NotificationKind`)
- 기존 단위 테스트 갱신 (`String "bookmark_listing_changed"` →
  `NotificationKind::ListingBookmarked` 등)
- `lib.rs` re-export
- 8 신규 단위 테스트 (variant as_str / from_str round-trip / serde / Other fallback)

**commit**: `feat(sp6-v-t2): NotificationKind enum (type-safe + Other forward-compat)`

---

## T3 — `PgNotificationRepository` enum round-trip + `require_one_of_roles` helper

**대상**: `crates/db/src/notification.rs` + `crates/auth/src/role_guard.rs`

- `PgNotificationRepository` 4 메서드 모두 `kind` enum ↔ varchar(50) 변환
  (write 시 `as_str()`, read 시 `from_str` Other fallback)
- `crates/db/src/error_map.rs` notification 매핑 갱신 (영향 없을 듯)
- 기존 통합 테스트 (`crates/db/tests/notification_integration.rs`) 갱신 — kind
  파라미터 enum 으로
- `auth::role_guard::require_one_of_roles(auth, &[UserRole::Admin, ...])` 신규 +
  단위 테스트 4

**commit**: `feat(sp6-v-t3): PgNotificationRepository enum round-trip + require_one_of_roles`

---

## T4 — admin approve/reject endpoints + listing_approved/rejected notification

**대상**: `services/api/src/routes/admin_listings.rs` (신규) + `main.rs`

- `AdminListingsState { listing_repo, notification_repo }`
- `POST /admin/listings/:id/approve`:
  1. `require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator])`
  2. `listing_repo.find` → `Listing::approve(now)` (도메인 PendingReview 가드)
  3. `listing_repo.save(listing, http_user_action(approve_listing))` — audit + outbox
  4. `Notification::try_new(... ListingApproved ...)` →
     `notification_repo.insert` — best-effort, 실패 시 warn
  5. response: `{ id, version, status }`
- `POST /admin/listings/:id/reject`:
  - body `{ reason: String }` (≤500자)
  - `Listing::reject(now)` + notification `ListingRejected` (payload.reason)
- 라우터 등록 (admin scope group)

**commit**: `feat(sp6-v-t4): admin approve/reject + listing_approved/rejected notification trigger`

---

## T5 — bookmark received notification + `/me/notifications` 4 endpoints

**대상**:

- `services/api/src/routes/bookmarks.rs` 갱신 — `BookmarksState` 에
  `listing_repo` + `notification_repo` 추가. `toggle_bookmark` handler 가
  bookmark save 후 listing.owner != bookmarker 면 notification INSERT (best-effort)
- `services/api/src/routes/notifications.rs` (신규):
  - `GET /me/notifications?unread_only=&limit=`
  - `GET /me/notifications/unread-count`
  - `PATCH /me/notifications/:id/read`
  - `POST /me/notifications/mark-all-read?kind=`
- `NotificationsState`, `NotificationResponse` (kind str + payload + read_at)
- `from_notification_repo_error` ProblemDetails helper
- 라우터 등록

**commit**: `feat(sp6-v-t5): bookmark trigger + /me/notifications 4 endpoints`

---

## T6 — backend 통합 테스트

**대상**: `crates/db/tests/notification_integration.rs` 보강 + `services/api/tests/...` 또는 인접 db 통합

11 시나리오:

- T4: approve_listing_admin_inserts_listing_approved_notification
- T4: reject_listing_admin_inserts_listing_rejected_notification (reason 포함)
- T4: approve_non_admin_returns_403
- T4: approve_non_pending_returns_409 (도메인 가드)
- T5: bookmark_own_listing_does_not_insert_notification
- T5: bookmark_other_listing_inserts_notification
- T5: get_my_notifications_unread_only_filters
- T5: get_unread_count_returns_zero_after_mark_read
- T5: mark_read_idempotent_on_already_read (이미 있음 — 갱신 X)
- T5: mark_all_read_by_kind_filters_correctly
- T5: notifications_user_isolation (A user / B user 격리)

**commit**: `test(sp6-v-t6): 11 integration tests (admin approve/reject + bookmark trigger + /me/notifications)`

---

## T7 — Frontend 알림 페이지 + 헤더 종 badge

**대상**: `apps/web/`

- `app/(authenticated)/me/notifications/page.tsx` — server component, 초기
  fetch
- `components/notifications/notification-list.tsx` — read 상태 별 그룹 + scroll
- `components/notifications/notification-card.tsx` — kind 별 메시지 템플릿
  (한국어) + 라우트 (e.g. listing_approved → `/listings/:id`)
- `components/notifications/notification-bell.tsx` — 헤더 종 + badge
  (TanStack Query polling 1분 interval)
- `lib/notifications/api.ts` — schemas + fetch + mutations
- `lib/notifications/use-unread-count.ts` — TanStack Query hook
- `lib/i18n/messages/notifications.ko.json` (신규)
- 헤더 layout 에 NotificationBell 통합
- Vitest 4: kind label mapping / unread count hook / mark-read mutation /
  navigation route
- Playwright e2e 1: 알림 발생 (test seed) → 종 badge 1 → 페이지 → 읽음 →
  badge 0 round-trip

**commit**: `feat(sp6-v-t7): /me/notifications 페이지 + 헤더 종 badge + e2e`

---

## T8 — workspace 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- 로컬 `pnpm -F web run typecheck test build` 그린
- push → 5 CI workflow 그린
- SSOT 갱신 (roadmap SP6-v ✅, project_progress 본문 추가, FU 76/80-87 추가)

**commit**: `docs(sp6-v-t8): SP6-v 종료 -- 알림 (NotificationKind enum + 3 trigger + 4 endpoints)`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| 도메인 | `crates/domain/insights/notification/src/{kind,entity,errors,lib}.rs` | enum + try_new + tests |
| auth | `crates/auth/src/role_guard.rs` | require_one_of_roles |
| PgImpl | `crates/db/src/notification.rs` | enum round-trip |
| Routes | `services/api/src/routes/{admin_listings,notifications,bookmarks}.rs` | 신규 + 확장 |
| Routes wiring | `services/api/src/main.rs` | admin + notifications + bookmarks state |
| 통합 테스트 | `crates/db/tests/notification_integration.rs` | 11 시나리오 |
| Frontend pages | `apps/web/app/(authenticated)/me/notifications/page.tsx` | 신규 |
| Frontend components | `apps/web/components/notifications/{notification-list,notification-card,notification-bell}.tsx` | 신규 |
| Frontend lib | `apps/web/lib/notifications/{api,mutations,use-unread-count}.ts` | 신규 |
| Frontend i18n | `apps/web/lib/i18n/messages/notifications.ko.json` | 신규 |
| Frontend tests | Vitest 4 + Playwright 1 e2e | 신규 |
| docs | spec + plan + roadmap + project_progress + MEMORY | 신규/갱신 |

총 ~30 파일.

---

## 위험 요소

- **`Notification.kind` 시그니처 변경**: SP5-ii 의 기존 통합 테스트 + 도메인
  테스트 영향. 모두 enum 으로 갱신 필요
- **multi-tx best-effort 알림**: spec § 5 명시. SP7 관측성에서 alert 룰 추가
- **frontend polling 1분**: 트래픽 ↑ 시 Cloudflare 캐시 layer + 5분 interval
  로 evolve
- **bookmark notification dedup 부재**: 같은 사용자가 같은 매물 N회 북마크
  toggle 시 N notifications. 1차 = 단순. FU 85 가 dedup
- **admin endpoint 가 LRQ workflow 와 분리**: 정식 reviewer assignment 는 BVQ/
  LRQ Operations BC 가 책임 — 본 SP 의 admin endpoint 는 *가벼운* approve/reject
  (FU 90 이 정식 워크플로우)
