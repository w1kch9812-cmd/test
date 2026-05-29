use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use listing_domain::repository::ListingRepository as ListingRepo;
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::ListingPhotoRepository;
use shared_kernel::id::{Id, ListingPhotoMarker};

use super::common::{setup_test_pool, test_ctx, truncate_all};
use super::{seed_active_listing, seed_user};

#[tokio::test]
async fn find_detail_returns_confirmed_photo_id_for_download_route() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-photo-id", "detail-photo-id@example.com").await;
    let viewer = seed_user(
        &pool,
        "zsub-detail-photo-id-v",
        "detail-photo-id-v@example.com",
    )
    .await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());
    let p_repo = PgListingPhotoRepository::new(pool.clone());
    let photo = ListingPhoto::try_new(
        Id::<ListingPhotoMarker>::new(),
        listing_id.clone(),
        "listings/test/confirmed-detail-photo.jpg",
        None,
        None,
        0,
        None,
        None,
        Some(100),
        PhotoContentType::Jpeg,
        chrono::Utc::now(),
    )
    .expect("confirmed photo");
    p_repo.save(&photo, test_ctx()).await.expect("save");

    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");

    assert_eq!(detail.photos.len(), 1);
    assert_eq!(detail.photos[0].photo_id, photo.id.as_str());
}
