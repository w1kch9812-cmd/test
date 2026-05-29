//! `/me/notifications` 핸들러 (SP6-v).
//!
//! 4 endpoints:
//! - `GET /me/notifications?unread_only=&limit=` — 최근 365일 (unread filter)
//! - `GET /me/notifications/unread-count` — badge 폴링
//! - `PATCH /me/notifications/:id/read` — 단건 (멱등)
//! - `POST /me/notifications/mark-all-read?kind=` — bulk

use std::str::FromStr;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use notification_domain::kind::NotificationKind;
use notification_domain::repository::{NotificationRepository, RepoError as NotifRepoError};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, NotificationMarker};

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{problem, ProblemResponse};

/// 핸들러 공유 상태.
#[derive(Clone)]
pub struct NotificationsState {
    /// `NotificationRepository` 구현체.
    pub notification_repo: Arc<dyn NotificationRepository>,
}

/// `GET /me/notifications` 쿼리 파라미터.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// `true` 면 미읽음만, `false` 또는 미설정이면 365일 최근.
    #[serde(default)]
    pub unread_only: bool,
    /// 최대 행 수 (default 50, max 200).
    pub limit: Option<u32>,
}

/// 알림 단건 응답.
#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    /// 알림 ID.
    pub id: String,
    /// 종류 (`snake_case`).
    pub kind: String,
    /// payload JSON.
    pub payload: serde_json::Value,
    /// 읽음 시각 (`null` = 미읽음).
    pub read_at: Option<DateTime<Utc>>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

/// `GET /me/notifications` 응답.
#[derive(Debug, Serialize)]
pub struct ListResponse {
    /// 알림 목록.
    pub notifications: Vec<NotificationResponse>,
}

/// `GET /me/notifications` — 인증 사용자 알림 목록.
#[tracing::instrument(skip(state, auth, q), fields(actor = %auth.user.id))]
pub async fn list_notifications(
    State(state): State<NotificationsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ProblemResponse> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let notifications = if q.unread_only {
        state
            .notification_repo
            .find_unread_by_user(&auth.user.id)
            .await
    } else {
        state
            .notification_repo
            .find_recent_by_user(&auth.user.id, limit)
            .await
    }
    .map_err(|e| {
        tracing::error!(error = %e, "notifications find failed");
        from_notification_repo_error(&e)
    })?;

    let mapped = notifications
        .into_iter()
        .map(|n| NotificationResponse {
            id: n.id.as_str().to_owned(),
            kind: n.kind.as_str().to_owned(),
            payload: n.payload,
            read_at: n.read_at,
            created_at: n.created_at,
        })
        .collect();

    Ok(Json(ListResponse {
        notifications: mapped,
    }))
}

/// `GET /me/notifications/unread-count` 응답.
#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    /// 미읽음 알림 수.
    pub count: i64,
}

/// `GET /me/notifications/unread-count` — badge 용 작은 응답.
///
/// 1차 = `find_unread_by_user(...).len()` 으로 충분 (수십 건 수준). 백만 단위
/// 도달 시 별도 SELECT count(*) endpoint 추가 (FU).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id))]
pub async fn unread_count(
    State(state): State<NotificationsState>,
    Extension(auth): Extension<AuthenticatedUser>,
) -> Result<Json<UnreadCountResponse>, ProblemResponse> {
    let unread = state
        .notification_repo
        .find_unread_by_user(&auth.user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "unread count failed");
            from_notification_repo_error(&e)
        })?;

    Ok(Json(UnreadCountResponse {
        count: i64::try_from(unread.len()).unwrap_or(i64::MAX),
    }))
}

/// `PATCH /me/notifications/:id/read` — 단건 멱등 읽음 처리.
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, notification_id = %id))]
pub async fn mark_read(
    State(state): State<NotificationsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, ProblemResponse> {
    let notif_id = Id::<NotificationMarker>::from_str(&id).map_err(|e| {
        problem(
            "validation",
            "notification id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    // ownership check 는 trait 가 직접 강제 안 함 — 본인 ID 만 mark_read 호출하는
    // 패턴 (SP6-v 1차). 다른 사용자 알림 mark_read 시도해도 UPDATE 가 row 0
    // (user_id mismatch + read_at IS NULL 조건 X). 멱등이 안전망.
    let ctx = http_user_action(&auth, "mark_notification_read");
    state
        .notification_repo
        .mark_read(&notif_id, Utc::now(), ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mark_read failed");
            from_notification_repo_error(&e)
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /me/notifications/mark-all-read` 쿼리 파라미터.
#[derive(Debug, Deserialize)]
pub struct MarkAllQuery {
    /// 종류 (예: `listing_approved`). 미설정 시 전체 unread 대상 — but trait
    /// 시그니처가 kind 강제 → 1차 = kind 필수 (전체 = `all` keyword X, 호출 측
    /// 명시).
    pub kind: String,
}

/// `POST /me/notifications/mark-all-read` 응답.
#[derive(Debug, Serialize)]
pub struct MarkAllResponse {
    /// 갱신된 row 수.
    pub marked_count: u64,
}

/// `POST /me/notifications/mark-all-read?kind=...` — bulk 읽음.
///
/// `kind` 미지원 코드 = `Other` fallback (forward-compat — 알 수 없는 kind 도
/// 안전하게 `NotificationKind::Other` 매칭. SP6-v 1차 = 알려진 3 kind 만 의미
/// 있음).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, kind = %q.kind))]
pub async fn mark_all_read(
    State(state): State<NotificationsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Query(q): Query<MarkAllQuery>,
) -> Result<Json<MarkAllResponse>, ProblemResponse> {
    // FromStr is Infallible — Other fallback.
    let kind = NotificationKind::from_str(&q.kind).unwrap_or(NotificationKind::Other);

    let ctx = http_user_action(&auth, "mark_all_notifications_read");
    let marked_count = state
        .notification_repo
        .mark_all_read_by_kind(&auth.user.id, kind, Utc::now(), ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mark_all_read_by_kind failed");
            from_notification_repo_error(&e)
        })?;

    Ok(Json(MarkAllResponse { marked_count }))
}

fn from_notification_repo_error(e: &NotifRepoError) -> ProblemResponse {
    match e {
        NotifRepoError::NotFound => problem(
            "not-found",
            "알림을 찾을 수 없어요",
            StatusCode::NOT_FOUND,
            None,
        ),
        NotifRepoError::Database(_) => problem(
            "internal-error",
            "내부 서버 오류",
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
        ),
    }
}
