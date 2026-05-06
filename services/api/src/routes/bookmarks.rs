//! `/listings/:id/bookmark` 와 `/me/bookmarks` 핸들러 (SP6-iii).
//!
//! 멱등 design — POST 가 UPSERT, DELETE 는 `NotFound` 무시 (200). 재시도 안전.

use std::str::FromStr;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use bookmark_domain::listing::BookmarkListing;
use bookmark_domain::repository::{BookmarkRepository, RepoError as BookmarkRepoError};
use chrono::{DateTime, Utc};
use listing_domain::repository::ListingRepository;
use notification_domain::entity::Notification;
use notification_domain::kind::NotificationKind;
use notification_domain::repository::NotificationRepository;
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, NotificationMarker};

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{problem, ProblemResponse};

/// 핸들러 공유 상태.
///
/// SP6-v: `listing_repo` + `notification_repo` 추가 — 본인 매물 아닌 경우
/// owner 에게 `listing_bookmarked` notification trigger. 모든 필드가 `_repo`
/// suffix 인 것은 의도된 layered repository pattern (axum State 가 한 번에
/// 주입 가능).
#[allow(clippy::struct_field_names)]
#[derive(Clone)]
pub struct BookmarksState {
    /// `BookmarkRepository` 구현체 (SP5-ii `PgBookmarkRepository`).
    pub bookmark_repo: Arc<dyn BookmarkRepository>,
    /// `ListingRepository` — bookmark 발생 시 owner 조회 (SP6-v).
    pub listing_repo: Arc<dyn ListingRepository>,
    /// `NotificationRepository` — `listing_bookmarked` 알림 INSERT (SP6-v).
    pub notification_repo: Arc<dyn NotificationRepository>,
}

/// `POST /listings/:id/bookmark` 요청 본문 (선택적 메모).
#[derive(Debug, Default, Deserialize)]
pub struct ToggleBookmarkRequest {
    /// 사용자 메모 (≤500자, 선택).
    #[serde(default)]
    pub note: Option<String>,
}

/// `POST /listings/:id/bookmark` 응답.
#[derive(Debug, Serialize)]
pub struct ToggleBookmarkResponse {
    /// 매물 ID.
    pub listing_id: String,
    /// 즐겨찾기 등록 시각.
    pub created_at: DateTime<Utc>,
}

/// `POST /listings/:id/bookmark` — 즐겨찾기 추가 (멱등 UPSERT).
///
/// 같은 `(user_id, listing_id)` 두 번째 호출 = `note` 갱신 (UPSERT).
#[tracing::instrument(skip(state, auth, body), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn toggle_bookmark(
    State(state): State<BookmarksState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(body): Json<ToggleBookmarkRequest>,
) -> Result<(StatusCode, Json<ToggleBookmarkResponse>), ProblemResponse> {
    let listing_id = Id::<ListingIdMarker>::from_str(&id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let now = Utc::now();
    let bm = BookmarkListing::try_new(auth.user.id.clone(), listing_id.clone(), body.note, now)
        .map_err(|e| {
            problem(
                "validation",
                "북마크 메모가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;

    let ctx = http_user_action(&auth, "bookmark_listing");
    state
        .bookmark_repo
        .save_listing_bookmark(&bm, ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "bookmark save failed");
            from_bookmark_repo_error(&e)
        })?;

    // SP6-v: listing.owner != bookmarker 면 broker 에게 알림 (best-effort).
    // owner 본인이 본인 매물 북마크 시 skip (자기 알림 노이즈 차단).
    match state.listing_repo.find(&listing_id).await {
        Ok(Some(listing)) if listing.owner_id != auth.user.id => {
            let notif = Notification::new(
                Id::<NotificationMarker>::new(),
                listing.owner_id.clone(),
                NotificationKind::ListingBookmarked,
                serde_json::json!({
                    "listing_id": listing.id.as_str(),
                    "title": listing.title.as_str(),
                    "bookmarker_id": auth.user.id.as_str(),
                    "bookmarker_name": auth.user.display_name,
                }),
                now,
            );
            let notif_ctx = http_user_action(&auth, "notify_listing_bookmarked");
            if let Err(e) = state.notification_repo.insert(&notif, notif_ctx).await {
                tracing::warn!(error = %e, "bookmark notification insert failed — proceeding");
            }
        }
        Ok(_) => {} // 본인 매물 또는 미존재 -- skip
        Err(e) => tracing::warn!(error = %e, "listing find for notification failed — proceeding"),
    }

    Ok((
        StatusCode::CREATED,
        Json(ToggleBookmarkResponse {
            listing_id: listing_id.as_str().to_owned(),
            created_at: now,
        }),
    ))
}

/// `DELETE /listings/:id/bookmark` — 해제 (멱등). 이미 없으면 200 반환.
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn delete_bookmark(
    State(state): State<BookmarksState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, ProblemResponse> {
    let listing_id = Id::<ListingIdMarker>::from_str(&id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let ctx = http_user_action(&auth, "delete_bookmark_listing");
    match state
        .bookmark_repo
        .delete_listing_bookmark(&auth.user.id, &listing_id, ctx)
        .await
    {
        // 멱등 — 이미 없으면 200 (NotFound 도 success 로 매핑).
        Ok(()) | Err(BookmarkRepoError::NotFound) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!(error = %e, "bookmark delete failed");
            Err(from_bookmark_repo_error(&e))
        }
    }
}

/// `GET /me/bookmarks` 응답.
#[derive(Debug, Serialize)]
pub struct MyBookmarksResponse {
    /// 사용자가 즐겨찾기 한 매물 목록.
    pub listings: Vec<BookmarkListingItem>,
}

/// `BookmarkListing` 응답 단건.
#[derive(Debug, Serialize)]
pub struct BookmarkListingItem {
    /// 매물 ID.
    pub listing_id: String,
    /// 메모 (선택).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 즐겨찾기 등록 시각.
    pub created_at: DateTime<Utc>,
}

/// `GET /me/bookmarks` — 내 매물 즐겨찾기 목록 (SP6-iii 1차 = listing 만).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id))]
pub async fn list_my_bookmarks(
    State(state): State<BookmarksState>,
    Extension(auth): Extension<AuthenticatedUser>,
) -> Result<Json<MyBookmarksResponse>, ProblemResponse> {
    let bms = state
        .bookmark_repo
        .find_listing_bookmarks(&auth.user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "bookmark find failed");
            from_bookmark_repo_error(&e)
        })?;

    let listings = bms
        .into_iter()
        .map(|bm| BookmarkListingItem {
            listing_id: bm.listing_id.as_str().to_owned(),
            note: bm.note,
            created_at: bm.created_at,
        })
        .collect();

    Ok(Json(MyBookmarksResponse { listings }))
}

fn from_bookmark_repo_error(e: &BookmarkRepoError) -> ProblemResponse {
    match e {
        BookmarkRepoError::NotFound => problem(
            "not-found",
            "북마크를 찾을 수 없어요",
            StatusCode::NOT_FOUND,
            None,
        ),
        BookmarkRepoError::Database(_) => problem(
            "internal-error",
            "내부 서버 오류",
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
        ),
    }
}
