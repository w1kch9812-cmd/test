use db::listing::PgListingRepository;
use listing_domain::repository::{
    ListingMarkerFilter, ListingMarkerFilterSpec, ListingMarkerMaskEncoding,
    ListingMarkerMaskQuery, ListingRepository,
};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;

use super::{
    activate_listing, lock_marker_tile_tests, make_listing, seed_anchor, seed_owner,
    setup_test_pool, truncate_all,
};

#[tokio::test]
async fn listing_marker_count_applies_price_area_and_type_filters() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-count", "marker-count@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100110000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Count listing");
    activate_listing(&repo, &mut listing, &owner).await;
    repo.upsert_listing_marker_projection(&listing.id)
        .await
        .unwrap();

    let matching_filter = ListingMarkerFilterSpec {
        types: vec![ListingType::Factory],
        transactions: vec![TransactionType::Sale],
        min_area_m2: Some(300),
        max_area_m2: Some(400),
        min_price_krw: Some(100_000_000),
        max_price_krw: Some(900_000_000),
    }
    .try_normalized()
    .expect("matching filter");
    let count = repo.count_listing_markers(matching_filter).await.unwrap();
    assert_eq!(count.total_count, 1);
    assert_eq!(count.projection_version, Some(1));
    assert_eq!(
        count.anchor_snapshot_id.as_deref(),
        Some("snapshot-test-v1")
    );

    let non_matching_filter = ListingMarkerFilterSpec {
        types: vec![ListingType::Warehouse],
        transactions: vec![TransactionType::Sale],
        min_area_m2: Some(300),
        max_area_m2: Some(400),
        min_price_krw: Some(100_000_000),
        max_price_krw: Some(900_000_000),
    }
    .try_normalized()
    .expect("non-matching filter");
    let count = repo
        .count_listing_markers(non_matching_filter)
        .await
        .unwrap();
    assert_eq!(count.total_count, 0);
}

#[tokio::test]
async fn listing_marker_filter_registry_round_trips_normalized_filter() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingRepository::new(pool);

    let filter = ListingMarkerFilterSpec {
        types: vec![ListingType::Warehouse, ListingType::Factory],
        transactions: vec![TransactionType::Sale],
        min_area_m2: Some(300),
        max_area_m2: Some(1000),
        min_price_krw: Some(100_000_000),
        max_price_krw: Some(5_000_000_000),
    }
    .try_normalized()
    .expect("normalized filter");

    let registered = repo
        .register_listing_marker_filter(filter.clone())
        .await
        .unwrap();
    assert_eq!(registered.filter_hash, filter.filter_hash());

    let resolved = repo
        .resolve_listing_marker_filter(&registered.filter_hash)
        .await
        .unwrap()
        .expect("registered filter");
    assert_eq!(resolved, filter);
}

#[tokio::test]
async fn listing_marker_mask_returns_show_ids_for_loaded_tile() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-mask", "marker-mask@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100130000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Mask listing");
    activate_listing(&repo, &mut listing, &owner).await;
    repo.upsert_listing_marker_projection(&listing.id)
        .await
        .unwrap();

    let mask = repo
        .find_listing_marker_mask(ListingMarkerMaskQuery {
            z: 0,
            x: 0,
            y: 0,
            filter: ListingMarkerFilter::AllActive,
            base_version: None,
        })
        .await
        .unwrap();

    assert_eq!(mask.encoding, ListingMarkerMaskEncoding::Show);
    assert_eq!(mask.marker_ids, vec![format!("lm_{}", listing.id.as_str())]);
    assert_eq!(mask.projection_version, Some(1));
    assert_eq!(mask.anchor_snapshot_id.as_deref(), Some("snapshot-test-v1"));
}
