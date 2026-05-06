# Sub-project 6-v: 알림 (Notifications) — Spec

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 | SP2c-T6 (Notification Aggregate), SP5-ii (PgNotificationRepository), SP6-iv (broker 등록), SP6-iii (북마크) |
| 후속 | FU 76 (외부 북마크 알림 + 매물 만료 알림) |
| 추정 | 8 task, 1.5-2일 |

---

## 1. 개요

알림 인프라 (도메인 + repo + DB) 는 SP2c/SP5-ii 에서 완성됐으나 **trigger 가
없음** — `Notification::try_new` 호출자 0. SP6-v 가 *진짜 알림이 발생하는 시점*
을 wire-up.

**3 trigger 시점**:

1. **Listing approved** — admin/broker 매물 승인 → broker 에게 `listing_approved`
2. **Listing rejected** — admin 가 거부 → broker 에게 `listing_rejected`
3. **Listing bookmarked** — 다른 사용자가 본인 매물 북마크 → broker 에게
   `listing_bookmarked` (FU 76 일부 — bookmark 자동)

**4 endpoint** (`/me/notifications`):

- `GET /me/notifications` — 최근 365일 알림 (unread filter 옵션)
- `GET /me/notifications/unread-count` — unread 카운트 (badge 용)
- `PATCH /me/notifications/:id/read` — 단건 읽음 (멱등)
- `POST /me/notifications/mark-all-read` — 전체 또는 kind 별 일괄

**부수 작업 — admin action endpoints**:

`/admin/listings/:id/approve` + `/admin/listings/:id/reject` 가 *없음*. 알림
trigger 가 의미 있으려면 먼저 mutation endpoint 가 필요. 본 SP 가 함께 닫음.

**SSS 핵심 결정**:

- **`NotificationKind` 도메인 enum 신설** (현재는 `String` 1-50자만 검증). String
  은 typo 위험 + 알 수 없는 kind 처리 분기 모호. enum 분기로 type-safe.
- **알림 INSERT 가 mutation tx 와 같은 commit** — listing approve 시점 한
  transaction 안에 `audit_log + outbox_event + notification` 같이 커밋.
  부분 실패 = 전체 rollback (eventual consistency 위험 차단)
- **mark_read 는 멱등** (이미 SP5-ii 에서 구현 완료)
- **unread count = 별도 endpoint** — list 응답에 포함하면 페이지네이션 수치와
  꼬임 + 캐시 미스 빈도 ↑. 작은 SELECT count(*) 가 깨끗

---

## 2. 범위

### 포함

#### 도메인 — `crates/domain/insights/notification`

- 신규 `NotificationKind` enum:
  - `ListingApproved` (broker 매물 승인 알림)
  - `ListingRejected` (broker 매물 반려 알림)
  - `ListingBookmarked` (broker 본인 매물 북마크 받음)
  - `Other` (forward-compat fallback — 향후 외부 시스템 알림 등)
- `Notification.kind: NotificationKind` (현재 String → enum 변경)
- `Notification::try_new` 시그니처 갱신
- `payload` 는 `serde_json::Value` 그대로 (kind 별 schema 는 FU 55 utoipa)

#### 도메인 — `crates/domain/core/listing` 확장

이미 `Listing::approve` / `reject` 도메인 메서드 존재 (SP2b-i). admin endpoint
가 호출하면서 notification 도 같은 ctx 의 outbox event 로 묶음.

#### Repository — `ListingRepository::save` 가 notification side-effect 트리거?

**아님** — repository 는 도메인 mutation 만. notification INSERT 는 caller (handler)
가 *같은 tx* 안에서 직접 — `MutationContext::events` 에 도메인 이벤트 추가하면
PgListingRepository 가 outbox event 만 INSERT. notification 은 별도 path.

**SSS 답**: handler 가 *2 repository 호출 단일 tx* — 가장 깨끗하지만 axum
extractor pattern 에서는 application service 의 영역. 1차 단순화:

- handler 가 `listing_repo.save(listing, ctx)` (audit + outbox) 후 *별도*
  `notification_repo.insert(notification, notif_ctx)` 호출. 두 tx 분리.
- 둘째 INSERT 실패 시 알림만 누락 (audit 은 살아 있음). 알림 자체가
  *eventually consistent* 라 best-effort 허용 — 운영 시 outbox consumer 가
  notification 도 fallback 으로 발행 가능 (FU 향후).

이 trade-off 는 spec § 5 에 명시. 진짜 단일-tx 답은 SP6-v-2 에서 application
service layer 가 도입될 때.

#### `services/api` — 신규 endpoints

```text
POST   /admin/listings/:id/approve     -- admin or operator role
POST   /admin/listings/:id/reject       -- admin or operator role + reason body
GET    /me/notifications               -- ?unread_only=true|false (default false)
GET    /me/notifications/unread-count
PATCH  /me/notifications/:id/read
POST   /me/notifications/mark-all-read -- ?kind=<NotificationKind> (선택, 없으면 전체)
```

`/listings/:id/bookmark` (POST) 핸들러 *수정* — 본인 매물 아닌 경우 notification
INSERT (best-effort).

#### Frontend — `apps/web`

- `app/(authenticated)/me/notifications/page.tsx` — 알림 목록 페이지
- `components/notifications/notification-list.tsx` — 카드 목록 + read state
- `components/notifications/notification-card.tsx` — kind 별 메시지 템플릿,
  클릭 시 navigate (예: listing approved → /listings/:id)
- `components/notifications/notification-bell.tsx` — 헤더 종 아이콘 + badge
  (unread count). polling 또는 stale-while-revalidate. Server-Sent Events /
  WebSocket 은 FU 80
- `lib/notifications/api.ts` — fetch + mutations
- `lib/notifications/use-unread-count.ts` — TanStack Query polling hook (1분
  interval)
- proxy.ts — `/me/notifications` 진입에 인증 필수 (이미 default protected)
- i18n `notifications.ko.json` 신규

### 미포함

- **WebSocket / SSE realtime push** — FU 80
- **외부 (Parcel/CourtAuction) 알림** — FU 76
- **매물 만료 알림 (expires_at)** — FU 81 (cron worker)
- **이메일 / SMS 알림** — FU 82 (외부 transport)
- **알림 설정 UI (사용자 별 kind opt-out)** — FU 83
- **search history 결과 변동 알림** — FU 84

---

## 3. 컴포넌트

### 3.1 `NotificationKind` enum

```rust
// crates/domain/insights/notification/src/kind.rs (신규)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    ListingApproved,
    ListingRejected,
    ListingBookmarked,
    Other,
}

impl NotificationKind {
    pub const fn as_str(self) -> &'static str { ... }
}

impl FromStr for NotificationKind {
    fn from_str(s: &str) -> Result<Self, NotificationKindError> {
        match s {
            "listing_approved" => Ok(Self::ListingApproved),
            "listing_rejected" => Ok(Self::ListingRejected),
            "listing_bookmarked" => Ok(Self::ListingBookmarked),
            // 미지원 코드 = Other (forward-compat — 새 kind 가 DB 에 들어와도 panic X)
            _ => Ok(Self::Other),
        }
    }
}
```

DB 컬럼은 `varchar(50)` 그대로. 도메인이 enum, DB 가 string. PgNotificationRepository
가 round-trip 시 `as_str` / `from_str` 변환.

### 3.2 admin approve/reject endpoints

```rust
// services/api/src/routes/admin_listings.rs (신규)

pub async fn approve_listing(
    State(state): State<AdminListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<TransitionResponse>, ProblemResponse> {
    // 1. RBAC: Admin or Operator role
    require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator])?;

    // 2. listing find + transition
    let mut listing = state.listing_repo.find(&listing_id).await?
        .ok_or_else(|| problem("not-found", ...))?;
    listing.approve(now)?; // 도메인 가드: PendingReview only

    // 3. tx commit (listing save = audit + outbox)
    state.listing_repo.save(&listing, http_user_action(&auth, "approve_listing")).await?;

    // 4. notification (best-effort — 별도 tx)
    let notification = Notification::try_new(
        Id::new(),
        listing.owner_id.clone(),
        NotificationKind::ListingApproved,
        json!({"listing_id": listing.id, "title": listing.title}),
        now,
    )?;
    let notif_ctx = http_user_action(&auth, "notify_listing_approved");
    if let Err(e) = state.notification_repo.insert(&notification, notif_ctx).await {
        tracing::warn!(error = %e, "notification insert failed — proceeding");
    }

    Ok(Json(TransitionResponse { ... }))
}
```

reject endpoint = 동일 패턴 + body `{ reason: String }` (rejection_reason 추적).

### 3.3 bookmark notification trigger

```rust
// services/api/src/routes/bookmarks.rs::toggle_bookmark 갱신
//
// 1. 기존 bookmark.save_listing_bookmark
// 2. listing find -> owner != bookmarker 면 notification.insert
//    (이미 같은 owner 가 북마크 한 경우 dedup — FU 85)

let listing = state.listing_repo.find(&listing_id).await?.ok_or_else(...)?;
if listing.owner_id != auth.user.id {
    let notif = Notification::try_new(
        Id::new(),
        listing.owner_id.clone(),
        NotificationKind::ListingBookmarked,
        json!({
            "listing_id": listing_id,
            "bookmarker_id": auth.user.id,
            "bookmarker_name": auth.user.display_name,
        }),
        Utc::now(),
    )?;
    if let Err(e) = state.notification_repo.insert(&notif, ctx_for_notif).await {
        tracing::warn!(error = %e, "bookmark notification failed — proceeding");
    }
}
```

`BookmarksState` 에 `listing_repo` + `notification_repo` 추가 — handler 가
3 repo 묶어 사용.

### 3.4 `/me/notifications` 핸들러 (`services/api/src/routes/notifications.rs`)

```rust
GET /me/notifications?unread_only=true&limit=50
  -> { notifications: [...], total: N }

GET /me/notifications/unread-count
  -> { count: N }

PATCH /me/notifications/:id/read
  -> 204 No Content

POST /me/notifications/mark-all-read?kind=listing_approved
  -> { marked_count: N }
```

### 3.5 RBAC helper — `require_one_of_roles`

`crates/auth/src/role_guard.rs` 에 신규:

```rust
pub fn require_one_of_roles(
    auth: &AuthenticatedUser,
    roles: &[UserRole],
) -> Result<(), AuthError> {
    if roles.iter().any(|r| auth.user.roles.contains(r)) {
        Ok(())
    } else {
        Err(AuthError::InsufficientRole)
    }
}
```

기존 `require_role` 와 공존 — admin/operator OR 매칭에 사용.

### 3.6 ProblemDetails 매핑 추가

| 도메인 에러 | HTTP | type ID |
|---|---|---|
| `NotificationError::EmptyKind` | 400 | `validation` |
| `NotificationError::KindTooLong` | 400 | `validation` |
| Notification `RepoError::NotFound` | 404 | `not-found` |
| Notification `RepoError::Database` | 500 | `internal-error` |
| `ListingError::InvalidTransition` (approve from non-pending) | 409 | `invalid-transition` (이미 매핑) |

---

## 4. 검증 기준 (DoD)

1. `NotificationKind` enum + 8 단위 테스트 (variant str / from_str / serde / Other fallback)
2. `Notification.kind` 시그니처 변경 + 기존 단위 테스트 갱신
3. `PgNotificationRepository` round-trip enum 검증
4. admin approve / reject endpoints + 통합 테스트 4 (RBAC 통과 / non-admin 403 / state 가드 / notification INSERT 검증)
5. bookmark trigger — 본인 매물 북마크 시 notification skip / 다른 매물 시 INSERT 통합 테스트 2
6. `/me/notifications` 4 endpoints + 통합 테스트 5 (list / unread filter / unread count / mark read 멱등 / mark-all-read by kind)
7. Frontend `/me/notifications` 페이지 + NotificationBell + NotificationCard
8. Vitest 4 (api / mutations / unread count hook / kind label mapping)
9. Playwright e2e 1: 알림 발생 → 종 badge 1 → 페이지 진입 → 읽음 → badge 0
10. clippy `--all-targets` + typecheck + Vitest + Playwright 그린

---

## 5. SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 모든 mutation `MutationContext`. notification 도 audit_log + outbox |
| 2 자동강제 | NotificationKind enum — typo 차단. unknown kind = `Other` (forward-compat) |
| 3 추적성 | notification.insert 가 audit_log + outbox 자동 (SP5-ii 패턴) |
| 4 안전성 | mark_read 멱등. notification INSERT 실패 = best-effort warn (mutation 자체 commit 보존) |
| 5 가시성 | unread count 별도 endpoint — badge 폴링 효율. tracing instrument |
| 6 SSOT | `notification` 테이블 = 진실. denormalization X |
| 7 명확성 | NotificationKind enum 으로 클라이언트 → 메시지 템플릿 / route 매핑 명확 |

---

## 6. Follow-up

- **FU 76**: 외부 (Parcel/IndustrialComplex/Mfr/CourtAuction) 북마크 알림
- **FU 80**: WebSocket / SSE realtime push
- **FU 81**: `expires_at` 만료 알림 cron worker
- **FU 82**: 이메일 / SMS 알림 transport (외부 send)
- **FU 83**: 사용자 별 kind opt-out (subscription preferences)
- **FU 84**: SearchHistory 결과 변동 알림
- **FU 85**: bookmark notification dedup (같은 사용자 N회 X)
- **FU 86**: notification retention 365일 cron (이미 schema 365 day window —
  실 cron 미구현)
- **FU 87**: notification → outbox publisher sink → 외부 webhook (옵션)

---

## 7. Risk

- **bookmark trigger best-effort**: 알림 INSERT 실패 시 silent. 운영 visibility
  는 tracing warn 만 — alert 룰 추가 필요 (SP7 관측성)
- **single-tx vs multi-tx** debate: spec 결정 = multi-tx 단순화. application
  service layer 도입 시 (SP6-v-2) 단일 tx 로 evolve
- **`Notification.kind` 변경의 호환성**: 기존 DB 데이터 (테스트 데이터만 존재)
  와 frontend 가 새 enum 과 매칭. 1차 = clean break. production 데이터 부재로
  마이그 불필요
- **admin endpoint 신설 = LRQ workflow 와 분리됨**: BVQ/LRQ (Operations BC) 가
  별도 검토 큐 — 본 SP 의 admin endpoint 는 *가벼운* approve/reject. 정식
  reviewer assignment 흐름은 별도 (FU 90)
- **frontend polling 비용**: 1분 interval × 동시 사용자 수 = 부하. SSE/WebSocket
  도입 (FU 80) 까지는 5분 간격 권장. 1차 = 1분 interval (SP6-v 기본)
