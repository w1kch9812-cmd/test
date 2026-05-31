# Platform Core Anchor Projection Import Plan - Part 04: Event Route And Importer

Parent index: [Platform Core Anchor Projection Import Implementation Plan](./2026-05-28-platform-core-anchor-projection-import.md).


## Task 3: Rust Internal Event Route

**Files:**
- Create: `services/api/src/routes/platform_core_events.rs`
- Modify: `services/api/src/main.rs`

- [ ] **Step 1: Write the route unit test**

Create route tests in `services/api/src/routes/platform_core_events.rs` under `#[cfg(test)]` that call a pure function:

```rust
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
    assert_eq!(row.anchor_snapshot_id.as_deref(), Some("anchor-snapshot-20260528T120000Z"));
}
```

- [ ] **Step 2: Run the route test and verify RED**

Run:

```powershell
cargo test -p api platform_core_events
```

Expected: compile failure because `platform_core_events` route does not exist.

- [ ] **Step 3: Implement the route module**

Create `services/api/src/routes/platform_core_events.rs` with:

```rust
//! Internal receiver for Platform Core events forwarded by the Next.js public route.

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use db::platform_core_anchor::{
    self, PlatformCoreEventInboxInsert, PlatformCoreEventInboxStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

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
    pub internal_auth_secret: String,
}

#[derive(Debug, Deserialize)]
struct PlatformCoreEventEnvelope {
    event_id: String,
    event_type: String,
    occurred_at: String,
    scope: String,
    payload: Value,
}

#[derive(Debug, Serialize)]
struct PlatformCoreEventAck {
    event_id: String,
    effect: &'static str,
    status: &'static str,
}

/// Persist an inbound Platform Core event.
pub async fn post_platform_core_event(
    State(state): State<PlatformCoreEventsState>,
    headers: HeaderMap,
    Json(event): Json<PlatformCoreEventEnvelope>,
) -> impl IntoResponse {
    if !internal_auth_ok(&headers, &state.internal_auth_secret) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"status": "rejected", "reason": "unauthorized"})),
        );
    }

    let Ok(row) = inbox_insert_from_event(&event) else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "rejected", "reason": "invalid_event"})),
        );
    };
    let effect = row.effect.clone();
    match platform_core_anchor::insert_inbox_event(&state.pool, &row).await {
        Ok(_) => (
            axum::http::StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "event_id": event.event_id,
                "effect": effect,
                "status": "accepted"
            })),
        ),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "rejected",
                "reason": "inbox_write_failed",
                "detail": error.to_string()
            })),
        ),
    }
}
```

Add helper functions in the same file:

```rust
fn internal_auth_ok(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(INTERNAL_AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| actual == expected)
}

fn inbox_insert_from_event(
    event: &PlatformCoreEventEnvelope,
) -> Result<PlatformCoreEventInboxInsert, &'static str> {
    if event.scope != "catalog" {
        return Err("scope");
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
        PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE => Ok(PlatformCoreEventInboxInsert {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            scope: event.scope.clone(),
            effect: "enqueue_anchor_projection_import".to_owned(),
            status: PlatformCoreEventInboxStatus::PendingImport,
            payload: event.payload.clone(),
            anchor_snapshot_id: event
                .payload
                .get("anchor_snapshot_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            source_geometry_version: event
                .payload
                .get("source_geometry_version")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }),
        _ => Err("event_type"),
    }
}
```

- [ ] **Step 4: Wire the route in `main.rs`**

Add module export:

```rust
pub mod platform_core_events;
```

Add router:

```rust
let platform_core_events_router: Router<()> = Router::new()
    .route(
        "/internal/platform-core/events",
        axum::routing::post(routes::platform_core_events::post_platform_core_event),
    )
    .with_state(routes::platform_core_events::PlatformCoreEventsState {
        pool: auth_event_state.pool.clone(),
        internal_auth_secret: auth_event_state.internal_auth_secret.clone(),
    });
```

Merge it before `internal`:

```rust
.merge(platform_core_events_router)
```

- [ ] **Step 5: Run route tests and verify GREEN**

Run:

```powershell
cargo test -p api platform_core_events
```

Expected: route tests pass.

## Task 4: Anchor Artifact Importer

**Files:**
- Create: `services/api/src/platform_core_anchor_import.rs`
- Create: `services/api/src/bin/platform_core_anchor_import.rs`
- Modify: `services/api/Cargo.toml`

- [ ] **Step 1: Write parser/checksum unit tests**

Create tests in `services/api/src/platform_core_anchor_import.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anchor_entry_into_db_row() {
        let entry = parse_anchor_entry(
            r#"{"schema_version":"platform-core.parcel_marker_anchor_artifact_entry.v1","pnu":"1111010100100090000","anchor_lng":126.978,"anchor_lat":37.5665,"anchor_srid":"EPSG:4326","algorithm":"polylabel","algorithm_version":"postgis-st_maximuminscribedcircle-v1","source_geometry_checksum_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
        )
        .expect("entry");

        assert_eq!(entry.pnu, "1111010100100090000");
        assert_eq!(entry.algorithm_version, "postgis-st_maximuminscribedcircle-v1");
    }

    #[test]
    fn rejects_wrong_entry_srid() {
        let err = parse_anchor_entry(
            r#"{"schema_version":"platform-core.parcel_marker_anchor_artifact_entry.v1","pnu":"1111010100100090000","anchor_lng":126.978,"anchor_lat":37.5665,"anchor_srid":"EPSG:3857","algorithm":"polylabel","algorithm_version":"postgis-st_maximuminscribedcircle-v1","source_geometry_checksum_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("EPSG:4326"));
    }
}
```

- [ ] **Step 2: Run parser tests and verify RED**

Run:

```powershell
cargo test -p api platform_core_anchor_import
```

Expected: compile failure because the importer module does not exist.

- [ ] **Step 3: Add existing workspace checksum dependency if needed**

In `services/api/Cargo.toml`, add:

```toml
sha2 = { workspace = true }
```

- [ ] **Step 4: Implement manifest and entry parsing**

Create `services/api/src/platform_core_anchor_import.rs` with:

```rust
//! Platform Core anchor artifact importer.

use db::platform_core_anchor::{AnchorArtifactRow, PlatformCoreAnchorImport};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

const MANIFEST_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_manifest.v1";
const ENTRY_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_entry.v1";

#[derive(Debug, Error)]
pub enum AnchorImportError {
    #[error("invalid anchor artifact json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("anchor artifact contract mismatch: {0}")]
    Contract(&'static str),
    #[error("anchor artifact checksum mismatch")]
    Checksum,
}

#[derive(Debug, Deserialize)]
struct AnchorArtifactEntry {
    schema_version: String,
    pnu: String,
    anchor_lng: f64,
    anchor_lat: f64,
    anchor_srid: String,
    algorithm: String,
    algorithm_version: String,
    source_geometry_checksum_sha256: String,
}

pub fn parse_anchor_entry(line: &str) -> Result<AnchorArtifactRow, AnchorImportError> {
    let entry: AnchorArtifactEntry = serde_json::from_str(line)?;
    if entry.schema_version != ENTRY_SCHEMA_VERSION {
        return Err(AnchorImportError::Contract("entry schema_version"));
    }
    if entry.anchor_srid != "EPSG:4326" {
        return Err(AnchorImportError::Contract("entry anchor_srid must be EPSG:4326"));
    }
    if entry.pnu.len() != 19 || !entry.pnu.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AnchorImportError::Contract("entry pnu"));
    }
    if !entry.anchor_lng.is_finite() || !(-180.0..=180.0).contains(&entry.anchor_lng) {
        return Err(AnchorImportError::Contract("entry anchor_lng"));
    }
    if !entry.anchor_lat.is_finite() || !(-90.0..=90.0).contains(&entry.anchor_lat) {
        return Err(AnchorImportError::Contract("entry anchor_lat"));
    }

    Ok(AnchorArtifactRow {
        pnu: entry.pnu,
        anchor_lng: entry.anchor_lng,
        anchor_lat: entry.anchor_lat,
        algorithm: entry.algorithm,
        algorithm_version: entry.algorithm_version,
        source_geometry_checksum_sha256: entry.source_geometry_checksum_sha256,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::with_capacity(64), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
        out
    })
}
```

- [ ] **Step 5: Run importer tests and verify GREEN**

Run:

```powershell
cargo test -p api platform_core_anchor_import
```

Expected: parser tests pass.
