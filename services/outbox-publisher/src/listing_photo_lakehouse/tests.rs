#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use outbox_event_domain::entity::OutboxEvent;
use outbox_publisher::Sink;
use serde_json::json;
use shared_kernel::id::{Id, OutboxEventMarker};

use super::{
    LakehouseArtifactRegistrar, LakehouseObjectArtifactRegistration, ListingPhotoLakehouseSink,
    ListingPhotoObjectReader, ObjectDigest, ObjectReadError, RegistryError,
};

#[derive(Debug)]
struct FakeObjectReader {
    calls: Mutex<Vec<(String, String)>>,
    result: Mutex<Result<ObjectDigest, ObjectReadError>>,
}

impl Default for FakeObjectReader {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            result: Mutex::new(Ok(ObjectDigest::default())),
        }
    }
}

#[async_trait]
impl ListingPhotoObjectReader for FakeObjectReader {
    async fn read_verified_object(
        &self,
        object_key: &str,
        expected_content_type: &str,
    ) -> Result<ObjectDigest, ObjectReadError> {
        self.calls
            .lock()
            .unwrap()
            .push((object_key.to_owned(), expected_content_type.to_owned()));
        self.result.lock().unwrap().clone()
    }
}

#[derive(Debug, Default)]
struct FakeRegistry {
    registrations: Mutex<Vec<LakehouseObjectArtifactRegistration>>,
}

#[async_trait]
impl LakehouseArtifactRegistrar for FakeRegistry {
    async fn register_object_artifact(
        &self,
        registration: LakehouseObjectArtifactRegistration,
    ) -> Result<(), RegistryError> {
        self.registrations.lock().unwrap().push(registration);
        Ok(())
    }
}

#[tokio::test]
async fn listing_photo_upload_event_registers_digest_verified_media_artifact() {
    let reader = Arc::new(FakeObjectReader {
        calls: Mutex::new(Vec::new()),
        result: Mutex::new(Ok(ObjectDigest {
            content_type: "image/webp".to_owned(),
            checksum_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_owned(),
            size_bytes: 2048,
        })),
    });
    let registry = Arc::new(FakeRegistry::default());
    let sink = ListingPhotoLakehouseSink::new(reader.clone(), registry.clone());

    sink.publish(&listing_photo_event(2048))
        .await
        .expect("publish");

    assert_eq!(
        reader.calls.lock().unwrap().as_slice(),
        &[(
            "media/listing-photo/listings/lst_1/photos/lph_1.webp".to_owned(),
            "image/webp".to_owned()
        )]
    );
    let registration = {
        let registrations = registry.registrations.lock().unwrap();
        assert_eq!(registrations.len(), 1);
        registrations[0].clone()
    };
    assert_eq!(
        registration.qualified_name,
        "gongzzang.gold.listing_photo_media"
    );
    assert_eq!(registration.namespace_id, "gongzzang_r2_production");
    assert_eq!(
        registration.object_key,
        "media/listing-photo/listings/lst_1/photos/lph_1.webp"
    );
    assert_eq!(registration.content_type, "image/webp");
    assert_eq!(
        registration.checksum_sha256,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(registration.size_bytes, 2048);
    assert_eq!(registration.logical_record_count, None);
}

#[tokio::test]
async fn unrelated_events_are_ignored_without_storage_or_registry_calls() {
    let reader = Arc::new(FakeObjectReader::default());
    let registry = Arc::new(FakeRegistry::default());
    let sink = ListingPhotoLakehouseSink::new(reader.clone(), registry.clone());

    sink.publish(&OutboxEvent {
        id: Id::<OutboxEventMarker>::new(),
        event_type: "listing.approved".to_owned(),
        aggregate_kind: "listing".to_owned(),
        aggregate_id: "lst_1".to_owned(),
        payload: json!({}),
        occurred_at: Utc::now(),
        published_at: None,
        correlation_id: "cor_1".to_owned(),
    })
    .await
    .expect("ignored");

    assert!(reader.calls.lock().unwrap().is_empty());
    assert!(registry.registrations.lock().unwrap().is_empty());
}

#[tokio::test]
async fn file_size_mismatch_is_not_registered() {
    let reader = Arc::new(FakeObjectReader {
        calls: Mutex::new(Vec::new()),
        result: Mutex::new(Ok(ObjectDigest {
            content_type: "image/webp".to_owned(),
            checksum_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_owned(),
            size_bytes: 4096,
        })),
    });
    let registry = Arc::new(FakeRegistry::default());
    let sink = ListingPhotoLakehouseSink::new(reader, registry.clone());

    let error = sink
        .publish(&listing_photo_event(2048))
        .await
        .expect_err("size mismatch");

    assert!(error.to_string().contains("size mismatch"));
    assert!(registry.registrations.lock().unwrap().is_empty());
}

fn listing_photo_event(file_size_bytes: i64) -> OutboxEvent {
    OutboxEvent {
        id: Id::<OutboxEventMarker>::new(),
        event_type: "listing_photo.upload_confirmed".to_owned(),
        aggregate_kind: "listing_photo".to_owned(),
        aggregate_id: "lph_1".to_owned(),
        payload: json!({
            "photo_id": "lph_1",
            "listing_id": "lst_1",
            "r2_key": "media/listing-photo/listings/lst_1/photos/lph_1.webp",
            "content_type": "image/webp",
            "file_size_bytes": file_size_bytes,
        }),
        occurred_at: Utc::now(),
        published_at: None,
        correlation_id: "cor_1".to_owned(),
    }
}
