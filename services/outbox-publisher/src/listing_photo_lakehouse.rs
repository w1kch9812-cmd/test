//! Listing photo media lakehouse registration sink.

#![allow(clippy::module_name_repetitions)]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, Region, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::Client as S3Client;
use outbox_event_domain::entity::OutboxEvent;
use outbox_publisher::{Sink, SinkError};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) use crate::platform_core_lakehouse_registry::LakehouseObjectArtifactRegistration;
use crate::platform_core_lakehouse_registry::PlatformCoreLakehouseRegistryClient;

const LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE: &str = "listing_photo.upload_confirmed";
const LISTING_PHOTO_AGGREGATE_KIND: &str = "listing_photo";
const LISTING_PHOTO_OBJECT_KEY_PREFIX: &str = "media/listing-photo/";
const LISTING_PHOTO_MEDIA_QUALIFIED_NAME: &str = "gongzzang.gold.listing_photo_media";
const LISTING_PHOTO_MEDIA_NAMESPACE_ID: &str = "gongzzang_r2_production";

#[derive(Debug, Clone)]
pub(crate) struct ListingPhotoLakehouseSink<R, G> {
    reader: Arc<R>,
    registrar: Arc<G>,
}

impl<R, G> ListingPhotoLakehouseSink<R, G> {
    pub(crate) const fn new(reader: Arc<R>, registrar: Arc<G>) -> Self {
        Self { reader, registrar }
    }
}

#[async_trait]
impl<R, G> Sink for ListingPhotoLakehouseSink<R, G>
where
    R: ListingPhotoObjectReader + Send + Sync + 'static,
    G: LakehouseArtifactRegistrar + Send + Sync + 'static,
{
    async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError> {
        if event.event_type != LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE {
            return Ok(());
        }
        let registration = build_registration(event, self.reader.as_ref()).await?;
        self.registrar
            .register_object_artifact(registration)
            .await
            .map_err(|error| SinkError::Publish(error.to_string()))
    }
}

#[async_trait]
pub(crate) trait ListingPhotoObjectReader: Send + Sync {
    async fn read_verified_object(
        &self,
        object_key: &str,
        expected_content_type: &str,
    ) -> Result<ObjectDigest, ObjectReadError>;
}

#[async_trait]
pub(crate) trait LakehouseArtifactRegistrar: Send + Sync {
    async fn register_object_artifact(
        &self,
        registration: LakehouseObjectArtifactRegistration,
    ) -> Result<(), RegistryError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ObjectDigest {
    pub(crate) content_type: String,
    pub(crate) checksum_sha256: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum ObjectReadError {
    #[error("listing photo object read failed: {0}")]
    Read(String),
    #[error("listing photo object not found")]
    NotFound,
    #[error("listing photo object content-length missing")]
    MissingContentLength,
    #[error("listing photo object content-length must be positive: {actual}")]
    InvalidContentLength { actual: i64 },
    #[error("listing photo object content-type missing")]
    MissingContentType,
    #[error("listing photo object content-type mismatch: expected {expected}, got {actual}")]
    ContentTypeMismatch { expected: String, actual: String },
    #[error("listing photo object size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum RegistryError {
    #[error("lakehouse artifact registry failed: {0}")]
    Register(String),
}

#[derive(Debug, Clone)]
pub(crate) struct R2ListingPhotoObjectReader {
    client: S3Client,
    bucket: String,
}

impl R2ListingPhotoObjectReader {
    #[must_use]
    pub(crate) fn new(config: ListingPhotoR2ReadConfig) -> Self {
        Self {
            client: config.s3_client(),
            bucket: config.bucket,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ListingPhotoR2ReadConfig {
    account_id: String,
    access_key: String,
    secret_key: String,
    bucket: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum ListingPhotoR2ReadConfigError {
    #[error("env {0} not set")]
    MissingEnv(&'static str),
    #[error("env {0} empty")]
    EmptyEnv(&'static str),
}

impl ListingPhotoR2ReadConfig {
    pub(crate) fn from_env() -> Result<Self, ListingPhotoR2ReadConfigError> {
        Ok(Self {
            account_id: require_env("LISTING_PHOTO_R2_ACCOUNT_ID")?,
            access_key: require_env("LISTING_PHOTO_R2_ACCESS_KEY")?,
            secret_key: require_env("LISTING_PHOTO_R2_SECRET_KEY")?,
            bucket: require_env("LISTING_PHOTO_R2_BUCKET")?,
        })
    }

    #[must_use]
    fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }

    #[must_use]
    fn s3_client(&self) -> S3Client {
        let creds = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "outbox-listing-photo-lakehouse",
        );
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(self.endpoint_url())
            .credentials_provider(creds)
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(2))
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_attempt_timeout(Duration::from_secs(30))
                    .build(),
            )
            .build();
        S3Client::from_conf(s3_config)
    }
}

#[async_trait]
impl ListingPhotoObjectReader for R2ListingPhotoObjectReader {
    async fn read_verified_object(
        &self,
        object_key: &str,
        expected_content_type: &str,
    ) -> Result<ObjectDigest, ObjectReadError> {
        validate_listing_photo_object_key(object_key)?;
        let output =
            self.client
                .get_object()
                .bucket(&self.bucket)
                .key(object_key)
                .send()
                .await
                .map_err(|error| {
                    if error.as_service_error().is_some_and(
                        aws_sdk_s3::operation::get_object::GetObjectError::is_no_such_key,
                    ) {
                        return ObjectReadError::NotFound;
                    }
                    ObjectReadError::Read(format!(
                        "{}",
                        aws_sdk_s3::error::DisplayErrorContext(&error)
                    ))
                })?;

        let expected_size = positive_content_length(output.content_length())?;
        let content_type = output
            .content_type()
            .ok_or(ObjectReadError::MissingContentType)?
            .to_owned();
        if content_type != expected_content_type {
            return Err(ObjectReadError::ContentTypeMismatch {
                expected: expected_content_type.to_owned(),
                actual: content_type,
            });
        }

        let mut stream = output.body;
        let mut hasher = Sha256::new();
        let mut actual_size = 0_u64;
        while let Some(chunk) = stream
            .try_next()
            .await
            .map_err(|error| ObjectReadError::Read(error.to_string()))?
        {
            actual_size = actual_size
                .checked_add(u64::try_from(chunk.len()).map_err(|error| {
                    ObjectReadError::Read(format!("chunk length conversion failed: {error}"))
                })?)
                .ok_or_else(|| ObjectReadError::Read("object size overflow".to_owned()))?;
            hasher.update(&chunk);
        }

        if actual_size != expected_size {
            return Err(ObjectReadError::SizeMismatch {
                expected: expected_size,
                actual: actual_size,
            });
        }

        Ok(ObjectDigest {
            content_type,
            checksum_sha256: format!("{:x}", hasher.finalize()),
            size_bytes: actual_size,
        })
    }
}

#[async_trait]
impl LakehouseArtifactRegistrar for PlatformCoreLakehouseRegistryClient {
    async fn register_object_artifact(
        &self,
        registration: LakehouseObjectArtifactRegistration,
    ) -> Result<(), RegistryError> {
        self.register_object_artifact_http(registration)
            .await
            .map(|_| ())
            .map_err(|error| RegistryError::Register(error.to_string()))
    }
}

async fn build_registration(
    event: &OutboxEvent,
    reader: &dyn ListingPhotoObjectReader,
) -> Result<LakehouseObjectArtifactRegistration, SinkError> {
    if event.aggregate_kind != LISTING_PHOTO_AGGREGATE_KIND {
        return Err(SinkError::Publish(format!(
            "{LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE} aggregate_kind must be {LISTING_PHOTO_AGGREGATE_KIND}"
        )));
    }

    let object_key = required_payload_string(event, "r2_key")?;
    validate_listing_photo_object_key(&object_key)
        .map_err(|error| SinkError::Publish(error.to_string()))?;
    let content_type = required_payload_string(event, "content_type")?;
    let file_size_bytes = required_payload_positive_u64(event, "file_size_bytes")?;
    let digest = reader
        .read_verified_object(&object_key, &content_type)
        .await
        .map_err(|error| SinkError::Publish(error.to_string()))?;

    if digest.content_type != content_type {
        return Err(SinkError::Publish(
            ObjectReadError::ContentTypeMismatch {
                expected: content_type,
                actual: digest.content_type,
            }
            .to_string(),
        ));
    }
    if digest.size_bytes != file_size_bytes {
        return Err(SinkError::Publish(
            ObjectReadError::SizeMismatch {
                expected: file_size_bytes,
                actual: digest.size_bytes,
            }
            .to_string(),
        ));
    }

    Ok(LakehouseObjectArtifactRegistration {
        qualified_name: LISTING_PHOTO_MEDIA_QUALIFIED_NAME.to_owned(),
        namespace_id: LISTING_PHOTO_MEDIA_NAMESPACE_ID.to_owned(),
        object_key,
        content_type: digest.content_type,
        checksum_sha256: digest.checksum_sha256,
        size_bytes: digest.size_bytes,
        logical_record_count: None,
    })
}

fn require_env(name: &'static str) -> Result<String, ListingPhotoR2ReadConfigError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(ListingPhotoR2ReadConfigError::EmptyEnv(name)),
        Ok(value) => Ok(value),
        Err(_) => Err(ListingPhotoR2ReadConfigError::MissingEnv(name)),
    }
}

fn positive_content_length(value: Option<i64>) -> Result<u64, ObjectReadError> {
    let value = value.ok_or(ObjectReadError::MissingContentLength)?;
    if value <= 0 {
        return Err(ObjectReadError::InvalidContentLength { actual: value });
    }
    u64::try_from(value).map_err(|error| ObjectReadError::Read(error.to_string()))
}

fn required_payload_string(event: &OutboxEvent, field: &'static str) -> Result<String, SinkError> {
    event
        .payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| SinkError::Publish(format!("missing listing photo payload field {field}")))
}

fn required_payload_positive_u64(
    event: &OutboxEvent,
    field: &'static str,
) -> Result<u64, SinkError> {
    let value = event
        .payload
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            SinkError::Publish(format!("missing listing photo payload field {field}"))
        })?;
    if value <= 0 {
        return Err(SinkError::Publish(format!(
            "listing photo payload field {field} must be positive"
        )));
    }
    u64::try_from(value).map_err(|error| SinkError::Publish(error.to_string()))
}

fn validate_listing_photo_object_key(object_key: &str) -> Result<(), ObjectReadError> {
    if object_key.trim() != object_key
        || !object_key.starts_with(LISTING_PHOTO_OBJECT_KEY_PREFIX)
        || object_key.contains('\\')
        || object_key.contains("//")
        || object_key.contains("/../")
        || object_key.contains("/./")
        || object_key.starts_with('/')
        || object_key.contains('?')
        || object_key.contains('#')
    {
        return Err(ObjectReadError::Read(
            "listing photo object key must be a normalized media/listing-photo/ key".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "listing_photo_lakehouse/tests.rs"]
mod tests;
