use chrono::Utc;
use db::listing::PgListingRepository;
use listing_domain::repository::ListingRepository;
use shared_kernel::mutation::MutationContext;
use sqlx::Row;

use super::{
    activate_listing, lock_marker_tile_tests, make_listing, seed_anchor, seed_owner,
    setup_test_pool, truncate_all,
};

#[tokio::test]
async fn listing_marker_overlay_tables_exist_with_expected_columns() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;

    let rows = sqlx::query(
        r"
        select table_name, column_name
        from information_schema.columns
        where table_name in (
            'listing_marker_tombstone_log',
            'listing_marker_delta_log',
            'listing_marker_dirty_tile_queue'
        )
        order by table_name, ordinal_position
        ",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let columns = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("table_name"),
                row.get::<String, _>("column_name"),
            )
        })
        .collect::<Vec<_>>();

    assert!(columns.contains(&(
        "listing_marker_tombstone_log".to_owned(),
        "marker_id".to_owned()
    )));
    assert!(columns.contains(&(
        "listing_marker_delta_log".to_owned(),
        "marker_id".to_owned()
    )));
    assert!(columns.contains(&(
        "listing_marker_dirty_tile_queue".to_owned(),
        "tile_z".to_owned()
    )));
}

#[tokio::test]
async fn listing_marker_projection_upsert_uses_platform_core_anchor_snapshot() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-projection-1",
        "marker-projection-1@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100090000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Projection listing");
    activate_listing(&repo, &mut listing, &owner).await;

    repo.upsert_listing_marker_projection(&listing.id)
        .await
        .unwrap();

    let row = sqlx::query(
        r"
        select
            marker_id,
            listing_id,
            pnu,
            anchor_snapshot_id,
            source_geometry_version,
            source_geometry_checksum_sha256,
            source_listing_version,
            listing_status,
            listing_type,
            transaction_type,
            price_krw,
            area_m2::text as area_m2
        from listing_marker_projection
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        row.get::<String, _>("marker_id"),
        format!("lm_{}", listing.id.as_str())
    );
    assert_eq!(row.get::<String, _>("listing_id"), listing.id.as_str());
    assert_eq!(row.get::<String, _>("pnu"), pnu);
    assert_eq!(
        row.get::<String, _>("anchor_snapshot_id"),
        "snapshot-test-v1"
    );
    assert_eq!(
        row.get::<String, _>("source_geometry_version"),
        "test-geometry-v1"
    );
    assert_eq!(
        row.get::<String, _>("source_geometry_checksum_sha256"),
        "a".repeat(64)
    );
    assert_eq!(row.get::<i64, _>("source_listing_version"), listing.version);
    assert_eq!(row.get::<String, _>("listing_status"), "active");
    assert_eq!(row.get::<String, _>("listing_type"), "factory");
    assert_eq!(row.get::<String, _>("transaction_type"), "sale");
    assert_eq!(row.get::<i64, _>("price_krw"), 500_000_000);
    assert_eq!(row.get::<String, _>("area_m2"), "330.58");
}

#[tokio::test]
async fn listing_marker_projection_is_created_when_active_listing_is_saved() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-projection-auto",
        "marker-projection-auto@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100140000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Auto projection listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let row = sqlx::query(
        r"
        select
            listing_status,
            visibility_scope,
            source_listing_version,
            projection_version
        from listing_marker_projection
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("listing_status"), "active");
    assert_eq!(row.get::<String, _>("visibility_scope"), "public");
    assert_eq!(row.get::<i64, _>("source_listing_version"), listing.version);
    assert_eq!(row.get::<i64, _>("projection_version"), 1);
}

#[tokio::test]
async fn listing_marker_projection_is_hidden_when_listing_leaves_active_state() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-projection-hidden",
        "marker-projection-hidden@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100150000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Hidden projection listing");
    activate_listing(&repo, &mut listing, &owner).await;
    listing.mark_sold(Utc::now()).unwrap();
    repo.save(
        &listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-sold", "mark_sold"),
    )
    .await
    .unwrap();

    let row = sqlx::query(
        r"
        select
            listing_status,
            visibility_scope,
            source_listing_version,
            projection_version
        from listing_marker_projection
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("listing_status"), "sold");
    assert_eq!(row.get::<String, _>("visibility_scope"), "owner_private");
    assert_eq!(row.get::<i64, _>("source_listing_version"), listing.version);
    assert_eq!(row.get::<i64, _>("projection_version"), 2);
}

#[tokio::test]
async fn listing_marker_projection_writes_tombstone_when_public_listing_becomes_sold() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(
        &pool,
        "zsub-marker-tombstone",
        "marker-tombstone@example.com",
    )
    .await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100160000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Tombstone listing");
    activate_listing(&repo, &mut listing, &owner).await;

    listing.mark_sold(Utc::now()).unwrap();
    repo.save(
        &listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-sold", "mark_sold"),
    )
    .await
    .unwrap();

    let row = sqlx::query(
        r"
        select marker_id, reason, projection_version
        from listing_marker_tombstone_log
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        row.get::<String, _>("marker_id"),
        format!("lm_{}", listing.id.as_str())
    );
    assert_eq!(row.get::<String, _>("reason"), "sold");
    assert_eq!(row.get::<i64, _>("projection_version"), 2);
}

#[tokio::test]
async fn listing_marker_projection_writes_delta_when_listing_becomes_public() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-delta", "marker-delta@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100170000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Delta listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let row = sqlx::query(
        r"
        select marker_id, change_kind, projection_version
        from listing_marker_delta_log
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        row.get::<String, _>("marker_id"),
        format!("lm_{}", listing.id.as_str())
    );
    assert_eq!(row.get::<String, _>("change_kind"), "became_public");
    assert_eq!(row.get::<i64, _>("projection_version"), 1);
}

#[tokio::test]
async fn listing_marker_projection_enqueues_dirty_tiles_for_public_change() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-dirty", "marker-dirty@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100200000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Dirty tile listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let rows = sqlx::query(
        r"
        select tile_z, status
        from listing_marker_dirty_tile_queue
        where status = 'pending'
        order by tile_z
        ",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let zooms = rows
        .iter()
        .map(|row| row.get::<i32, _>("tile_z"))
        .collect::<Vec<_>>();

    assert_eq!(zooms, vec![0, 6, 10, 11, 12, 13, 14]);
    assert!(rows
        .iter()
        .all(|row| row.get::<String, _>("status") == "pending"));
}
