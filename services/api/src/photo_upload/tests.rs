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
