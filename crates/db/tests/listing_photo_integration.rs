//! `PgListingPhotoRepository` 통합 테스트 — 12 필드 round-trip + `display_order`
//! 정렬 + soft-delete 제외 + hard delete + `NotFound` + `ON DELETE CASCADE`
//! + SP5-iv transactional `audit_log` / `outbox_event` 검증.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

/// 테스트용 단순 도메인 이벤트.
#[derive(Debug)]
struct TestEvent {
    event_type: &'static str,
    aggregate_id: String,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
}

impl DomainEvent for TestEvent {
    fn event_type(&self) -> &'static str {
        self.event_type
    }
    fn aggregate_id(&self) -> String {
        self.aggregate_id.clone()
    }
    fn payload(&self) -> serde_json::Value {
        self.payload.clone()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

/// `User` + `Listing` 시드 — `listing_photo` 의 `FK` 충족.
async fn seed_listing(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<ListingMarker> {
    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let owner_id = owner.id.clone();
    user_repo.save(&owner, test_ctx()).await.unwrap();

    let listing_repo = PgListingRepository::new(pool.clone());
    let listing = Listing::try_new_draft(
        Id::new(),
        owner_id,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(100_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(100.00).unwrap(),
        ListingTitle::try_new("photo test").unwrap(),
        Description::try_new("").unwrap(),
        None,
        now,
    )
    .expect("listing");
    let listing_id = listing.id.clone();
    listing_repo.save(&listing, test_ctx()).await.unwrap();
    listing_id
}

fn make_photo(listing_id: Id<ListingMarker>, order: i32) -> ListingPhoto {
    let now = Utc::now();
    ListingPhoto::try_new(
        Id::new(),
        listing_id,
        &format!("listings/test/photo-{order}.jpg"),
        None,
        None,
        order,
        Some(1920),
        Some(1080),
        Some(2_000_000),
        PhotoContentType::Jpeg,
        now,
    )
    .expect("photo")
}

#[tokio::test]
async fn round_trip_via_find_by_listing() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-1", "photo1@example.com").await;
    let repo = PgListingPhotoRepository::new(pool);

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo, test_ctx()).await.expect("save");

    let photos = repo.find_by_listing(&listing_id).await.expect("find");
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0].r2_key, photo.r2_key);
    assert_eq!(photos[0].display_order, 0);
    assert_eq!(photos[0].content_type, PhotoContentType::Jpeg);
    assert_eq!(photos[0].width_px, Some(1920));
    assert_eq!(photos[0].height_px, Some(1080));
    assert_eq!(photos[0].file_size_bytes, Some(2_000_000));
    assert!(photos[0].deleted_at.is_none());
}

#[tokio::test]
async fn find_by_listing_orders_by_display_order_asc() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-2", "photo2@example.com").await;
    let repo = PgListingPhotoRepository::new(pool);

    repo.save(&make_photo(listing_id.clone(), 2), test_ctx())
        .await
        .unwrap();
    repo.save(&make_photo(listing_id.clone(), 0), test_ctx())
        .await
        .unwrap();
    repo.save(&make_photo(listing_id.clone(), 1), test_ctx())
        .await
        .unwrap();

    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 3);
    assert_eq!(photos[0].display_order, 0);
    assert_eq!(photos[1].display_order, 1);
    assert_eq!(photos[2].display_order, 2);
}

#[tokio::test]
async fn soft_deleted_photo_excluded_from_find() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-3", "photo3@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo, test_ctx()).await.unwrap();

    sqlx::query("update listing_photo set deleted_at = now() where id = $1")
        .bind(photo.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 0);
}

#[tokio::test]
async fn delete_removes_photo() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-4", "photo4@example.com").await;
    let repo = PgListingPhotoRepository::new(pool);

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo, test_ctx()).await.unwrap();

    repo.delete(&photo.id, test_ctx()).await.expect("delete ok");
    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 0);
}

#[tokio::test]
async fn delete_nonexistent_returns_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);
    let id: Id<ListingPhotoMarker> = Id::new();
    let err = repo.delete(&id, test_ctx()).await.unwrap_err();
    assert!(matches!(err, RepoError::NotFound));
}

#[tokio::test]
async fn cascade_delete_on_listing_removal() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-5", "photo5@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo, test_ctx()).await.unwrap();

    // listing 삭제 → ON DELETE CASCADE 가 listing_photo 도 제거.
    sqlx::query("delete from listing where id = $1")
        .bind(listing_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 0);
}

// ---- SP5-iv: transactional audit_log + outbox_event 검증 ----

#[tokio::test]
async fn save_inserts_photo_audit_log_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-aud-1", "phaud1@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id, 0);
    let ctx = MutationContext::new_system_action("corr-photo-aud-1", "upload_photo");
    repo.save(&photo, ctx).await.expect("save");

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'listing_photo' and resource_id = $1",
    )
    .bind(photo.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn save_photo_with_events_inserts_outbox_per_event() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-evt-1", "phevt1@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id, 0);
    let event1: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "listing_photo.uploaded",
        aggregate_id: photo.id.as_str().to_owned(),
        payload: serde_json::json!({"r2_key": photo.r2_key.clone()}),
        occurred_at: Utc::now(),
    });
    let event2: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "listing_photo.thumbnail_queued",
        aggregate_id: photo.id.as_str().to_owned(),
        payload: serde_json::json!({}),
        occurred_at: Utc::now(),
    });
    let ctx = MutationContext::new_system_action("corr-photo-evt-1", "upload_photo")
        .with_events(vec![event1, event2]);
    repo.save(&photo, ctx).await.expect("save");

    let outbox_count: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'listing_photo' and aggregate_id = $1",
    )
    .bind(photo.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outbox_count.0, 2);
}

#[tokio::test]
async fn delete_photo_audit_logs_with_action_delete() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-del-1", "phdel1@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id, 0);
    repo.save(&photo, test_ctx()).await.unwrap();

    let delete_ctx = MutationContext::new_system_action("corr-photo-del-1", "delete");
    repo.delete(&photo.id, delete_ctx).await.expect("delete");

    let delete_audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'listing_photo' and resource_id = $1 and action = 'delete'",
    )
    .bind(photo.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(delete_audit_count.0, 1);
}
