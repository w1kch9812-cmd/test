//! `PgListingPhotoRepository` 통합 테스트 — 12 필드 round-trip + `display_order`
//! 정렬 + soft-delete 제외 + hard delete + `NotFound` + `ON DELETE CASCADE`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

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
    user_repo.save(&owner).await.unwrap();

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
    listing_repo.save(&listing).await.unwrap();
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
    repo.save(&photo).await.expect("save");

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

    repo.save(&make_photo(listing_id.clone(), 2)).await.unwrap();
    repo.save(&make_photo(listing_id.clone(), 0)).await.unwrap();
    repo.save(&make_photo(listing_id.clone(), 1)).await.unwrap();

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
    repo.save(&photo).await.unwrap();

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
    repo.save(&photo).await.unwrap();

    repo.delete(&photo.id).await.expect("delete ok");
    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 0);
}

#[tokio::test]
async fn delete_nonexistent_returns_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingPhotoRepository::new(pool);
    let id: Id<ListingPhotoMarker> = Id::new();
    let err = repo.delete(&id).await.unwrap_err();
    assert!(matches!(err, RepoError::NotFound));
}

#[tokio::test]
async fn cascade_delete_on_listing_removal() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let listing_id = seed_listing(&pool, "zsub-photo-5", "photo5@example.com").await;
    let repo = PgListingPhotoRepository::new(pool.clone());

    let photo = make_photo(listing_id.clone(), 0);
    repo.save(&photo).await.unwrap();

    // listing 삭제 → ON DELETE CASCADE 가 listing_photo 도 제거.
    sqlx::query("delete from listing where id = $1")
        .bind(listing_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let photos = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(photos.len(), 0);
}
