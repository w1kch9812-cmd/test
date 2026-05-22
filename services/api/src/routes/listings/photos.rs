use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_role;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use listing_photo_domain::entity::ListingPhoto;
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingPhotoMarker};
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{problem, ProblemResponse};

use super::mutation::load_listing_for_actor;
use super::state::ListingsState;

#[derive(Debug, Deserialize)]
pub struct RequestPhotoUploadRequest {
    /// 표시 순서 (≥0).
    pub display_order: i32,
    /// MIME content-type (`image/jpeg`/`image/png`/`image/webp`).
    pub content_type: String,
    /// 캡션 (≤200자, 선택).
    pub caption: Option<String>,
}

/// `POST /listings/:id/photos` 응답.
#[derive(Debug, Serialize)]
pub struct RequestPhotoUploadResponse {
    /// 새 사진 ID (`lph_<26 ULID>`).
    pub photo_id: String,
    /// pre-signed PUT URL — 1차 mock (SP4-iii-e R2 통합 전).
    pub presigned_put_url: String,
    /// R2 객체 키 (`listings/<lst_id>/<lph_id>.<ext>`).
    pub r2_key: String,
    /// URL 만료 시각 (mock 은 +15분).
    pub expires_at: DateTime<Utc>,
}

/// `POST /listings/:id/photos` — pre-signed URL 발급 (Broker + 소유자 전용).
///
/// 1차 mock: `presigned_put_url = "MOCK://..."`. SP4-iii-e 의 `aws-sdk-s3` 통합
/// 후 실 R2 URL 반환. `ListingPhoto` row 는 *지금* 생성됨 — frontend 가 PUT
/// 성공 시 별도 confirm endpoint 호출은 후속 (FU 49).
#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip(state, auth, body), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn request_photo_upload(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(body): Json<RequestPhotoUploadRequest>,
) -> Result<(StatusCode, Json<RequestPhotoUploadResponse>), ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let listing = load_listing_for_actor(&state, &auth, &id).await?;

    let content_type = listing_photo_domain::entity::PhotoContentType::from_str(&body.content_type)
        .map_err(|e| {
            problem(
                "validation",
                "content_type 가 유효하지 않아요 (image/jpeg, image/png, image/webp)",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;

    let photo_id = Id::<ListingPhotoMarker>::new();
    let ext = match content_type {
        listing_photo_domain::entity::PhotoContentType::Jpeg => "jpg",
        listing_photo_domain::entity::PhotoContentType::Png => "png",
        listing_photo_domain::entity::PhotoContentType::Webp => "webp",
    };
    let r2_key = format!(
        "listings/{}/{}.{ext}",
        listing.id.as_str(),
        photo_id.as_str()
    );

    let now = Utc::now();
    let photo = ListingPhoto::try_new(
        photo_id.clone(),
        listing.id.clone(),
        &r2_key,
        None,
        body.caption.as_deref(),
        body.display_order,
        None,
        None,
        None,
        content_type,
        now,
    )
    .map_err(|e| {
        problem(
            "validation",
            "사진 메타가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let ctx = http_user_action(&auth, "request_photo_upload");
    state.photo_repo.save(&photo, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "photo save failed");
        // listing-photo 의 RepoError 와 listing 의 RepoError 는 별개 enum.
        // 매핑은 간단히 재현 (NotFound/Conflict/Database).
        match e {
            listing_photo_domain::repository::RepoError::NotFound => problem(
                "not-found",
                "리소스를 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            ),
            listing_photo_domain::repository::RepoError::Conflict => problem(
                "version-conflict",
                "충돌이 발생했어요",
                StatusCode::CONFLICT,
                None,
            ),
            listing_photo_domain::repository::RepoError::Database(_) => problem(
                "internal-error",
                "내부 서버 오류",
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
        }
    })?;

    // SECURITY/UX: SP4-iii-e R2 통합 전. presigned URL 은 mock — frontend e2e 가
    // 실 PUT 시도 안 함. tracing target=`photo.upload.mock` 으로 후속 R2 통합 시
    // 검색 가능하도록 marker 남김.
    tracing::info!(
        target: "photo.upload.mock",
        photo_id = %photo_id,
        r2_key = %r2_key,
        "issued mock presigned URL (SP4-iii-e pending)"
    );

    Ok((
        StatusCode::CREATED,
        Json(RequestPhotoUploadResponse {
            photo_id: photo_id.as_str().to_owned(),
            presigned_put_url: format!("MOCK://r2/{r2_key}"),
            r2_key,
            expires_at: now + chrono::Duration::minutes(15),
        }),
    ))
}

/// `DELETE /listings/:id/photos/:photo_id` — soft-delete.
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %listing_id, photo_id = %photo_id))]
pub async fn delete_photo(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path((listing_id, photo_id)): Path<(String, String)>,
) -> Result<StatusCode, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    // 매물 ownership 검증 (사진 단독 ownership 컬럼은 없음 — listing 으로 추적).
    let _listing = load_listing_for_actor(&state, &auth, &listing_id).await?;

    let pid = Id::<ListingPhotoMarker>::from_str(&photo_id).map_err(|e| {
        problem(
            "validation",
            "photo_id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let ctx = http_user_action(&auth, "delete_photo");
    state.photo_repo.delete(&pid, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "photo delete failed");
        match e {
            listing_photo_domain::repository::RepoError::NotFound => problem(
                "not-found",
                "사진을 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            ),
            listing_photo_domain::repository::RepoError::Conflict
            | listing_photo_domain::repository::RepoError::Database(_) => problem(
                "internal-error",
                "내부 서버 오류",
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
        }
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────
// SP6-iii: GET /listings/:id 매물 상세
// ─────────────────────────────────────────────────────────────────────────
