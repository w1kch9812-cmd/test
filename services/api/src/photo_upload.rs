use std::env;
use std::time::Duration;

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, Region, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use listing_photo_domain::entity::PhotoContentType;
use serde::Serialize;
use thiserror::Error;

const LISTING_PHOTO_UPLOAD_EXPIRES: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct ListingPhotoUploadConfig {
    pub account_id: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Debug, Error)]
pub enum ListingPhotoUploadConfigError {
    #[error("env {0} not set")]
    MissingEnv(&'static str),
    #[error("env {0} empty")]
    EmptyEnv(&'static str),
}

impl ListingPhotoUploadConfig {
    pub fn from_env() -> Result<Self, ListingPhotoUploadConfigError> {
        Ok(Self {
            account_id: require_env("LISTING_PHOTO_R2_ACCOUNT_ID")?,
            access_key: require_env("LISTING_PHOTO_R2_ACCESS_KEY")?,
            secret_key: require_env("LISTING_PHOTO_R2_SECRET_KEY")?,
            bucket: require_env("LISTING_PHOTO_R2_BUCKET")?,
        })
    }

    #[must_use]
    pub fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }

    #[must_use]
    pub fn s3_client(&self) -> S3Client {
        let creds = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "api-listing-photo-upload",
        );
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(self.endpoint_url())
            .credentials_provider(creds)
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(1))
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_attempt_timeout(Duration::from_secs(15))
                    .build(),
            )
            .build();
        S3Client::from_conf(s3_config)
    }
}

fn require_env(name: &'static str) -> Result<String, ListingPhotoUploadConfigError> {
    match env::var(name) {
        Ok(v) if v.trim().is_empty() => Err(ListingPhotoUploadConfigError::EmptyEnv(name)),
        Ok(v) => Ok(v),
        Err(_) => Err(ListingPhotoUploadConfigError::MissingEnv(name)),
    }
}

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
    pub fn new(config: ListingPhotoUploadConfig) -> Self {
        Self {
            client: config.s3_client(),
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

    use super::ListingPhotoUploadConfig;
    use super::{
        DisabledListingPhotoUploadUrlIssuer, ListingPhotoUploadUrlIssuer, PhotoUploadUrlError,
        PhotoUploadUrlRequest, R2ListingPhotoUploadUrlIssuer,
    };

    fn r2_config() -> ListingPhotoUploadConfig {
        ListingPhotoUploadConfig {
            account_id: "account-id".to_owned(),
            access_key: "access-key".to_owned(),
            secret_key: "secret-key".to_owned(),
            bucket: "listing-photos".to_owned(),
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
