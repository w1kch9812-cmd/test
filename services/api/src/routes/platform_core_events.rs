//! Internal receiver for Platform Core events forwarded by the Next.js public route.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use db::platform_core_anchor::{self, PlatformCoreEventInboxInsert, PlatformCoreEventInboxStatus};
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use subtle::ConstantTimeEq;

const INTERNAL_AUTH_HEADER: &str = "x-internal-auth";
const GOLD_POINTER_EVENT_TYPE: &str = "catalog.industrial_complex.gold_pointer.published.v1";
const PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE: &str =
    "catalog.parcel_marker_anchor.snapshot.published.v1";

/// Route state.
#[derive(Clone)]
pub struct PlatformCoreEventsState {
    /// Shared Postgres pool.
    pub pool: PgPool,
    /// Shared internal auth secret.
    pub internal_auth_secret: Arc<str>,
}

/// Platform Core event envelope forwarded by the public receiver.
#[derive(Debug, Deserialize)]
pub struct PlatformCoreEventEnvelope {
    event_id: String,
    event_type: String,
    occurred_at: String,
    scope: String,
    payload: Value,
}

/// Persist an inbound Platform Core event.
pub async fn post_platform_core_event(
    State(state): State<PlatformCoreEventsState>,
    headers: HeaderMap,
    Json(event): Json<PlatformCoreEventEnvelope>,
) -> impl IntoResponse {
    if !internal_auth_ok(&headers, &state.internal_auth_secret) {
        return json_response(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"status": "rejected", "reason": "unauthorized"}),
        );
    }

    let Ok(row) = inbox_insert_from_event(&event) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({"status": "rejected", "reason": "invalid_event"}),
        );
    };
    let effect = row.effect.clone();
    match platform_core_anchor::insert_inbox_event(&state.pool, &row).await {
        Ok(_) => json_response(
            StatusCode::ACCEPTED,
            serde_json::json!({
                "event_id": event.event_id,
                "effect": effect,
                "status": "accepted"
            }),
        ),
        Err(error) => {
            tracing::error!(
                event_id = %event.event_id,
                event_type = %event.event_type,
                error = %error,
                "failed to persist platform core event"
            );
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status": "rejected", "reason": "inbox_write_failed"}),
            )
        }
    }
}

const fn json_response(status: StatusCode, body: Value) -> (StatusCode, Json<Value>) {
    (status, Json(body))
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

fn inbox_insert_from_event(
    event: &PlatformCoreEventEnvelope,
) -> Result<PlatformCoreEventInboxInsert, &'static str> {
    if event.scope != "catalog" {
        return Err("scope");
    }
    if event.occurred_at.trim().is_empty() {
        return Err("occurred_at");
    }

    match event.event_type.as_str() {
        GOLD_POINTER_EVENT_TYPE => Ok(PlatformCoreEventInboxInsert {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            scope: event.scope.clone(),
            effect: "invalidate_catalog_cache".to_owned(),
            status: PlatformCoreEventInboxStatus::Accepted,
            payload: event.payload.clone(),
            anchor_snapshot_id: None,
            source_geometry_version: None,
        }),
        PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE => {
            let anchor_snapshot_id = event
                .payload
                .get("anchor_snapshot_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.trim().is_empty())
                .ok_or("anchor_snapshot_id")?;
            let source_geometry_version = event
                .payload
                .get("source_geometry_version")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|value| !value.trim().is_empty())
                .ok_or("source_geometry_version")?;

            Ok(PlatformCoreEventInboxInsert {
                event_id: event.event_id.clone(),
                event_type: event.event_type.clone(),
                scope: event.scope.clone(),
                effect: "enqueue_anchor_projection_import".to_owned(),
                status: PlatformCoreEventInboxStatus::PendingImport,
                payload: event.payload.clone(),
                anchor_snapshot_id: Some(anchor_snapshot_id),
                source_geometry_version: Some(source_geometry_version),
            })
        }
        _ => Err("event_type"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_snapshot_event_maps_to_pending_import_inbox_row() {
        let event = PlatformCoreEventEnvelope {
            event_id: "0196f0b0-3e01-7000-8000-000000000002".to_owned(),
            event_type: PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE.to_owned(),
            occurred_at: "2026-05-28T12:00:00Z".to_owned(),
            scope: "catalog".to_owned(),
            payload: serde_json::json!({
                "type": PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE,
                "schema_version": 1,
                "anchor_snapshot_id": "anchor-snapshot-20260528T120000Z",
                "source_geometry_version": "silver.parcel_boundaries@20260528",
                "artifact_manifest_url": "https://platform-core.example.com/artifacts/anchor-snapshot.json",
                "artifact_checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "row_count": 1,
                "published_at": "2026-05-28T12:00:00Z"
            }),
        };

        let row = inbox_insert_from_event(&event).expect("inbox row");
        assert_eq!(row.event_id, event.event_id);
        assert_eq!(row.effect, "enqueue_anchor_projection_import");
        assert_eq!(
            row.anchor_snapshot_id.as_deref(),
            Some("anchor-snapshot-20260528T120000Z")
        );
        assert_eq!(
            row.source_geometry_version.as_deref(),
            Some("silver.parcel_boundaries@20260528")
        );
    }
}
