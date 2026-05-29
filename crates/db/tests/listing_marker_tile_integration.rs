//! Integration tests for Gongzzang-owned listing marker PBF tiles.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::OnceLock;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::{
    ListingMarkerFilter, ListingMarkerFilterSpec, ListingMarkerTileQuery, ListingRepository,
};
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use sqlx::Row;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};
use tokio::sync::{Mutex, MutexGuard};

static MARKER_TILE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

async fn lock_marker_tile_tests() -> MutexGuard<'static, ()> {
    MARKER_TILE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}

async fn seed_owner(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Marker Tile Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let owner_id = owner.id.clone();
    repo.save(&owner, test_ctx()).await.unwrap();
    owner_id
}

fn make_listing_of_type(
    owner_id: Id<UserMarker>,
    pnu: &str,
    title: &str,
    listing_type: ListingType,
) -> Listing {
    Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        owner_id,
        Pnu::try_new(pnu).unwrap(),
        listing_type,
        TransactionType::Sale,
        MoneyKrw::try_new(500_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(330.58).unwrap(),
        ListingTitle::try_new(title).unwrap(),
        Description::try_new("marker tile test listing").unwrap(),
        Utc::now(),
    )
    .expect("listing")
}

fn make_listing(owner_id: Id<UserMarker>, pnu: &str, title: &str) -> Listing {
    make_listing_of_type(owner_id, pnu, title, ListingType::Factory)
}

async fn activate_listing(
    repo: &PgListingRepository,
    listing: &mut Listing,
    owner: &Id<UserMarker>,
) {
    repo.save(listing, test_ctx()).await.unwrap();

    listing.submit_for_review(Utc::now()).unwrap();
    repo.save(
        listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-submit", "submit_for_review"),
    )
    .await
    .unwrap();

    listing.approve(Utc::now()).unwrap();
    repo.save(
        listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-approve", "approve_listing"),
    )
    .await
    .unwrap();
}

async fn seed_anchor(pool: &sqlx::PgPool, pnu: &str) {
    sqlx::query(
        r"
        insert into parcel_marker_anchor (
            pnu,
            anchor_point,
            algorithm,
            algorithm_version,
            anchor_snapshot_id,
            source_geometry_version,
            source_geometry_checksum_sha256,
            platform_core_updated_at,
            synced_at
        )
        values (
            $1,
            ST_SetSRID(ST_MakePoint(126.9780, 37.5665), 4326),
            'polylabel',
            '1',
            'snapshot-test-v1',
            'test-geometry-v1',
            repeat('a', 64),
            now(),
            now()
        )
        ",
    )
    .bind(pnu)
    .execute(pool)
    .await
    .unwrap();
}

#[path = "listing_marker_tile_integration/filter_index.rs"]
mod filter_index;

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
            0,
            0,
            0,
            ListingMarkerFilter::Normalized(warehouse_only),
        ))
        .await
        .unwrap();

    assert_eq!(tile.eligible_count, 1);
    assert_eq!(tile.represented_count, 1);
    assert_eq!(tile.feature_count, 1);
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
            0,
            0,
            0,
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
            0,
            0,
            0,
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
