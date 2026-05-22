use db::listing::PgListingRepository;
use listing_domain::repository::ListingRepository as ListingRepo;
use shared_kernel::id::{Id, ListingMarker};

use super::common::{setup_test_pool, truncate_all};
use super::{seed_active_listing, seed_user};

#[tokio::test]
async fn increment_view_count_increments_value() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-5", "detail5@example.com").await;
    let viewer = seed_user(&pool, "zsub-detail-5v", "detail5v@example.com").await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());

    l_repo.increment_view_count(&listing_id).await.expect("ok");
    l_repo.increment_view_count(&listing_id).await.expect("ok");

    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");
    assert_eq!(detail.listing.view_count, 2);
}

#[tokio::test]
async fn increment_view_count_nonexistent_returns_not_found() {
    use listing_domain::repository::RepoError as ListingRepoError;

    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let l_repo = PgListingRepository::new(pool);

    let fake_id = Id::<ListingMarker>::new();
    let err = l_repo
        .increment_view_count(&fake_id)
        .await
        .expect_err("not found");
    assert!(matches!(err, ListingRepoError::NotFound));
}
