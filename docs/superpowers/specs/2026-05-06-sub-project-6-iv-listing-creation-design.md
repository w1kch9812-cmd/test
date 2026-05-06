# Sub-project 6-iv: Broker 매물 등록 (Listing Creation) — Spec

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 | SP6-i (Auth Core), SP6-ii (Listing 검색), SP5-iv (`MutationContext` 일원화), SP2b-i (Listing Aggregate) |
| 후속 | SP6-iii 매물 상세, SP6-v 알림 |
| 추정 | 7-9 task, 2-3일 |

---

## 1. 개요

브로커 (`UserRole::Broker`) 가 매물을 직접 등록하는 첫 mutation 화면. SP6-i 의 인증 + SP6-ii 의 search 가 read-only 였던 것에 비해, 처음으로 사용자 생성 mutation 이 frontend → backend tx 로 흐름.

**핵심 흐름**:

1. 브로커 로그인 → `/listings/new` 진입 (RBAC 가드)
2. 폼 입력 (parcel_pnu, listing_type, transaction_type, price/deposit/monthly_rent, area, title, description, photos)
3. `POST /listings` → `Listing::try_new_draft` → `PgListingRepository::save(listing, ctx)` → audit_log + outbox tx
4. 사진 업로드 — multipart `POST /listings/:id/photos` → R2 upload → `PgListingPhotoRepository::save` (소프트 인프라: SP6-iv 1차 = pre-signed URL 발급으로 한정 가능)
5. `POST /listings/:id/submit-for-review` → `Listing::submit_for_review(now)` → audit_log + outbox event

**SSS 핵심 결정**:

- *서버측 도메인 invariant 가 SSOT*. 폼 zod 검증은 UX 에디터 — 도메인 거부 시 RFC 7807 ProblemDetails 반환
- 모든 mutation 이 `MutationContext::new_user_action(actor_id, "create_listing"|"submit_for_review"|...)` 통과 → audit_log 자동
- `actor_id == owner_id` 강제. 다른 사용자 매물 수정/삭제 = `403 forbidden`
- Photo upload 는 *pre-signed URL 패턴* 1차 채택 (R2 직접 업로드, backend 가 메타만 보존). 직접 multipart 패스스루는 SP4-iii-e 의 R2 client 통합 후

---

## 2. 범위

### 포함

#### Backend (`services/api/src/routes/listings.rs` 확장 + 신규 모듈)

- `POST /listings` — `CreateListingRequest` → `Listing::try_new_draft` → `repo.save(listing, ctx)` → `201 Created` with `id`
- `PATCH /listings/:id` — partial update (Draft / Rejected 만 허용)
  - 변경 가능 필드: `title`, `description`, `price`, `deposit`, `monthly_rent`, `area`, `geom_point`, `contact_visibility`
  - 변경 불가: `parcel_pnu`, `listing_type`, `transaction_type` (이건 Aggregate 새로 생성)
  - `if-match: <version>` header 필수 (OCC) → 미일치면 `409 Conflict` (FU 15 부재 → version body 비교로 우회)
- `POST /listings/:id/submit-for-review` — `Listing::submit_for_review(now)` → `200 OK` with new version
- `POST /listings/:id/revise` — `Rejected → Draft` (`Listing::revise_after_rejection`) → `200 OK`
- `POST /listings/:id/photos` — pre-signed PUT URL 발급 (R2 path: `listings/<id>/<photo_id>.<ext>`) + `ListingPhoto` row INSERT (소프트 mode: meta-only)
  - body: `{ display_order, content_type, file_size_bytes? }`
  - response: `{ photo_id, presigned_put_url, expires_at }`
- `DELETE /listings/:id/photos/:photo_id` — soft-delete (`ListingPhoto.deleted_at`)

#### Domain — `Listing::update` 메서드 신규 (SP2b-i 보강)

- `crates/domain/core/listing/src/entity.rs` 에 `update_editable_fields(...)` 추가:
  - `title?`, `description?`, `price?`, `deposit?`, `monthly_rent?`, `area?`, `geom_point?`, `contact_visibility?`
  - 상태 가드: `Draft` 또는 `Rejected` 만 — 그 외 `Err(ListingError::ImmutableState { current })`
  - `version += 1`, `updated_at = at`
  - `transaction_type` 은 변경 불가 → `try_new_draft` 의 cross-field invariant 그대로 유지
- 신규 error variant: `ListingError::ImmutableState { current: ListingStatus }`

#### Frontend (`apps/web/app/(authenticated)/listings/new/`)

- `page.tsx` — 등록 폼 (broker only — proxy.ts 가 `/listings/new` 진입 시 `Broker` 역할 강제)
- `listing-form.tsx` — react-hook-form + zod resolver:
  - parcel_pnu (19 digits) + V-World 미리보기 (geocode 옵션 `useParcelLookup` hook — 후속, 1차는 텍스트 입력)
  - listing_type, transaction_type (select)
  - price + deposit + monthly_rent (cross-field zod refine = `transaction_type` 별)
  - area (number) + title (≤200) + description (≤5000)
  - PhotoUploader: pre-signed URL 받아 R2 직접 PUT, progress 표시
- `app/(authenticated)/listings/[id]/edit/page.tsx` — Draft / Rejected 매물 수정 (PATCH)
- `lib/listings/mutations.ts` — `useCreateListing`, `useUpdateListing`, `useSubmitForReview`, `useRevise`, `usePhotoUpload` (TanStack Query mutations)
- 한국어 메시지 (`listings.ko.json` 확장)
- proxy.ts — `/listings/new` 와 `/listings/:id/edit` 에 `Broker` 역할 강제

#### CI / 검증

- backend: 6 통합 테스트 (POST happy / PATCH OCC conflict / submit_for_review / RBAC 403 / cross-tenant 403 / photo presigned URL)
- frontend: 3 e2e (broker 로그인 → 등록 → 검색에서 보임 / non-broker 차단 / 폼 에러 메시지)
- frontend: 5 단위 (zod cross-field, form rendering, photo upload mock)

### 미포함

- **R2 직접 업로드 mock 외 실 통합**: pre-signed URL 발급은 구현하지만 실 R2 client 는 SP4-iii-e 와 묶어 별도 commit
- **사진 multipart upload**: 1차는 pre-signed URL 만 — 후속 (FU 49) 가 multipart proxy
- **사진 reorder UI**: SP6-iv-2 follow-up
- **매물 삭제** (DELETE /listings/:id): 정책 결정 필요 (soft? 30일 grace?) — FU 50
- **broker 검증 status 가드**: 현재 `User.broker_verified_at` 가 None 이어도 매물 등록 가능. 도메인 정책 결정 필요 — FU 51
- **archived → revise 부활**: 현재 머신은 Active → Sold 만 — Archived 미정의

---

## 3. 컴포넌트

### 3.1 도메인 — `Listing::update_editable_fields`

```rust
// crates/domain/core/listing/src/entity.rs

pub struct ListingUpdate {
    pub title: Option<ListingTitle>,
    pub description: Option<Description>,
    pub price: Option<MoneyKrw>,
    pub deposit: Option<Option<MoneyKrw>>,    // outer = 변경 의도, inner = Some/None 값
    pub monthly_rent: Option<Option<MoneyKrw>>,
    pub area: Option<AreaM2>,
    pub geom_point: Option<Option<PointSrid>>,
    pub contact_visibility: Option<ContactVisibility>,
}

impl Listing {
    pub fn update_editable_fields(
        &mut self,
        update: ListingUpdate,
        at: DateTime<Utc>,
    ) -> Result<(), ListingError> {
        if !matches!(self.status, ListingStatus::Draft | ListingStatus::Rejected) {
            return Err(ListingError::ImmutableState { current: self.status });
        }

        // transaction_type 변경 불가 → deposit/monthly_rent 변경 시 cross-field invariant 재검증
        let new_deposit = update.deposit.unwrap_or(self.deposit.clone());
        let new_monthly_rent = update.monthly_rent.unwrap_or(self.monthly_rent.clone());
        let dep_required = self.transaction_type.requires_deposit();
        let rent_required = self.transaction_type.requires_monthly_rent();
        if new_deposit.is_some() != dep_required || new_monthly_rent.is_some() != rent_required {
            return Err(ListingError::TransactionFieldsMismatch {
                transaction_type: self.transaction_type,
                deposit_required: dep_required,
                monthly_rent_required: rent_required,
            });
        }

        if let Some(t) = update.title { self.title = t; }
        if let Some(d) = update.description { self.description = d; }
        if let Some(p) = update.price { self.price = p; }
        self.deposit = new_deposit;
        self.monthly_rent = new_monthly_rent;
        if let Some(a) = update.area { self.area = a; }
        if let Some(g) = update.geom_point { self.geom_point = g; }
        if let Some(c) = update.contact_visibility { self.contact_visibility = c; }

        self.version += 1;
        self.updated_at = at;
        Ok(())
    }
}
```

### 3.2 HTTP — `MutationContext` helper

```rust
// services/api/src/http/mutation_ctx.rs (신규)

use auth::middleware::AuthenticatedUser;
use shared_kernel::mutation::MutationContext;
use serde_json::json;

/// HTTP 요청 → `MutationContext::new_user_action(actor_id, action)`.
/// `correlation_id` 는 axum extension 의 `X-Request-Id` (있으면) 또는 자동 ULID.
pub fn http_user_action(auth: &AuthenticatedUser, action: &str, correlation_id: &str) -> MutationContext {
    MutationContext::new_user_action(
        Some(auth.user.id),
        action,
        correlation_id,
    )
}
```

(첫 helper. SP7 관측성에서 X-Request-Id middleware 추가 예정 — 현재는 `cor_<ULID>` 자동 생성)

### 3.3 RBAC

- `crates/auth/src/role_guard.rs` 의 `require_role(auth, UserRole::Broker)` 그대로 사용.
- 본 SP 가 새 helper 추가 안 함 — 핸들러 첫줄에 호출.

```rust
pub async fn create_listing(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(body): Json<CreateListingRequest>,
) -> Result<(StatusCode, Json<CreateListingResponse>), ProblemResponse> {
    require_role(&auth, UserRole::Broker)
        .map_err(|_| problem("forbidden", "broker role required", StatusCode::FORBIDDEN, None))?;
    // ...
}
```

### 3.4 OCC — `if-match` header 패턴 (FU 15 부재 우회)

`PATCH /listings/:id` 가 받는 `if-match: <version>` 와 DB 의 현재 version 비교:

```rust
let current = state.listing_repo.find_by_id(&id).await?
    .ok_or(problem("not_found", "listing not found", StatusCode::NOT_FOUND, None))?;
if current.version != if_match_version {
    return Err(problem("version_conflict", "stale version", StatusCode::CONFLICT, None));
}
```

이후 도메인 메서드 가 `version += 1`. PgRepository::save 가 다시 OCC 검증 (DB 동시 update 차단). 이중 가드.

### 3.5 Photo Upload — Pre-signed URL pattern (SP6-iv 1차)

**철학**: 1MB+ 이미지 multipart 가 backend RAM 통과 = 비효율. R2 가 S3-호환 → pre-signed URL 발급 후 frontend 가 R2 에 직접 PUT.

**1차 mode (이번 SP — R2 client 미통합)**:

- `POST /listings/:id/photos` 가 `presigned_put_url: "<placeholder-pending-r2-integration>"` 반환
- `ListingPhoto` row 는 *생성됨*: `r2_key = "listings/<id>/<photo_id>.<ext>"`
- backend log 에 `tracing::info!(target = "photo.upload.mock", ...)` 발생
- frontend e2e 가 mock URL 검증 (실 PUT 시도 안 함)

**2차 mode (SP4-iii-e 후 — R2 통합)**:

- 실 `aws-sdk-s3` 로 pre-sign — `Cargo.toml` dep 추가
- `r2_key` 가 실제 R2 객체와 매핑

이 분리 = SP6-iv 가 SP4-iii-e blocker 안 됨. Listing creation 만으로 가치 있음.

### 3.6 Frontend — `listing-form.tsx`

- react-hook-form `useForm<CreateListingFormValues>`
- zod refine for cross-field:

```ts
const createListingSchema = z.object({
  parcel_pnu: z.string().regex(/^\d{19}$/),
  listing_type: z.enum(['factory', 'warehouse', 'office', 'retail', 'land', 'multi_purpose']),
  transaction_type: z.enum(['sale', 'monthly_rent', 'jeonse']),
  price_krw: z.number().int().positive(),
  deposit_krw: z.number().int().positive().nullable(),
  monthly_rent_krw: z.number().int().positive().nullable(),
  area_m2: z.number().positive(),
  title: z.string().min(1).max(200),
  description: z.string().max(5000),
  contact_visibility: z.enum(['public', 'login_required', 'verified_only']),
}).refine(
  (data) => {
    if (data.transaction_type === 'sale') {
      return data.deposit_krw === null && data.monthly_rent_krw === null;
    }
    if (data.transaction_type === 'jeonse') {
      return data.deposit_krw !== null && data.monthly_rent_krw === null;
    }
    if (data.transaction_type === 'monthly_rent') {
      return data.deposit_krw !== null && data.monthly_rent_krw !== null;
    }
    return false;
  },
  { message: 'transaction.fields.mismatch' }
);
```

이 cross-field 가 client side. Server-side 도메인이 다시 검증. Client 가 통과해도 server 가 거부할 수 있음 (SSOT = 도메인).

### 3.7 ProblemDetails 매핑

| 도메인 에러 | HTTP | type ID |
|---|---|---|
| `ListingError::TransactionFieldsMismatch` | 400 | `transaction-fields-mismatch` |
| `ListingError::InvalidTransition` | 409 | `invalid-transition` |
| `ListingError::ImmutableState` | 409 | `immutable-state` |
| `RepoError::OptimisticLockingConflict` | 409 | `version-conflict` |
| `RepoError::NotFound` | 404 | `not-found` |
| `AuthError::InsufficientRole` | 403 | `forbidden` |
| Domain validation generic | 400 | `validation` |

---

## 4. 검증 기준 (DoD)

1. `Listing::update_editable_fields` + 단위 테스트 5+ (SP2b-i 보강)
2. `services/api` 5 endpoint (POST / PATCH / submit / revise / photos) — 6+ 통합 테스트
3. RBAC: non-broker = 403, cross-tenant edit = 403
4. OCC: stale `if-match` = 409, 도메인 + DB 이중 가드
5. Frontend: `/listings/new` + `/listings/:id/edit` 폼 + e2e 3
6. ProblemDetails 7 매핑 (위 표)
7. 한국어 message bundle 갱신
8. clippy `--all-targets` + Vitest + Playwright + axe 그린
9. 5 CI workflow 그린

---

## 5. SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 모든 mutation 이 `MutationContext::new_user_action` 통과 — audit_log + outbox 자동 |
| 2 자동강제 | server-side 도메인이 SSOT. zod refine 은 UX assist, 불일치 시 도메인 에러가 ProblemDetails 로 |
| 3 추적성 | broker 의 매물 등록/수정/제출 모두 audit_log + outbox event |
| 4 안전성 | OCC `if-match` (요청 시점) + DB OCC (commit 시점) 이중. `ImmutableState` 도메인 invariant |
| 5 가시성 | tracing instrument 모든 핸들러 + `correlation_id` propagation (ULID) |
| 6 SSOT | 도메인 = 단일 진실. zod 는 derived (utoipa → ts 후속) |
| 7 명확성 | RFC 7807 ProblemDetails type ID 표 — frontend 가 type 으로 분기 |

---

## 6. Follow-up

- **FU 49**: Multipart photo upload proxy — large file 처리 + virus scan hook
- **FU 50**: 매물 삭제 정책 (soft delete + 30일 grace)
- **FU 51**: broker_verified_at 가드 — 미검증 broker = 매물 등록 차단 정책 결정
- **FU 52**: V-World geocode 통합 — parcel_pnu 입력 보조 (자동완성)
- **FU 53**: 사진 reorder UI (drag-drop)
- **FU 54**: PATCH archived/expired 부활 머신 (현재 unsupported)
- **FU 55**: utoipa schema → TypeScript zod 자동 생성 (`packages/api-types`)

---

## 7. Risk

- **R2 client 부재**: 1차는 mock URL — frontend e2e 가 실 R2 안 침. SP4-iii-e 의 `aws-sdk-s3` 통합이 들어와야 진짜 동작
- **broker 미검증**: 현재 정책 미정. 1차는 `Broker` role 만 있으면 등록 가능 (FU 51 까지)
- **`Description` 입력 sanitization**: HTML/script injection 방어는 도메인 invariant 가 아님 — `description.try_new` 가 길이만 체크. XSS 는 frontend render 시점 책임 (React 가 default-escape)
