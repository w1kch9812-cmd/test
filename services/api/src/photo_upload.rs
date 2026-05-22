use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use listing_photo_domain::entity::PhotoContentType;
use serde::Serialize;
use thiserror::Error;

use crate::r2_raw_capture::R2RawCaptureConfig;

const LISTING_PHOTO_UPLOAD_EXPIRES: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct PhotoUploadUrlRequest {
    pub r2_key: String,
    pub content_type: PhotoContentType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UploadHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoUploadUrl {
    pub url: String,
    pub required_headers: Vec<UploadHeader>,
}

#[derive(Debug, Error)]
pub enum PhotoUploadUrlError {
    #[error("listing photo upload storage is not configured")]
    Disabled,
    #[error("listing photo upload presigning config: {0}")]
    PresigningConfig(String),
    #[error("listing photo upload presign failed: {0}")]
    Presign(String),
}

#[async_trait]
pub trait ListingPhotoUploadUrlIssuer: Send + Sync {
    async fn issue_upload_url(
        &self,
        request: PhotoUploadUrlRequest,
    ) -> Result<PhotoUploadUrl, PhotoUploadUrlError>;
}

#[derive(Debug, Default)]
pub struct DisabledListingPhotoUploadUrlIssuer;

#[async_trait]
impl ListingPhotoUploadUrlIssuer for DisabledListingPhotoUploadUrlIssuer {
    async fn issue_upload_url(
        &self,
        _request: PhotoUploadUrlRequest,
    ) -> Result<PhotoUploadUrl, PhotoUploadUrlError> {
        Err(PhotoUploadUrlError::Disabled)
    }
}

#[derive(Debug, Clone)]
pub struct R2ListingPhotoUploadUrlIssuer {
    client: S3Client,
    bucket: String,
    expires_in: Duration,
}

impl R2ListingPhotoUploadUrlIssuer {
    #[must_use]
    pub fn new(config: R2RawCaptureConfig) -> Self {
        Self {
            client: config.s3_client("api-listing-photo-upload"),
            bucket: config.bucket,
            expires_in: LISTING_PHOTO_UPLOAD_EXPIRES,
        }
    }
}

#[async_trait]
impl ListingPhotoUploadUrlIssuer for R2ListingPhotoUploadUrlIssuer {
    async fn issue_upload_url(
        &self,
        request: PhotoUploadUrlRequest,
    ) -> Result<PhotoUploadUrl, PhotoUploadUrlError> {
        let presigning_config = PresigningConfig::expires_in(self.expires_in)
            .map_err(|error| PhotoUploadUrlError::PresigningConfig(error.to_string()))?;
        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&request.r2_key)
            .content_type(request.content_type.as_str())
            .presigned(presigning_config)
            .await
            .map_err(|error| PhotoUploadUrlError::Presign(error.to_string()))?;
        let mut required_headers = presigned
            .headers()
            .map(|(name, value)| UploadHeader {
                name: name.to_owned(),
                value: value.to_owned(),
            })
            .collect::<Vec<_>>();
        required_headers.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(PhotoUploadUrl {
            url: presigned.uri().to_owned(),
            required_headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use listing_photo_domain::entity::PhotoContentType;

    use super::{
        DisabledListingPhotoUploadUrlIssuer, ListingPhotoUploadUrlIssuer, PhotoUploadUrlError,
        PhotoUploadUrlRequest, R2ListingPhotoUploadUrlIssuer,
    };
    use crate::r2_raw_capture::R2RawCaptureConfig;

    fn r2_config() -> R2RawCaptureConfig {
        R2RawCaptureConfig {
            account_id: "account-id".to_owned(),
            access_key: "access-key".to_owned(),
            secret_key: "secret-key".to_owned(),
            bucket: "listing-photos".to_owned(),
            bronze_prefix: "bronze".to_owned(),
            fallback_dir: None,
        }
    }

    #[tokio::test]
    async fn disabled_issuer_fails_without_mock_url() {
        let result = DisabledListingPhotoUploadUrlIssuer
            .issue_upload_url(PhotoUploadUrlRequest {
                r2_key: "listings/lst_1/lph_1.jpg".to_owned(),
                content_type: PhotoContentType::Jpeg,
            })
            .await;

        assert!(matches!(result, Err(PhotoUploadUrlError::Disabled)));
    }

    #[tokio::test]
    async fn r2_issuer_presigns_put_with_required_content_type_header() {
        let issuer = R2ListingPhotoUploadUrlIssuer::new(r2_config());
        let result = issuer
            .issue_upload_url(PhotoUploadUrlRequest {
                r2_key: "listings/lst_1/lph_1.jpg".to_owned(),
                content_type: PhotoContentType::Jpeg,
            })
            .await;

        assert!(result.is_ok(), "expected presigned URL");
        if let Ok(upload) = result {
            assert!(upload
                .url
                .starts_with("https://account-id.r2.cloudflarestorage.com/"));
            assert!(upload.url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
            assert!(upload.url.contains("x-id=PutObject"));
            assert!(upload.required_headers.iter().any(|header| {
                header.name == "content-type" && header.value == PhotoContentType::Jpeg.as_str()
            }));
            assert!(!upload.url.starts_with("MOCK://"));
        }
    }
}
