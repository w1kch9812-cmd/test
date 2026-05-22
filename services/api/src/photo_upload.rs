use std::env;
use std::str::FromStr;
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

#[derive(Debug, Clone)]
pub struct PhotoDownloadUrlRequest {
    pub r2_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoDownloadUrl {
    pub url: String,
}

#[derive(Debug, Error)]
pub enum PhotoDownloadUrlError {
    #[error("listing photo download storage is not configured")]
    Disabled,
    #[error("listing photo download presigning config: {0}")]
    PresigningConfig(String),
    #[error("listing photo download presign failed: {0}")]
    Presign(String),
}

#[async_trait]
pub trait ListingPhotoDownloadUrlIssuer: Send + Sync {
    async fn issue_download_url(
        &self,
        request: PhotoDownloadUrlRequest,
    ) -> Result<PhotoDownloadUrl, PhotoDownloadUrlError>;
}

#[derive(Debug, Default)]
pub struct DisabledListingPhotoDownloadUrlIssuer;

#[async_trait]
impl ListingPhotoDownloadUrlIssuer for DisabledListingPhotoDownloadUrlIssuer {
    async fn issue_download_url(
        &self,
        _request: PhotoDownloadUrlRequest,
    ) -> Result<PhotoDownloadUrl, PhotoDownloadUrlError> {
        Err(PhotoDownloadUrlError::Disabled)
    }
}

#[derive(Debug, Clone)]
pub struct PhotoObjectVerifyRequest {
    pub r2_key: String,
    pub expected_content_type: PhotoContentType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoObjectMetadata {
    pub file_size_bytes: i64,
    pub content_type: PhotoContentType,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PhotoObjectVerifyError {
    #[error("listing photo upload storage is not configured")]
    Disabled,
    #[error("listing photo object not found")]
    NotFound,
    #[error("listing photo object content-length missing")]
    MissingContentLength,
    #[error("listing photo object content-length must be > 0 (got {actual})")]
    InvalidContentLength { actual: i64 },
    #[error("listing photo object content-type missing")]
    MissingContentType,
    #[error("listing photo object content-type mismatch: expected {expected}, got {actual}")]
    ContentTypeMismatch {
        expected: PhotoContentType,
        actual: String,
    },
    #[error("listing photo object HEAD failed: {0}")]
    Head(String),
}

#[async_trait]
pub trait ListingPhotoObjectVerifier: Send + Sync {
    async fn verify_uploaded_object(
        &self,
        request: PhotoObjectVerifyRequest,
    ) -> Result<PhotoObjectMetadata, PhotoObjectVerifyError>;
}

#[derive(Debug, Default)]
pub struct DisabledListingPhotoObjectVerifier;

#[async_trait]
impl ListingPhotoObjectVerifier for DisabledListingPhotoObjectVerifier {
    async fn verify_uploaded_object(
        &self,
        _request: PhotoObjectVerifyRequest,
    ) -> Result<PhotoObjectMetadata, PhotoObjectVerifyError> {
        Err(PhotoObjectVerifyError::Disabled)
    }
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

#[derive(Debug, Clone)]
pub struct R2ListingPhotoDownloadUrlIssuer {
    client: S3Client,
    bucket: String,
    expires_in: Duration,
}

impl R2ListingPhotoDownloadUrlIssuer {
    #[must_use]
    pub fn new(config: ListingPhotoUploadConfig) -> Self {
        Self {
            client: config.s3_client(),
            bucket: config.bucket,
            expires_in: LISTING_PHOTO_UPLOAD_EXPIRES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct R2ListingPhotoObjectVerifier {
    client: S3Client,
    bucket: String,
}

impl R2ListingPhotoObjectVerifier {
    #[must_use]
    pub fn new(config: ListingPhotoUploadConfig) -> Self {
        Self {
            client: config.s3_client(),
            bucket: config.bucket,
        }
    }
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

#[async_trait]
impl ListingPhotoDownloadUrlIssuer for R2ListingPhotoDownloadUrlIssuer {
    async fn issue_download_url(
        &self,
        request: PhotoDownloadUrlRequest,
    ) -> Result<PhotoDownloadUrl, PhotoDownloadUrlError> {
        let presigning_config = PresigningConfig::expires_in(self.expires_in)
            .map_err(|error| PhotoDownloadUrlError::PresigningConfig(error.to_string()))?;
        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&request.r2_key)
            .presigned(presigning_config)
            .await
            .map_err(|error| PhotoDownloadUrlError::Presign(error.to_string()))?;
        Ok(PhotoDownloadUrl {
            url: presigned.uri().to_owned(),
        })
    }
}

#[async_trait]
impl ListingPhotoObjectVerifier for R2ListingPhotoObjectVerifier {
    async fn verify_uploaded_object(
        &self,
        request: PhotoObjectVerifyRequest,
    ) -> Result<PhotoObjectMetadata, PhotoObjectVerifyError> {
        let output =
            self.client
                .head_object()
                .bucket(&self.bucket)
                .key(&request.r2_key)
                .send()
                .await
                .map_err(|error| {
                    if error.as_service_error().is_some_and(
                        aws_sdk_s3::operation::head_object::HeadObjectError::is_not_found,
                    ) {
                        return PhotoObjectVerifyError::NotFound;
                    }
                    PhotoObjectVerifyError::Head(format!(
                        "{}",
                        aws_sdk_s3::error::DisplayErrorContext(&error)
                    ))
                })?;

        verified_object_metadata_from_head(
            request.expected_content_type,
            output.content_length(),
            output.content_type(),
        )
    }
}

fn verified_object_metadata_from_head(
    expected_content_type: PhotoContentType,
    content_length: Option<i64>,
    content_type: Option<&str>,
) -> Result<PhotoObjectMetadata, PhotoObjectVerifyError> {
    let file_size_bytes = content_length.ok_or(PhotoObjectVerifyError::MissingContentLength)?;
    if file_size_bytes <= 0 {
        return Err(PhotoObjectVerifyError::InvalidContentLength {
            actual: file_size_bytes,
        });
    }
    let actual_content_type = content_type.ok_or(PhotoObjectVerifyError::MissingContentType)?;
    let actual = PhotoContentType::from_str(actual_content_type).map_err(|_| {
        PhotoObjectVerifyError::ContentTypeMismatch {
            expected: expected_content_type,
            actual: actual_content_type.to_owned(),
        }
    })?;
    if actual != expected_content_type {
        return Err(PhotoObjectVerifyError::ContentTypeMismatch {
            expected: expected_content_type,
            actual: actual_content_type.to_owned(),
        });
    }
    Ok(PhotoObjectMetadata {
        file_size_bytes,
        content_type: actual,
    })
}

#[cfg(test)]
mod tests {
    use listing_photo_domain::entity::PhotoContentType;

    use super::ListingPhotoUploadConfig;
    use super::{
        verified_object_metadata_from_head, DisabledListingPhotoDownloadUrlIssuer,
        DisabledListingPhotoObjectVerifier, DisabledListingPhotoUploadUrlIssuer,
        ListingPhotoDownloadUrlIssuer, ListingPhotoObjectVerifier, ListingPhotoUploadUrlIssuer,
        PhotoDownloadUrlError, PhotoDownloadUrlRequest, PhotoObjectVerifyError,
        PhotoObjectVerifyRequest, PhotoUploadUrlError, PhotoUploadUrlRequest,
        R2ListingPhotoDownloadUrlIssuer, R2ListingPhotoUploadUrlIssuer,
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
    async fn disabled_object_verifier_fails_without_mock_success() {
        let result = DisabledListingPhotoObjectVerifier
            .verify_uploaded_object(PhotoObjectVerifyRequest {
                r2_key: "listings/lst_1/lph_1.jpg".to_owned(),
                expected_content_type: PhotoContentType::Jpeg,
            })
            .await;

        assert!(matches!(result, Err(PhotoObjectVerifyError::Disabled)));
    }

    #[tokio::test]
    async fn disabled_download_issuer_fails_without_mock_url() {
        let result = DisabledListingPhotoDownloadUrlIssuer
            .issue_download_url(PhotoDownloadUrlRequest {
                r2_key: "listings/lst_1/lph_1.jpg".to_owned(),
            })
            .await;

        assert!(matches!(result, Err(PhotoDownloadUrlError::Disabled)));
    }

    #[test]
    fn verified_object_metadata_requires_positive_content_length() {
        let result = verified_object_metadata_from_head(
            PhotoContentType::Jpeg,
            Some(0),
            Some(PhotoContentType::Jpeg.as_str()),
        );

        assert!(matches!(
            result,
            Err(PhotoObjectVerifyError::InvalidContentLength { actual: 0 })
        ));
    }

    #[test]
    fn verified_object_metadata_rejects_content_type_mismatch() {
        let result = verified_object_metadata_from_head(
            PhotoContentType::Jpeg,
            Some(100),
            Some(PhotoContentType::Png.as_str()),
        );

        assert!(matches!(
            result,
            Err(PhotoObjectVerifyError::ContentTypeMismatch {
                expected: PhotoContentType::Jpeg,
                ..
            })
        ));
    }

    #[test]
    fn verified_object_metadata_accepts_expected_metadata() {
        let result = verified_object_metadata_from_head(
            PhotoContentType::Webp,
            Some(100),
            Some(PhotoContentType::Webp.as_str()),
        );

        assert!(result.is_ok(), "expected valid metadata");
        if let Ok(metadata) = result {
            assert_eq!(metadata.file_size_bytes, 100);
            assert_eq!(metadata.content_type, PhotoContentType::Webp);
        }
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

    #[tokio::test]
    async fn r2_download_issuer_presigns_get_without_mock_url() {
        let issuer = R2ListingPhotoDownloadUrlIssuer::new(r2_config());
        let result = issuer
            .issue_download_url(PhotoDownloadUrlRequest {
                r2_key: "listings/lst_1/lph_1.jpg".to_owned(),
            })
            .await;

        assert!(result.is_ok(), "expected presigned URL");
        if let Ok(download) = result {
            assert!(download
                .url
                .starts_with("https://account-id.r2.cloudflarestorage.com/"));
            assert!(download.url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
            assert!(download.url.contains("x-id=GetObject"));
            assert!(!download.url.starts_with("MOCK://"));
        }
    }
}
