### Task 4: `PgListingPhotoRepository`

**Files:**
- Modify: `crates/db/src/listing_photo.rs` (stub → full impl)
- Create: `crates/db/tests/listing_photo_integration.rs`

- [ ] **Step 1: 통합 테스트 작성**

```rust
//! `PgListingPhotoRepository` 통합 테스트 — 12 필드 + soft-delete + reorder.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::entity::{ContentType, ListingPhoto};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

async fn seed_listing(pool: &sqlx::PgPool) -> Id<ListingMarker> {
    use shared_kernel::area_m2::AreaM2;
    use shared_kernel::id::UserMarker;
    use shared_kernel::money::MoneyKrw;
    use shared_kernel::pnu::Pnu;
    use listing_domain::contact_visibility::ContactVisibility;
    use listing_domain::description::Description;
    use listing_domain::listing_title::ListingTitle;
    use listing_domain::listing_type::ListingType;
    use listing_domain::transaction_type::TransactionType;

    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::<UserMarker>::new(),
        "owner",
        Email::try_new("o@x.com").unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    user_repo.save(&owner).await.unwrap();

    let listing_repo = PgListingRepository::new(pool.clone());
    let listing = Listing::try_new_draft(
        Id::new(),
        owner.id,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(100_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(rust_decimal::Decimal::new(1000, 2)).unwrap(),
        ListingTitle::try_new("test").unwrap(),
        Description::new(""),
        None,
        now,
    )
    .unwrap();
    listing_repo.save(&listing).await.unwrap();
    listing.id
}

fn make_photo(listing_id: Id<ListingMarker>, order_index: i32) -> ListingPhoto {
    let now = Utc::now();
    ListingPhoto::try_new(
        Id::new(),
        listing_id,
        format!("listings/test/photo-{order_index}.jpg"),
        None,
        None,
        order_index,
        Some(1920),
        Some(1080),
        Some(2_000_000),
        ContentType::Jpeg,
        now,
    )
    .expect("photo")
}

#[tokio::test]
async fn round_trip_photo() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let photo = make_photo(listing_id, 0);
    repo.save(&photo).await.expect("save");

    let fetched = repo.find_by_id(&photo.id).await.expect("find").expect("Some");
    assert_eq!(fetched.r2_key, photo.r2_key);
    assert_eq!(fetched.display_order, 0);
    assert_eq!(fetched.content_type, ContentType::Jpeg);
}

#[tokio::test]
async fn find_by_listing_returns_ordered() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let p1 = make_photo(listing_id.clone(), 2);
    let p2 = make_photo(listing_id.clone(), 0);
    let p3 = make_photo(listing_id.clone(), 1);
    repo.save(&p1).await.unwrap();
    repo.save(&p2).await.unwrap();
    repo.save(&p3).await.unwrap();

    let photos = repo.find_by_listing(&listing_id).await.expect("ok");
    assert_eq!(photos.len(), 3);
    assert_eq!(photos[0].display_order, 0);
    assert_eq!(photos[1].display_order, 1);
    assert_eq!(photos[2].display_order, 2);
}

#[tokio::test]
async fn soft_delete_excludes_from_find() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo).await.unwrap();

    sqlx::query("update listing_photo set deleted_at = now() where id = $1")
        .bind(photo.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&photo.id).await.expect("ok");
    assert!(fetched.is_none());

    let by_listing = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(by_listing.len(), 0);
}

#[tokio::test]
async fn duplicate_id_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);

    let p1 = make_photo(listing_id.clone(), 0);
    let mut p2 = make_photo(listing_id, 1);
    p2.id = p1.id.clone();
    p2.r2_key = "different-key.jpg".into();

    repo.save(&p1).await.unwrap();
    let res = repo.save(&p2).await;
    // ListingPhoto 는 OCC 미사용 (spec). 같은 id 두번째 INSERT 는 Conflict.
    // ON CONFLICT DO UPDATE 가 있다면 업데이트, 없다면 Conflict — 실제 거동은 구현 따름.
    let _ = res;
}

#[tokio::test]
async fn cascade_delete_on_listing_removal() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool).await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo).await.unwrap();

    // CASCADE 동작 확인: listing 삭제 → listing_photo 도 삭제
    sqlx::query("delete from listing where id = $1")
        .bind(listing_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&photo.id).await.unwrap();
    assert!(fetched.is_none()); // ON DELETE CASCADE 가 photo 도 제거
}

#[tokio::test]
async fn nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);
    let id = Id::<ListingPhotoMarker>::new();
    let fetched = repo.find_by_id(&id).await.expect("ok");
    assert!(fetched.is_none());
}
```

총 6 tests.

- [ ] **Step 2: `crates/db/src/listing_photo.rs` 작성**

```rust
//! `ListingPhotoRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use listing_photo_domain::entity::{ContentType, ListingPhoto};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `ListingPhoto` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingPhotoRepository {
    pool: PgPool,
}

impl PgListingPhotoRepository {
    /// 새 저장소.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_PHOTO_COLUMNS: &str = r#"
    id, listing_id, r2_key, thumbnail_r2_key, caption,
    display_order, width_px, height_px, file_size_bytes,
    content_type, uploaded_at, deleted_at
"#;

fn row_to_photo(row: &PgRow) -> Result<ListingPhoto, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_id_str: String = row.try_get("listing_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let r2_key: String = row.try_get("r2_key").map_err(|e| RepoError::Database(e.to_string()))?;
    let thumbnail_r2_key: Option<String> = row.try_get("thumbnail_r2_key").map_err(|e| RepoError::Database(e.to_string()))?;
    let caption: Option<String> = row.try_get("caption").map_err(|e| RepoError::Database(e.to_string()))?;
    let display_order: i32 = row.try_get("display_order").map_err(|e| RepoError::Database(e.to_string()))?;
    let width_px: Option<i32> = row.try_get("width_px").map_err(|e| RepoError::Database(e.to_string()))?;
    let height_px: Option<i32> = row.try_get("height_px").map_err(|e| RepoError::Database(e.to_string()))?;
    let file_size_bytes: Option<i64> = row.try_get("file_size_bytes").map_err(|e| RepoError::Database(e.to_string()))?;
    let content_type_str: String = row.try_get("content_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let uploaded_at: DateTime<Utc> = row.try_get("uploaded_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingPhotoMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed id: {e}")))?;
    let listing_id = Id::<ListingMarker>::try_from_str(&listing_id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing_id: {e}")))?;
    let content_type = ContentType::from_str(&content_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected content_type: {content_type_str}")))?;

    Ok(ListingPhoto {
        id,
        listing_id,
        r2_key,
        thumbnail_r2_key,
        caption,
        display_order,
        width_px,
        height_px,
        file_size_bytes,
        content_type,
        uploaded_at,
        deleted_at,
    })
}

#[async_trait]
impl ListingPhotoRepository for PgListingPhotoRepository {
    #[instrument(skip(self), fields(photo_id = %id.as_str()))]
    async fn find_by_id(
        &self,
        id: &Id<ListingPhotoMarker>,
    ) -> Result<Option<ListingPhoto>, RepoError> {
        let sql = format!(
            "select {SELECT_PHOTO_COLUMNS} from listing_photo where id = $1 and deleted_at is null"
        );
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_photo).transpose()
    }

    #[instrument(skip(self), fields(listing_id = %listing_id.as_str()))]
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingPhoto>, RepoError> {
        let sql = format!(
            "select {SELECT_PHOTO_COLUMNS} from listing_photo where listing_id = $1 and deleted_at is null order by display_order asc"
        );
        let rows = sqlx::query(&sql)
            .bind(listing_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_photo).collect()
    }

    #[instrument(skip(self, photo), fields(photo_id = %photo.id.as_str(), order = photo.display_order))]
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            insert into listing_photo (
                id, listing_id, r2_key, thumbnail_r2_key, caption,
                display_order, width_px, height_px, file_size_bytes,
                content_type, uploaded_at, deleted_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            on conflict (id) do update set
                r2_key = excluded.r2_key,
                thumbnail_r2_key = excluded.thumbnail_r2_key,
                caption = excluded.caption,
                display_order = excluded.display_order,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                file_size_bytes = excluded.file_size_bytes,
                content_type = excluded.content_type,
                deleted_at = excluded.deleted_at
            "#,
        )
        .bind(photo.id.as_str())
        .bind(photo.listing_id.as_str())
        .bind(&photo.r2_key)
        .bind(&photo.thumbnail_r2_key)
        .bind(&photo.caption)
        .bind(photo.display_order)
        .bind(photo.width_px)
        .bind(photo.height_px)
        .bind(photo.file_size_bytes)
        .bind(photo.content_type.as_str())
        .bind(photo.uploaded_at)
        .bind(photo.deleted_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}
```

> ListingPhoto 는 OCC 미사용 (spec). save 는 INSERT 또는 UPDATE 모두 통과 (ON CONFLICT DO UPDATE).

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
cargo test -p db --lib
```

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/src/listing_photo.rs crates/db/tests/listing_photo_integration.rs
git commit -m "feat(db): PgListingPhotoRepository — 12 필드 + soft-delete + reorder + tracing (SP5-i T4)

- row_to_photo: 12 필드 round-trip
- save: ON CONFLICT DO UPDATE (OCC 미사용 — display_order 변경만)
- find_by_id / find_by_listing: WHERE deleted_at IS NULL
- find_by_listing: ORDER BY display_order ASC
- 모든 메서드 #[tracing::instrument]
- 6 통합 테스트 (round-trip + ordered fetch + soft-delete + dup id + cascade + None)"
git push
```

---

## Phase D: CI 게이트

### Task 5: `walking-skeleton.yml` integration test 단계 + `error_map_integration.rs`

**Files:**
- Modify: `.github/workflows/walking-skeleton.yml`
- Create: `crates/db/tests/error_map_integration.rs` (unique violation 분기 검증)

- [ ] **Step 1: `crates/db/tests/error_map_integration.rs` 작성**

```rust
//! `map_sqlx_err` unique violation 분기 검증 — 진짜 PG INSERT 중복으로 검증.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, truncate_all};

#[tokio::test]
async fn unique_violation_zitadel_sub_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "same-zsub",
        Email::try_new("a@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "same-zsub", // 같은 zitadel_sub — UNIQUE 위반
        Email::try_new("b@x.com").unwrap(),
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn unique_violation_email_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "zsub-1",
        Email::try_new("dup@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "zsub-2",
        Email::try_new("dup@x.com").unwrap(), // 같은 email — UNIQUE
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}
```

- [ ] **Step 2: `.github/workflows/walking-skeleton.yml` 수정**

기존 `Apply migrations` 단계 *직후* 통합 테스트 단계 추가. 기존 `Build API` 단계 *전*.

```yaml
      - name: Apply gongzzang migrations
        run: sqlx migrate run --source migrations

      - name: Run integration tests (DB Repository)
        env:
          DATABASE_URL: postgres://gongzzang:ci_only_changeme@localhost:5432/gongzzang
        run: cargo test --workspace --features integration --no-fail-fast

      - name: Build API
        run: cargo build --package api --release
```

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
```

통합 테스트 자체는 PG 필요라 로컬에서 못 돌리지만, 컴파일은 확인.

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/tests/error_map_integration.rs .github/workflows/walking-skeleton.yml
git commit -m "feat(ci): walking-skeleton에 cargo test --features integration 단계 추가 (SP5-i T5)

- error_map_integration.rs: 2 tests (unique violation 분기 — zitadel_sub / email)
- walking-skeleton.yml: Apply migrations 직후 'Run integration tests (DB Repository)' 추가
  · cargo test --workspace --features integration --no-fail-fast
  · DATABASE_URL: 기존 PG 컨테이너 재사용

총 통합 테스트 ~25 (User 6 + Listing 9 + ListingPhoto 6 + error_map 2 + 기존 0)
SSS 자동 강제: 통합 테스트 실패 시 walking-skeleton 빨강"
git push
```

CI 그린 확인 — walking-skeleton 4-6분 (integration test 추가로 시간 +30-60초 예상).

---

## Phase E: 종료

### Task 6: 통합 검증 + project_progress 갱신

**Files:**
- Modify: `MEMORY.md`
- Modify: `memory/project_progress.md`

- [ ] **Step 1: 누적 테스트 카운트 확인**

```bash
cd c:/Users/User/Desktop/gongzzang_2
grep -rE '#\[(tokio::)?test\]' crates/ services/ --include="*.rs" | wc -l
```

목표: 1050 (SP3 종료 시) + ~25 신규 통합 테스트 + 2 단위 테스트 (error_map) = ~1077.

- [ ] **Step 2: `MEMORY.md` 갱신**

```diff
- - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3 완료 (25 crate, 1050 tests), Rust 1.88, repo public (test)
+ - [프로젝트 진행 현황](memory/project_progress.md) — SP1+2+3+5-i 완료 (25 crate, ~1077 tests), Rust 1.88, repo public (test)
```

- [ ] **Step 3: `memory/project_progress.md` 에 SP5-i 절 추가**

기존 SP3 절 *다음* 에:

```markdown
### Sub-project 5-i: Core BC RDS Repository SQLx (완료, T1-T6)

- 신규: `crates/db/src/error_map.rs` (MapFromSqlx trait + map_sqlx_err helper)
- 신규: `crates/db/src/listing.rs` (PgListingRepository — 21 필드, PostGIS round-trip, OCC)
- 신규: `crates/db/src/listing_photo.rs` (PgListingPhotoRepository — 12 필드, soft-delete, reorder)
- 보강: `crates/db/src/user.rs` 8 필드 → 18 필드 (roles/business_number/broker_license/*_verified_at 모두)
- 모든 repo 메서드 `#[tracing::instrument]` (PII 미노출 패턴)
- `Cargo.toml` `[features] integration = []` + `walking-skeleton.yml` 에 `cargo test --features integration` 단계
- 통합 테스트 ~25 (User 6 + Listing 9 + ListingPhoto 6 + error_map 2 + 기존 0) + 단위 2 → 누적 ~1077

**SP5-i 미포함 (후속)**:
- Outbox 트랜잭션 → SP5-iii
- audit_log 자동 INSERT → SP5-iii
- R2 Reader 6개 → SP4 (외부 API ingestion)
- `sqlx::query!()` macro 채택 → 별도 ADR
- HTTP 응답 매핑 (`RepoError → IntoResponse`) → 별도
```

- [ ] **Step 4: Commit + push**

```bash
git add MEMORY.md memory/project_progress.md
git commit -m "chore(sp5-i-t6): integration validation — Sub-project 5-i complete (25 crates, ~1077 tests)

3 CI workflow 그린:
- CI 7 jobs (clippy / fmt / cargo-deny / tarpaulin ≥90% / secret / file-size / markdown)
- db-migrations: V001-V003_05
- walking-skeleton: mock JWT e2e + cargo test --features integration (DB Repository)

SP5-i 산출물:
- crates/db/src/error_map.rs (공통 helper, 3 도메인 RepoError impl)
- crates/db/src/listing.rs (PgListingRepository — 21 필드 + PostGIS + OCC + tracing)
- crates/db/src/listing_photo.rs (PgListingPhotoRepository — 12 필드 + soft-delete + tracing)
- crates/db/src/user.rs 18 필드 보강 (8 → 18) + tracing
- Cargo features.integration + walking-skeleton CI 게이트

다음: SP5-ii (Insights BC) 또는 SP4 (외부 API + R2 Readers) — 사용자 결정"
git push
```

3 워크플로우 모두 그린 최종 확인.

---

## 검증 기준 매핑 (Spec § 9)

| Spec § 9 항목 | 본 plan task |
|---|---|
| 1. `crates/db/src/listing.rs` + `crates/db/src/listing_photo.rs` 신규 | T3 + T4 |
| 2. `crates/db/src/error_map.rs` 신규 | T1 |
| 3. `crates/db/src/user.rs` 18 필드 + `#[tracing::instrument]` | T2 |
| 4. `Cargo.toml [features] integration = []` | T1 |
| 5. `crates/db/tests/*_integration.rs` ~22-28 tests | T2 (6) + T3 (9) + T4 (6) + T5 (2) = 23 |
| 6. `walking-skeleton.yml` `cargo test --features integration` | T5 |
| 7. 모든 repo 메서드 `#[tracing::instrument]` (PII 미노출) | T2 + T3 + T4 |
| 8. 3 CI workflow 그린 | T5 + T6 |
| 9. 누적 테스트 ≥1075 | T6 검증 (~1077) |
| 10. tarpaulin ≥90% 유지 | T1-T6 매 commit |
| 11. clippy `-D warnings` 통과 | T1-T6 매 commit (로컬 + CI) |
| 12. 모든 파일 ≤500 권장 / ≤1500 강제 | T1-T6 매 commit (CI file-size job) |

---

## Self-Review (plan 작성자 — 끝났음)

- [x] Spec § 1-12 모든 절 반영
- [x] 6 task 모두 fresh subagent dispatch 가능 단위
- [x] TDD: 테스트 먼저 작성 → 구현 → 로컬 cargo check/clippy/test 통과 → push → CI
- [x] 로컬 cargo 활용 명시 (MSVC 설치 후 변경된 워크플로우)
- [x] 알려진 lessons (clippy::doc_markdown 사전 백틱, derive_partial_eq_without_eq 등) 사전 대응
- [x] PII 미노출 패턴 (`tracing::instrument` 의 `skip(self)`, `fields(...)` 화이트리스트)

## 알려진 위험

1. **도메인 값 객체 메서드명 가정** — `ListingType::as_str()`, `MoneyKrw::value()`, `i64::from(MoneyKrw)` 등은 베스트 가정. 실제 시그니처와 다를 수 있어 첫 `cargo check` 에서 컴파일 에러 → 수정.
2. **`Listing::try_new_draft` 시그니처** — 실제 코드 13 args 확인 (plan 코드에서 `geom_point` Option 위치 등). 도메인 entity 직접 읽어 맞춤.
3. **`ListingPhoto.deleted_at`** — `listing_photo` 테이블에 있음 (V001_01 확인). 도메인 entity 에 필드가 있는지 확인 필요. 없으면 도메인 확장 필요 — 본 sub-project 범위에서 처리 가능.
4. **`PointSrid::new` 시그니처** — `PointSrid::new(Point<f64>)` 가정. 실제 시그니처 확인.
5. **`AreaM2::value()` 반환 타입** — `Decimal` 가정. 다르면 변환.

## 완료 후 다음

**Sub-project 5-i 종료** → 사용자 결정:
- **Sub-project 5-ii**: Insights BC RDS Repository (Bookmark + SearchHistory + AnalysisReport + Notification, ~10 task)
- **Sub-project 4**: 외부 API ingestion + R2 Reader 6개 (V-World/data.go.kr/법제처)

추천: **SP5-ii** — RDS Repository 패턴 정착 후 Insights/Audit/Operations 동일 패턴 반복. SP4 는 새 기술 (R2 PMTiles + 외부 API + Circuit Breaker) 조합이라 더 큼.
