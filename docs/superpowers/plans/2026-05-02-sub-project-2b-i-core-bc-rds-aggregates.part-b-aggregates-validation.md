# Sub-project 2b-i - Part B: Aggregates, Repositories, Validation, And Handoff

Parent index: [Sub-project 2b-i Core BC RDS Aggregates](./2026-05-02-sub-project-2b-i-core-bc-rds-aggregates.md).

## Phase C — Aggregate 구현 (Tasks 8-13)

각 BC crate는 spec § 8.3 패턴 따름. *Repository trait는 port만* (SQLx 구현은 sub-project 5). 도메인 메서드는 *invariant 체크 + 상태 변경 + 도메인 이벤트 emit* 책임.

### Task 8: User Aggregate — struct + 기본 invariant

**스펙:** spec § 5.1 user 테이블 (lines 152-176) + spec § 8.3 패턴

**Files:**
- `crates/domain/core/user/src/entity.rs` — `User` struct
- `crates/domain/core/user/src/errors.rs` — `UserError` enum

**Aggregate 필드 (spec § 5.1 컬럼 1:1):**

```rust
pub struct User {
    pub id: Id<UserMarker>,
    pub zitadel_sub: String,                            // varchar(255)
    pub email: Email,
    pub phone_kr_hash: Option<String>,                  // SHA-256 hash (PIPA)
    pub display_name: String,                           // varchar(100), non-empty
    pub user_kind: UserKind,                            // enum: Individual / Corporation
    pub business_number: Option<BusinessNumber>,
    pub business_verified_at: Option<DateTime<Utc>>,
    pub broker_license_number: Option<BrokerLicense>,
    pub broker_verified_at: Option<DateTime<Utc>>,
    pub roles: Vec<UserRole>,                           // text[]
    pub nice_verified_at: Option<DateTime<Utc>>,
    pub marketing_consent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,              // soft-delete (PIPA RTBF)
    pub version: i64,                                   // optimistic locking
}
```

`UserKind` (enum, 2값 individual/corporation) + `UserRole` (enum, 7값: Buyer/Seller/Broker/Developer/Enterprise/Operator/Admin) — *이 task에서 함께 정의* (spec § 5.1 line 158, 163).

**핵심 invariant 메서드 (이 task에서 작성):**
```rust
pub fn try_new(...) -> Result<Self, UserError>      // 생성 시 invariant 검증
pub fn is_business_verified(&self) -> bool          // business_verified_at IS NOT NULL
pub fn is_broker(&self) -> bool                      // broker_verified_at IS NOT NULL
pub fn is_active(&self) -> bool                      // deleted_at IS NULL
pub fn has_role(&self, role: UserRole) -> bool       // roles 검사
```

**Invariant rules:**
- `display_name` 비어있지 않음 (≤100자)
- `business_verified_at` is Some → `business_number` 도 Some 강제
- `broker_verified_at` is Some → `broker_license_number` 도 Some 강제
- `user_kind == Corporation` → `business_number` 권장 (Some)이지만 *강제 X* (개인사업자 제외)

- [ ] Tests (≥15): 정상 생성, 각 invariant 위반 거부, role 검사, soft-delete 상태, business/broker 검증 일관성
- [ ] CI green

```bash
git commit -m "feat(user-domain): User Aggregate struct + UserKind + UserRole + try_new invariants"
```

### Task 9: User 도메인 메서드 + Repository trait

**Files:**
- Modify: `crates/domain/core/user/src/entity.rs` — 도메인 메서드 추가
- Create: `crates/domain/core/user/src/repository.rs` — `UserRepository` trait

**도메인 메서드 (mutate self + version 증가):**
```rust
pub fn verify_business(&mut self, bn: BusinessNumber, at: DateTime<Utc>)
    -> Result<(), UserError>;                         // user_kind == Corporation && bn 일치 시
pub fn revoke_business_verification(&mut self, at: DateTime<Utc>);
pub fn verify_broker(&mut self, license: BrokerLicense, at: DateTime<Utc>)
    -> Result<(), UserError>;
pub fn add_role(&mut self, role: UserRole);          // 중복 방지
pub fn remove_role(&mut self, role: UserRole);
pub fn record_login(&mut self, at: DateTime<Utc>);
pub fn soft_delete(&mut self, at: DateTime<Utc>);    // PIPA RTBF
pub fn record_marketing_consent(&mut self, at: DateTime<Utc>);
```

매 메서드 끝에 `self.version += 1; self.updated_at = at;` 자동.

**Repository trait:**
```rust
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError>;
    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError>;
    async fn save(&self, user: &User) -> Result<(), RepoError>;  // version 충돌 시 RepoError::Conflict
}

pub enum RepoError {
    NotFound,
    Conflict,
    Database(String),
}
```

`RepoError`는 *각 BC crate에 중복 정의*하지 말고 shared-kernel `repository_error.rs`에 단일 정의 (다음 task에서 추가하거나 미리 추출).

- [ ] Tests (≥12): 8 메서드 happy path + version 증가 검증 + 4 invariant violation
- [ ] CI green

```bash
git commit -m "feat(user-domain): User domain methods (verify_*, soft_delete, etc.) + UserRepository trait"
```

### Task 10: Listing Aggregate — struct + invariant

**스펙:** spec § 5.1 listing 테이블 (lines 180-214) + spec § 8.3 (lines 844-905) + V003_01 cross-field CHECK

**Files:**
- `crates/domain/core/listing/src/entity.rs`
- `crates/domain/core/listing/src/errors.rs`

**필드:** spec § 8.3 lines 848-869 따라 — 17 필드. 핵심:
- `id: Id<ListingMarker>`
- `owner_id: Id<UserMarker>` (FK)
- `parcel_pnu: Pnu` (R2 매핑, FK 아님)
- `listing_type: ListingType`
- `transaction_type: TransactionType`
- `price: MoneyKrw`
- `deposit: Option<MoneyKrw>`
- `monthly_rent: Option<MoneyKrw>`
- `area: AreaM2`
- `title: ListingTitle`
- `description: Description`
- `status: ListingStatus` (default Draft)
- `contact_visibility: ContactVisibility` (default LoginRequired)
- `view_count: u64`, `bookmark_count: u64`
- `geom_point: Option<PointSrid>`
- `created_at`, `updated_at`, `expires_at`, `version`

**Invariant rules (V003_01과 일치):**
| transaction_type | deposit | monthly_rent |
|---|---|---|
| Sale | None | None |
| MonthlyRent | Some | Some |
| Jeonse | Some | None |

`try_new` 또는 `try_create_draft`에서 강제. 위반 시 `ListingError::TransactionFieldsMismatch`.

추가 invariant:
- `price > 0` (MoneyKrw가 자동 강제)
- `area > 0` (AreaM2가 자동 강제)
- `geom_point.srid == Wgs84` (PointSrid가 자동 강제)
- `monthly_rent.is_some()` 시 `deposit > monthly_rent * 12` 같은 도메인 규칙 — *Plan 2b-i 범위 외*, 추후

- [ ] Tests (≥18): 정상 3가지 transaction_type, 각 invariant 위반 9가지, MoneyKrw 음수 (already covered by Money), area 0 (already), geom SRID
- [ ] CI green

```bash
git commit -m "feat(listing-domain): Listing Aggregate struct + try_new transaction_type invariant"
```

### Task 11: Listing 도메인 메서드 (상태 머신) + Repository trait

**스펙:** spec § 8.3 lines 882-895 (상태 전이 메서드)

**Files:**
- Modify: `crates/domain/core/listing/src/entity.rs`
- Create: `crates/domain/core/listing/src/repository.rs`

**도메인 메서드 (모두 `Result<(), ListingError>` + version 증가):**
```rust
pub fn submit_for_review(&mut self, at: DateTime<Utc>) -> Result<(), ListingError>;
pub fn approve(&mut self, reviewed_by: Id<UserMarker>, at: DateTime<Utc>);
pub fn reject(&mut self, reviewed_by: Id<UserMarker>, reason: String, at: DateTime<Utc>);
pub fn mark_sold(&mut self, at: DateTime<Utc>);
pub fn expire(&mut self, at: DateTime<Utc>);
pub fn revise_after_rejection(&mut self, at: DateTime<Utc>);   // Rejected → Draft
pub fn increment_view_count(&mut self);
pub fn record_bookmark(&mut self);                              // bookmark_count++
pub fn release_bookmark(&mut self);                             // count-- (saturating)
```

매 transition은 `ListingStatus::can_transition_to`로 검증. 위반 시 `ListingError::InvalidTransition { from, to }`.

**Repository trait:**
```rust
#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn find(&self, id: &Id<ListingMarker>) -> Result<Option<Listing>, RepoError>;
    async fn find_markers_in_bbox(&self, bbox: BoundingBox)
        -> Result<Vec<ListingMarker>, RepoError>;   // 지도 마커용 lightweight projection
    async fn save(&self, listing: &Listing) -> Result<(), RepoError>;
    async fn find_by_owner(&self, owner_id: &Id<UserMarker>, status: Option<ListingStatus>)
        -> Result<Vec<Listing>, RepoError>;
}

pub struct ListingMarker {
    pub id: Id<ListingMarker>,
    pub geom: PointSrid,
    pub price: MoneyKrw,
    pub listing_type: ListingType,
    pub transaction_type: TransactionType,
}

pub struct BoundingBox {
    pub min_lng: f64, pub min_lat: f64,
    pub max_lng: f64, pub max_lat: f64,
}
```

- [ ] Tests (≥20): 9 메서드 × happy path + InvalidTransition 6가지 + version 증가
- [ ] CI green

```bash
git commit -m "feat(listing-domain): Listing state machine methods + ListingRepository trait"
```

### Task 12: ListingPhoto Aggregate + Repository

**스펙:** spec § 5.1 listing_photo (lines 219-237)

**Files:**
- `crates/domain/core/listing-photo/src/entity.rs` + `errors.rs` + `repository.rs`

**필드 (spec § 5.1 1:1):**
```rust
pub struct ListingPhoto {
    pub id: Id<ListingPhotoMarker>,        // lph_... (3-char prefix per spec § 5.1 정정)
    pub listing_id: Id<ListingMarker>,     // FK + ON DELETE CASCADE
    pub r2_key: String,                    // 'listings/lst_.../photos/p1.jpg'
    pub thumbnail_r2_key: Option<String>,
    pub caption: Option<String>,           // ≤200자
    pub display_order: i32,                // ≥0
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
    pub file_size_bytes: Option<i64>,
    pub content_type: PhotoContentType,    // image/jpeg, image/png, image/webp
    pub uploaded_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

`PhotoContentType` (enum 3값) + `ListingPhotoMarker` 추가 (Id<ListingPhotoMarker>) — `IdPrefix const PREFIX = "lph"` 구현 시 shared-kernel `id.rs`도 갱신.

**도메인 메서드:**
```rust
pub fn try_new(...) -> Result<Self, ListingPhotoError>;
pub fn soft_delete(&mut self, at: DateTime<Utc>);
pub fn reorder(&mut self, new_order: i32) -> Result<(), ListingPhotoError>;  // ≥0
```

**Repository trait:**
```rust
#[async_trait::async_trait]
pub trait ListingPhotoRepository: Send + Sync {
    async fn find_by_listing(&self, listing_id: &Id<ListingMarker>)
        -> Result<Vec<ListingPhoto>, RepoError>;     // ON DELETE CASCADE 의존
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError>;
    async fn delete(&self, id: &Id<ListingPhotoMarker>) -> Result<(), RepoError>;
}
```

- [ ] Tests (≥10): try_new + content_type 3값 + soft_delete + reorder + display_order 음수 거부
- [ ] *shared-kernel id.rs에 ListingPhotoMarker 추가* + 그 변경의 테스트도 추가 (prefix `lph`)
- [ ] CI green

```bash
git commit -m "feat(listing-photo-domain): ListingPhoto Aggregate + R2 key + soft-delete + ListingPhotoMarker"
```

### Task 13: 통합 검증

**Files:**
- Modify: `tests/migrations/test_v001_full.sh` — 변경 없음 (DB는 Plan 2a에서 끝)
- Verify: 모든 신규 crate 컴파일 + tarpaulin ≥90% 유지
- Update: `MEMORY.md` — Plan 2b-i 완료 추가

검증 명령어 (CI에서 자동 실행):
```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo tarpaulin --workspace --skip-clean --out Lcov --fail-under 90
```

이 task에서 코드 변경 없음. CI green이 곧 완료 신호.

- [ ] Step 1: workspace 전체 cargo check + clippy 통과 확인
- [ ] Step 2: 누적 테스트 수 보고 (Plan 2a 167 → ?)
- [ ] Step 3: tarpaulin 90% 유지 확인
- [ ] Step 4: Plan 2b-i 완료 commit + memory 갱신

```bash
git commit -m "chore(2b-i): integration validation — all 4 Core BC crates compile + tarpaulin ≥90%"
```

---

## Self-Review Checklist (plan 작성자)

- [x] Spec § 8.1, § 5.1, § 8.3 line ranges 직접 인용 (paraphrase 최소화)
- [x] 13 task 모두 *spec § X 직독 + verbatim* 패턴
- [x] 값 객체 6개 (Tasks 2-7) — Phase E 패턴 (Tasks 12-25) 동일하게 dispatch
- [x] Aggregate 3개 (Tasks 8, 10, 12) — try_new + invariant + 도메인 메서드 + Repository trait
- [x] Repository trait는 *port only* (구현 sub-project 5)
- [x] 도메인 이벤트 *명시 X* — Plan 2c에서 outbox 도입 시 함께
- [x] DB 변경 없음 — 마이그레이션 task 없음
- [x] tarpaulin 90% 게이트 유지 (Phase C 추가 코드만큼 테스트 추가)

## 알려진 위험

1. **ListingPhoto prefix `lph`** — spec § 5.1 패치된 인라인 코멘트 (Plan 2a Task 12 수정)와 일치. 새 IdPrefix marker를 shared-kernel에 추가하는 work이 Task 12에 포함됨
2. **RepoError 위치** — Task 9에서 shared-kernel에 둘지 vs 각 BC crate에 둘지 결정. SSOT 관점에서 *shared-kernel에 단일 정의 + re-export*. 이 결정 Task 9 dispatch에서 명확화
3. **async-trait 의존** — Repository trait가 `async fn in trait` 안정 (Rust 1.85)이지만 *dyn-compatible*하려면 여전히 `#[async_trait]` 필요. Task 1에서 workspace dep 추가
4. **IdPrefix 추가 markers** — Task 12에서 ListingPhotoMarker (`lph`) 추가. 향후 Plan 2c/2d에서 더 많은 marker 필요 (audit, outbox, pipeline 등) — 그때 shared-kernel id.rs 추가 갱신

## 완료 후 다음

- **Plan 2b-ii** — R2 정적 BC 4개 (Parcel, Building, IndustrialComplex, Manufacturer) — Reader trait 위주
- **Plan 2c** — Market BC + Insights BC + Operations BC + Pipeline + R2 디렉토리 + 도메인 이벤트 + 최종 검증
- **Sub-project 3** — Auth (Zitadel JWT 미들웨어)
- **Sub-project 5** — Repository SQLx 구현체

이후 Plan 2b-i + 2b-ii + 2c 합치면 Sub-project 2 완전 종료.
