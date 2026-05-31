//! Integration tests for Gongzzang-owned listing marker PBF tiles.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::OnceLock;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
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
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};
use tokio::sync::{Mutex, MutexGuard};

static MARKER_TILE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const SEOUL_Z11_X: u32 = 1746;
const SEOUL_Z11_Y: u32 = 793;
const SEOUL_Z14_X: u32 = 13970;
const SEOUL_Z14_Y: u32 = 6344;

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
#[path = "listing_marker_tile_integration/projection.rs"]
mod projection;
#[path = "listing_marker_tile_integration/tiles.rs"]
mod tiles;
