//! Internal Prometheus metrics route.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use sqlx::{PgPool, Row};
use subtle::ConstantTimeEq;

const INTERNAL_AUTH_HEADER: &str = "x-internal-auth";
const PROMETHEUS_TEXT_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Shared state for internal metrics.
#[derive(Clone)]
pub struct MetricsState {
    /// Shared Postgres pool.
    pub pool: PgPool,
    /// `X-Internal-Auth` shared secret.
    pub internal_auth_secret: Arc<str>,
}

#[derive(Debug, Clone, PartialEq)]
struct ListingMarkerMetrics {
    dirty_tiles_pending: i64,
    dirty_tile_oldest_age_seconds: f64,
    tombstones_active: i64,
    deltas_active: i64,
}

/// `GET /internal/metrics`.
#[tracing::instrument(skip(state, headers))]
pub async fn get_metrics(State(state): State<MetricsState>, headers: HeaderMap) -> Response {
    if !internal_auth_ok(&headers, &state.internal_auth_secret) {
        return text_response(
            StatusCode::UNAUTHORIZED,
            "invalid internal auth\n".to_owned(),
        );
    }

    match load_listing_marker_metrics(&state.pool).await {
        Ok(metrics) => text_response(StatusCode::OK, render_listing_marker_metrics(metrics)),
        Err(error) => {
            tracing::warn!(error = %error, "metrics query failed");
            text_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "metrics unavailable\n".to_owned(),
            )
        }
    }
}

async fn load_listing_marker_metrics(pool: &PgPool) -> Result<ListingMarkerMetrics, sqlx::Error> {
    let row = sqlx::query(
        r"
        select
            (
                select count(*)::int8
                from listing_marker_dirty_tile_queue
                where status = 'pending'
            ) as dirty_tiles_pending,
            (
                select coalesce(
                    extract(epoch from now() - min(first_seen_at))::float8,
                    0.0
                )
                from listing_marker_dirty_tile_queue
                where status = 'pending'
            ) as dirty_tile_oldest_age_seconds,
            (
                select count(*)::int8
                from listing_marker_tombstone_log
                where expires_at > now()
            ) as tombstones_active,
            (
                select count(*)::int8
                from listing_marker_delta_log
                where expires_at > now()
            ) as deltas_active
        ",
    )
    .fetch_one(pool)
    .await?;

    Ok(ListingMarkerMetrics {
        dirty_tiles_pending: row.try_get("dirty_tiles_pending")?,
        dirty_tile_oldest_age_seconds: row.try_get("dirty_tile_oldest_age_seconds")?,
        tombstones_active: row.try_get("tombstones_active")?,
        deltas_active: row.try_get("deltas_active")?,
    })
}

fn text_response(status: StatusCode, body: String) -> Response {
    let mut response = (status, body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(PROMETHEUS_TEXT_CONTENT_TYPE),
    );
    response
}

fn render_listing_marker_metrics(metrics: ListingMarkerMetrics) -> String {
    format!(
        "# HELP gongzzang_listing_marker_dirty_tiles_pending Pending listing marker dirty tile rebuilds.\n\
         # TYPE gongzzang_listing_marker_dirty_tiles_pending gauge\n\
         gongzzang_listing_marker_dirty_tiles_pending {}\n\
         # HELP gongzzang_listing_marker_dirty_tile_oldest_age_seconds Oldest pending listing marker dirty tile age in seconds.\n\
         # TYPE gongzzang_listing_marker_dirty_tile_oldest_age_seconds gauge\n\
         gongzzang_listing_marker_dirty_tile_oldest_age_seconds {}\n\
         # HELP gongzzang_listing_marker_tombstones_active Active listing marker tombstones.\n\
         # TYPE gongzzang_listing_marker_tombstones_active gauge\n\
         gongzzang_listing_marker_tombstones_active {}\n\
         # HELP gongzzang_listing_marker_deltas_active Active listing marker deltas.\n\
         # TYPE gongzzang_listing_marker_deltas_active gauge\n\
         gongzzang_listing_marker_deltas_active {}\n",
        metrics.dirty_tiles_pending,
        metrics.dirty_tile_oldest_age_seconds,
        metrics.tombstones_active,
        metrics.deltas_active
    )
}

fn internal_auth_ok(headers: &HeaderMap, expected: &str) -> bool {
    let provided = headers
        .get(INTERNAL_AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let actual = provided.as_bytes();
    let expected = expected.as_bytes();
    actual.len() == expected.len() && actual.ct_eq(expected).unwrap_u8() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_listing_marker_metrics() {
        let body = render_listing_marker_metrics(ListingMarkerMetrics {
            dirty_tiles_pending: 3,
            dirty_tile_oldest_age_seconds: 12.5,
            tombstones_active: 2,
            deltas_active: 7,
        });

        assert!(body.contains("gongzzang_listing_marker_dirty_tiles_pending 3"));
        assert!(body.contains("gongzzang_listing_marker_dirty_tile_oldest_age_seconds 12.5"));
        assert!(body.contains("gongzzang_listing_marker_tombstones_active 2"));
        assert!(body.contains("gongzzang_listing_marker_deltas_active 7"));
    }
}
