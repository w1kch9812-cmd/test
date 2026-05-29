//! `/admin/listings/:id/{approve,reject}` 핸들러 (SP6-v).
//!
//! Admin / Operator 전용 매물 승인/반려. 도메인 transition 후 broker 에게
//! `listing_approved` / `listing_rejected` notification INSERT (best-effort
//! multi-tx — spec § 5).

use std::str::FromStr;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_one_of_roles;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::Utc;
use listing_domain::repository::ListingRepository;
use notification_domain::entity::Notification;
use notification_domain::kind::NotificationKind;
use notification_domain::repository::NotificationRepository;
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, NotificationMarker};
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{from_listing_error, from_listing_repo_error, problem, ProblemResponse};

/// admin listings 핸들러 공유 상태.
#[derive(Clone)]
pub struct AdminListingsState {
    /// `Listing` 저장소.
    pub listing_repo: Arc<dyn ListingRepository>,
    /// `Notification` 저장소 — 승인/반려 알림 INSERT.
    pub notification_repo: Arc<dyn NotificationRepository>,
}

/// 상태 전이 응답 (이미 listings.rs 의 `TransitionResponse` 와 shape 동일하지만
/// route 분리 + admin scope 명확화 위해 별도 정의).
#[derive(Debug, Serialize)]
pub struct AdminTransitionResponse {
    /// 매물 ID.
    pub id: String,
    /// 전이 후 version.
    pub version: i64,
    /// 전이 후 status (`snake_case`).
    pub status: String,
}

/// `POST /admin/listings/:id/approve` — 매물 승인 (Admin / Operator 전용).
///
/// 1. RBAC: Admin 또는 Operator
/// 2. listing find + `Listing::approve(now)` (도메인 가드: `PendingReview` only)
/// 3. `listing_repo.save` (audit + outbox 같은 tx)
/// 4. notification.insert (`listing_approved`, broker 수신, best-effort)
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn approve_listing(
    State(state): State<AdminListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<AdminTransitionResponse>, ProblemResponse> {
    require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).map_err(|_| {
        problem(
            "forbidden",
            "admin 또는 operator 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let listing_id = parse_listing_id(&id)?;
    let mut listing = load_listing(&state, &listing_id).await?;

    let now = Utc::now();
    listing.approve(now).map_err(|e| from_listing_error(&e))?;

    state
        .listing_repo
        .save(&listing, http_user_action(&auth, "approve_listing"))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing save (approve) failed");
            from_listing_repo_error(&e)
        })?;

    // best-effort notification (multi-tx, spec § 5 — single-tx 는 SP6-v-2)
    let notif = Notification::new(
        Id::<NotificationMarker>::new(),
        listing.owner_id.clone(),
        NotificationKind::ListingApproved,
        serde_json::json!({
            "listing_id": listing.id.as_str(),
            "title": listing.title.as_str(),
        }),
        now,
    );
    let notif_ctx = http_user_action(&auth, "notify_listing_approved");
    if let Err(e) = state.notification_repo.insert(&notif, notif_ctx).await {
        tracing::warn!(error = %e, "notification insert failed — proceeding (best-effort)");
    }

    Ok(Json(AdminTransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

/// `POST /admin/listings/:id/reject` 요청 본문.
#[derive(Debug, Deserialize)]
pub struct RejectListingRequest {
    /// 반려 사유 (1-500자, broker 가 다음 단계에서 보게 됨).
    pub reason: String,
}

/// `POST /admin/listings/:id/reject` — 매물 반려 (Admin / Operator 전용).
///
/// reason 은 notification.payload 에 보존 — broker 가 알림 클릭 시 표시.
#[tracing::instrument(skip(state, auth, body), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn reject_listing(
    State(state): State<AdminListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(body): Json<RejectListingRequest>,
) -> Result<Json<AdminTransitionResponse>, ProblemResponse> {
    require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).map_err(|_| {
        problem(
            "forbidden",
            "admin 또는 operator 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let reason = body.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(problem(
            "validation",
            "reason 은 1-500자여야 해요",
            StatusCode::BAD_REQUEST,
            Some(format!("got {} chars", reason.chars().count())),
        ));
    }

    let listing_id = parse_listing_id(&id)?;
    let mut listing = load_listing(&state, &listing_id).await?;

    let now = Utc::now();
    listing.reject(now).map_err(|e| from_listing_error(&e))?;

    state
        .listing_repo
        .save(&listing, http_user_action(&auth, "reject_listing"))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing save (reject) failed");
            from_listing_repo_error(&e)
        })?;

    let notif = Notification::new(
        Id::<NotificationMarker>::new(),
        listing.owner_id.clone(),
        NotificationKind::ListingRejected,
        serde_json::json!({
            "listing_id": listing.id.as_str(),
            "title": listing.title.as_str(),
            "reason": reason,
        }),
        now,
    );
    let notif_ctx = http_user_action(&auth, "notify_listing_rejected");
    if let Err(e) = state.notification_repo.insert(&notif, notif_ctx).await {
        tracing::warn!(error = %e, "notification insert failed — proceeding (best-effort)");
    }

    Ok(Json(AdminTransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

fn parse_listing_id(id: &str) -> Result<Id<ListingIdMarker>, ProblemResponse> {
    Id::<ListingIdMarker>::from_str(id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })
}

async fn load_listing(
    state: &AdminListingsState,
    listing_id: &Id<ListingIdMarker>,
) -> Result<listing_domain::entity::Listing, ProblemResponse> {
    state
        .listing_repo
        .find(listing_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing find failed");
            from_listing_repo_error(&e)
        })?
        .ok_or_else(|| {
            problem(
                "not-found",
                "매물을 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            )
        })
}
