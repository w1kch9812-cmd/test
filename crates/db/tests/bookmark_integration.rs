//! `PgBookmarkRepository` 통합 테스트 (SP5-ii) — composite PK + polymorphic
//! external + audit/outbox 검증.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::similar_names
)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use bookmark_domain::external::BookmarkExternal;
use bookmark_domain::external_kind::BookmarkExternalKind;
use bookmark_domain::listing::BookmarkListing;
use bookmark_domain::repository::{BookmarkRepository, RepoError as BmRepoError};
use chrono::{DateTime, Utc};
use db::bookmark::PgBookmarkRepository;
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::ListingPhotoRepository;
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::email::Email;
use shared_kernel::id::{
    BookmarkExternalMarker, Id, ListingMarker, ListingPhotoMarker, UserMarker,
};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

/// 테스트 이벤트 — outbox 검증용.
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

async fn seed_user(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let user = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let user_id = user.id.clone();
    repo.save(&user, test_ctx()).await.unwrap();
    user_id
}

async fn seed_listing(pool: &sqlx::PgPool, owner: Id<UserMarker>) -> Id<ListingMarker> {
    let repo = PgListingRepository::new(pool.clone());
    let now = Utc::now();
    let listing = Listing::try_new_draft(
        Id::new(),
        owner,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(100_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(100.00).unwrap(),
        ListingTitle::try_new("bookmark test").unwrap(),
        Description::try_new("").unwrap(),
        now,
    )
    .expect("listing");
    let listing_id = listing.id.clone();
    repo.save(&listing, test_ctx()).await.unwrap();
    listing_id
}

#[tokio::test]
async fn round_trip_listing_bookmark_with_audit() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-1", "bm1@example.com").await;
    let listing_id = seed_listing(&pool, user_id.clone()).await;
    let repo = PgBookmarkRepository::new(pool.clone());

    let bm = BookmarkListing::try_new(
        user_id.clone(),
        listing_id.clone(),
        Some("관심".to_owned()),
        Utc::now(),
    )
    .expect("bookmark");
    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-bm-1", "create_bookmark");
    repo.save_listing_bookmark(&bm, ctx).await.expect("save");

    let bookmarks = repo.find_listing_bookmarks(&user_id).await.expect("find");
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].note.as_deref(), Some("관심"));

    // audit_log row 1 + resource_id = listing_id
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'bookmark_listing' and resource_id = $1",
    )
    .bind(listing_id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn round_trip_external_bookmark_polymorphic() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-2", "bm2@example.com").await;
    let repo = PgBookmarkRepository::new(pool.clone());

    let bm = BookmarkExternal::try_new(
        Id::<BookmarkExternalMarker>::new(),
        user_id.clone(),
        BookmarkExternalKind::CourtAuction,
        "2024타경12345",
        Some("관심 경매".to_owned()),
        Utc::now(),
    )
    .expect("external bookmark");
    let bm_id = bm.id.clone();
    let ctx =
        MutationContext::new_user_action(user_id.clone(), "corr-bm-2", "create_external_bookmark");
    repo.save_external_bookmark(&bm, ctx).await.expect("save");

    let externals = repo.find_external_bookmarks(&user_id).await.expect("find");
    assert_eq!(externals.len(), 1);
    assert_eq!(externals[0].target_kind, BookmarkExternalKind::CourtAuction);
    assert_eq!(externals[0].target_id, "2024타경12345");

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'bookmark_external' and resource_id = $1",
    )
    .bind(bm_id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn delete_listing_bookmark_audit_logs() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-3", "bm3@example.com").await;
    let listing_id = seed_listing(&pool, user_id.clone()).await;
    let repo = PgBookmarkRepository::new(pool.clone());

    let bm = BookmarkListing::try_new(user_id.clone(), listing_id.clone(), None, Utc::now())
        .expect("bookmark");
    repo.save_listing_bookmark(&bm, test_ctx())
        .await
        .expect("save");

    let delete_ctx = MutationContext::new_user_action(user_id.clone(), "corr-bm-3", "delete");
    repo.delete_listing_bookmark(&user_id, &listing_id, delete_ctx)
        .await
        .expect("delete");

    let bookmarks = repo.find_listing_bookmarks(&user_id).await.expect("find");
    assert_eq!(bookmarks.len(), 0);

    // delete audit row 1개 (action='delete')
    let delete_audit: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'bookmark_listing' and action = 'delete'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(delete_audit.0, 1);
}

#[tokio::test]
async fn delete_listing_bookmark_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-4", "bm4@example.com").await;
    let listing_id = seed_listing(&pool, user_id.clone()).await;
    let repo = PgBookmarkRepository::new(pool);

    let err = repo
        .delete_listing_bookmark(&user_id, &listing_id, test_ctx())
        .await
        .unwrap_err();
    assert!(matches!(err, BmRepoError::NotFound));
}

#[tokio::test]
async fn save_listing_bookmark_with_events_inserts_outbox() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-5", "bm5@example.com").await;
    let listing_id = seed_listing(&pool, user_id.clone()).await;
    let repo = PgBookmarkRepository::new(pool.clone());

    let bm = BookmarkListing::try_new(user_id.clone(), listing_id.clone(), None, Utc::now())
        .expect("bookmark");
    let event: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "bookmark.created",
        aggregate_id: listing_id.as_str().to_owned(),
        payload: serde_json::json!({"by": "test"}),
        occurred_at: Utc::now(),
    });
    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-bm-5", "create_bookmark")
        .with_events(vec![event]);
    repo.save_listing_bookmark(&bm, ctx).await.expect("save");

    let outbox: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'bookmark_listing' and aggregate_id = $1",
    )
    .bind(listing_id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outbox.0, 1);
}

#[tokio::test]
async fn upsert_listing_bookmark_updates_note() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-bm-6", "bm6@example.com").await;
    let listing_id = seed_listing(&pool, user_id.clone()).await;
    let repo = PgBookmarkRepository::new(pool);

    let bm1 = BookmarkListing::try_new(
        user_id.clone(),
        listing_id.clone(),
        Some("first".to_owned()),
        Utc::now(),
    )
    .expect("bm1");
    repo.save_listing_bookmark(&bm1, test_ctx())
        .await
        .expect("first");

    let bm2 = BookmarkListing::try_new(
        user_id.clone(),
        listing_id.clone(),
        Some("updated".to_owned()),
        Utc::now(),
    )
    .expect("bm2");
    repo.save_listing_bookmark(&bm2, test_ctx())
        .await
        .expect("upsert");

    let bookmarks = repo.find_listing_bookmarks(&user_id).await.expect("find");
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].note.as_deref(), Some("updated"));
}

// ── SP6-iii: find_detail_by_id + JOIN bookmark_count + is_bookmarked ────────

#[tokio::test]
async fn find_detail_active_returns_full_data_with_zero_bookmarks() {
    use listing_domain::repository::ListingRepository as ListingRepo;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-1", "detail1@example.com").await;
    let viewer = seed_user(&pool, "zsub-detail-1v", "detail1v@example.com").await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());

    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");
    assert_eq!(detail.listing.id, listing_id);
    assert_eq!(detail.bookmark_count, 0);
    assert!(!detail.is_bookmarked);
    assert!(detail.photos.is_empty()); // 사진 미등록
}

#[tokio::test]
async fn find_detail_with_viewer_bookmark_sets_is_bookmarked_true() {
    use listing_domain::repository::ListingRepository as ListingRepo;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-2", "detail2@example.com").await;
    let viewer = seed_user(&pool, "zsub-detail-2v", "detail2v@example.com").await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());
    let bm_repo = PgBookmarkRepository::new(pool.clone());

    // viewer 가 북마크 한다.
    let bm =
        BookmarkListing::try_new(viewer.clone(), listing_id.clone(), None, Utc::now()).expect("bm");
    bm_repo
        .save_listing_bookmark(&bm, test_ctx())
        .await
        .expect("save");

    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");
    assert_eq!(detail.bookmark_count, 1);
    assert!(detail.is_bookmarked);
}

#[tokio::test]
async fn find_detail_other_users_bookmark_does_not_set_is_bookmarked() {
    use listing_domain::repository::ListingRepository as ListingRepo;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-3", "detail3@example.com").await;
    let other = seed_user(&pool, "zsub-detail-3o", "detail3o@example.com").await;
    let viewer = seed_user(&pool, "zsub-detail-3v", "detail3v@example.com").await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());
    let bm_repo = PgBookmarkRepository::new(pool.clone());

    // 다른 사용자 (other) 가 북마크 한다.
    let bm =
        BookmarkListing::try_new(other.clone(), listing_id.clone(), None, Utc::now()).expect("bm");
    bm_repo
        .save_listing_bookmark(&bm, test_ctx())
        .await
        .expect("save");

    // viewer 시점 — bookmark_count 1 이지만 is_bookmarked 는 false (본인 X).
    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");
    assert_eq!(detail.bookmark_count, 1);
    assert!(!detail.is_bookmarked);
}

#[tokio::test]
async fn find_detail_nonexistent_returns_none() {
    use listing_domain::repository::ListingRepository as ListingRepo;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let viewer = seed_user(&pool, "zsub-detail-4v", "detail4v@example.com").await;
    let l_repo = PgListingRepository::new(pool.clone());

    let fake_id = Id::<ListingMarker>::new();
    let result = l_repo
        .find_detail_by_id(&fake_id, &viewer)
        .await
        .expect("ok");
    assert!(result.is_none());
}

#[tokio::test]
async fn find_detail_excludes_pending_upload_photos() {
    use listing_domain::repository::ListingRepository as ListingRepo;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_user(&pool, "zsub-detail-pending", "detail-pending@example.com").await;
    let viewer = seed_user(
        &pool,
        "zsub-detail-pending-v",
        "detail-pending-v@example.com",
    )
    .await;
    let listing_id = seed_active_listing(&pool, owner).await;
    let l_repo = PgListingRepository::new(pool.clone());
    let p_repo = PgListingPhotoRepository::new(pool.clone());
    let pending = ListingPhoto::try_new(
        Id::<ListingPhotoMarker>::new(),
        listing_id.clone(),
        "listings/test/pending-detail-photo.jpg",
        None,
        None,
        0,
        None,
        None,
        None,
        PhotoContentType::Jpeg,
        Utc::now(),
    )
    .expect("pending photo");
    p_repo.save(&pending, test_ctx()).await.expect("save");

    let detail = l_repo
        .find_detail_by_id(&listing_id, &viewer)
        .await
        .expect("ok")
        .expect("found");

    assert!(detail.photos.is_empty());
}

#[path = "bookmark_integration/detail_photo.rs"]
mod detail_photo;

#[path = "bookmark_integration/view_count.rs"]
mod view_count;

/// 도우미 — Active 상태 매물 시드 (`find_detail` RBAC 테스트용).
async fn seed_active_listing(pool: &sqlx::PgPool, owner: Id<UserMarker>) -> Id<ListingMarker> {
    let repo = PgListingRepository::new(pool.clone());
    let now = Utc::now();
    let mut listing = Listing::try_new_draft(
        Id::new(),
        owner,
        Pnu::try_new("1111010100100090000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(200_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(150.0).unwrap(),
        ListingTitle::try_new("active listing for detail test").unwrap(),
        Description::try_new("").unwrap(),
        now,
    )
    .expect("listing");
    listing.submit_for_review(now).expect("submit");
    listing.approve(now).expect("approve");
    let listing_id = listing.id.clone();
    repo.save(&listing, test_ctx()).await.expect("save");
    listing_id
}
