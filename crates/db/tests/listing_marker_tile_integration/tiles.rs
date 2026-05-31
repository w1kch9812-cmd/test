use chrono::Utc;
use db::listing::PgListingRepository;
use listing_domain::repository::{
    ListingMarkerFilter, ListingMarkerFilterSpec, ListingMarkerTileQuery, ListingRepository,
};
use shared_kernel::listing_type::ListingType;
use shared_kernel::mutation::MutationContext;
use shared_kernel::transaction_type::TransactionType;

use super::{
    activate_listing, lock_marker_tile_tests, make_listing, make_listing_of_type, seed_anchor,
    seed_owner, setup_test_pool, test_ctx, truncate_all, SEOUL_Z11_X, SEOUL_Z11_Y, SEOUL_Z14_X,
    SEOUL_Z14_Y,
};

#[tokio::test]
async fn listing_marker_tile_applies_normalized_filter_spec() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-tile-filter",
        "marker-tile-filter@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100120000";
    seed_anchor(&pool, pnu).await;

    let mut factory = make_listing_of_type(
        owner.clone(),
        pnu,
        "Filtered factory listing",
        ListingType::Factory,
    );
    let mut warehouse = make_listing_of_type(
        owner.clone(),
        pnu,
        "Filtered warehouse listing",
        ListingType::Warehouse,
    );

    activate_listing(&repo, &mut factory, &owner).await;
    activate_listing(&repo, &mut warehouse, &owner).await;
    repo.upsert_listing_marker_projection(&factory.id)
        .await
        .unwrap();
    repo.upsert_listing_marker_projection(&warehouse.id)
        .await
        .unwrap();

    let warehouse_only = ListingMarkerFilterSpec {
        types: vec![ListingType::Warehouse],
        transactions: vec![TransactionType::Sale],
        min_area_m2: Some(300),
        max_area_m2: Some(400),
        min_price_krw: Some(100_000_000),
        max_price_krw: Some(900_000_000),
    }
    .try_normalized()
    .expect("warehouse filter");

    let tile = repo
        .find_listing_marker_tile(ListingMarkerTileQuery::new(
            14,
            SEOUL_Z14_X,
            SEOUL_Z14_Y,
            ListingMarkerFilter::Normalized(warehouse_only),
        ))
        .await
        .unwrap();

    assert_eq!(tile.eligible_count, 1);
    assert_eq!(tile.represented_count, 1);
    assert_eq!(tile.feature_count, 1);
}

#[tokio::test]
async fn listing_marker_tile_aggregates_low_zoom_without_dropping_records() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-aggregate",
        "marker-aggregate@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100190000";
    seed_anchor(&pool, pnu).await;

    let mut first = make_listing(owner.clone(), pnu, "Aggregate listing one");
    let mut second = make_listing(owner.clone(), pnu, "Aggregate listing two");
    activate_listing(&repo, &mut first, &owner).await;
    activate_listing(&repo, &mut second, &owner).await;

    let query = ListingMarkerTileQuery::try_new(
        11,
        SEOUL_Z11_X,
        SEOUL_Z11_Y,
        ListingMarkerFilter::AllActive,
    )
    .expect("low zoom listing marker query");
    let tile = repo.find_listing_marker_tile(query).await.unwrap();

    assert!(!tile.bytes.is_empty());
    assert_eq!(tile.eligible_count, 2);
    assert_eq!(tile.represented_count, 2);
    assert_eq!(tile.feature_count, 0);
    assert_eq!(tile.aggregate_count, 1);
}

#[tokio::test]
async fn listing_marker_tile_represents_every_active_listing_on_same_pnu() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-tile-1", "marker-tile-1@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100070000";
    seed_anchor(&pool, pnu).await;

    let mut first = make_listing(owner.clone(), pnu, "Marker tile listing one");
    let mut second = make_listing(owner.clone(), pnu, "Marker tile listing two");
    let draft = make_listing(owner.clone(), pnu, "Marker tile draft");

    activate_listing(&repo, &mut first, &owner).await;
    activate_listing(&repo, &mut second, &owner).await;
    repo.upsert_listing_marker_projection(&first.id)
        .await
        .unwrap();
    repo.upsert_listing_marker_projection(&second.id)
        .await
        .unwrap();
    repo.save(&draft, test_ctx()).await.unwrap();

    let tile = repo
        .find_listing_marker_tile(ListingMarkerTileQuery::new(
            14,
            SEOUL_Z14_X,
            SEOUL_Z14_Y,
            ListingMarkerFilter::AllActive,
        ))
        .await
        .unwrap();

    assert!(!tile.bytes.is_empty());
    assert_eq!(tile.layer_name, "listing");
    assert_eq!(tile.eligible_count, 2);
    assert_eq!(tile.represented_count, 2);
    assert_eq!(tile.feature_count, 2);
    assert_eq!(tile.aggregate_count, 0);
}

#[tokio::test]
async fn listing_marker_tile_uses_auto_projection_without_manual_upsert() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-tile-no-projection",
        "marker-tile-no-projection@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100100000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Missing projection listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let tile = repo
        .find_listing_marker_tile(ListingMarkerTileQuery::new(
            14,
            SEOUL_Z14_X,
            SEOUL_Z14_Y,
            ListingMarkerFilter::AllActive,
        ))
        .await
        .unwrap();

    assert_eq!(tile.eligible_count, 1);
    assert_eq!(tile.represented_count, 1);
    assert_eq!(tile.feature_count, 1);
}

#[tokio::test]
async fn listing_marker_save_rejects_active_listing_without_anchor() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-tile-2", "marker-tile-2@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let mut listing = make_listing(
        owner.clone(),
        "1111010100100080000",
        "Missing anchor listing",
    );

    repo.save(&listing, test_ctx()).await.unwrap();
    listing.submit_for_review(Utc::now()).unwrap();
    repo.save(
        &listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-submit", "submit_for_review"),
    )
    .await
    .unwrap();
    listing.approve(Utc::now()).unwrap();

    let err = repo
        .save(
            &listing,
            MutationContext::new_user_action(
                owner.clone(),
                "corr-marker-approve",
                "approve_listing",
            ),
        )
        .await
        .unwrap_err();

    let message = err.to_string();
    assert!(message.contains("missing PNU anchor"));
}

#[tokio::test]
async fn listing_marker_tile_validation_rejects_out_of_range_coordinates() {
    let _guard = lock_marker_tile_tests().await;
    assert!(ListingMarkerTileQuery::try_new(25, 0, 0, ListingMarkerFilter::AllActive).is_err());
    assert!(ListingMarkerTileQuery::try_new(4, 16, 0, ListingMarkerFilter::AllActive).is_err());
    assert!(ListingMarkerTileQuery::try_new(4, 0, 16, ListingMarkerFilter::AllActive).is_err());
}
