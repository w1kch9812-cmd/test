use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_role;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::Utc;
use listing_domain::entity::Listing;
use serde::Serialize;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{from_listing_error, from_listing_repo_error, problem, ProblemResponse};

use super::super::state::ListingsState;

// ─────────────────────────────────────────────────────────────────────────
// 상태 전이 (SP6-iv T5)
// ─────────────────────────────────────────────────────────────────────────

/// 상태 전이 응답 — id + new version.
#[derive(Debug, Serialize)]
pub struct TransitionResponse {
    /// 매물 ID.
    pub id: String,
    /// 전이 후 version.
    pub version: i64,
    /// 전이 후 status (`snake_case`).
    pub status: String,
}

/// `POST /listings/:id/submit-for-review` — Draft → PendingReview (Broker 전용).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn submit_for_review(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<TransitionResponse>, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let mut listing = load_listing_for_actor(&state, &auth, &id).await?;
    listing
        .submit_for_review(Utc::now())
        .map_err(|e| from_listing_error(&e))?;

    let ctx = http_user_action(&auth, "submit_for_review");
    state.listing_repo.save(&listing, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "listing save (submit) failed");
        from_listing_repo_error(&e)
    })?;

    Ok(Json(TransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

/// `POST /listings/:id/revise` — Rejected → Draft (Broker 전용).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn revise(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<TransitionResponse>, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let mut listing = load_listing_for_actor(&state, &auth, &id).await?;
    listing
        .revise_after_rejection(Utc::now())
        .map_err(|e| from_listing_error(&e))?;

    let ctx = http_user_action(&auth, "revise_listing");
    state.listing_repo.save(&listing, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "listing save (revise) failed");
        from_listing_repo_error(&e)
    })?;

    Ok(Json(TransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

/// 공통 — listing id parse + find + ownership check.
pub(in crate::routes::listings) async fn load_listing_for_actor(
    state: &ListingsState,
    auth: &AuthenticatedUser,
    id: &str,
) -> Result<Listing, ProblemResponse> {
    let listing_id = Id::<ListingIdMarker>::from_str(id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let listing = state
        .listing_repo
        .find(&listing_id)
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
        })?;
    if listing.owner_id != auth.user.id {
        return Err(problem(
            "forbidden",
            "본인 매물만 조작할 수 있어요",
            StatusCode::FORBIDDEN,
            None,
        ));
    }
    Ok(listing)
}
