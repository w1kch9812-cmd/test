#![allow(clippy::expect_used)]

use auth::verifier::Verifier;
use chrono::{Duration, Utc};

use crate::photo_upload::ListingPhotoUploadConfigError;
use auth::platform_core_service::PlatformCoreServiceAuthMetadataConfig;

use super::{
    build_building_reader_from_platform_core_base_url,
    build_building_reader_from_platform_core_base_url_with_service_auth_metadata,
    build_parcel_lookup_from_platform_core_base_url,
    build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata,
    build_photo_download_issuer_from_config_result, build_photo_object_verifier_from_config_result,
    build_photo_upload_issuer_from_config_result, build_verifier, required_env, StartupError,
};

#[test]
fn required_env_returns_typed_error_when_missing() {
    const NAME: &str = "GONGZZANG_TEST_REQUIRED_ENV";
    std::env::remove_var(NAME);

    let result = required_env(NAME);

    assert!(matches!(result, Err(StartupError::MissingEnv { name }) if name == NAME));
}

#[test]
fn production_rejects_auth_dev_mode() {
    let result = build_verifier(true, true);

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason }) if reason.contains("AUTH_DEV_MODE"))
    );
}

#[test]
fn non_production_allows_auth_dev_mode() {
    let result = build_verifier(true, false);

    assert!(result.is_ok(), "expected dev verifier");
    if let Ok(verifier) = result {
        assert!(matches!(verifier.as_ref(), Verifier::Dev));
    }
}

#[test]
fn production_rejects_missing_listing_photo_upload_r2_config() {
    let result = build_photo_upload_issuer_from_config_result(
        true,
        Err(ListingPhotoUploadConfigError::MissingEnv(
            "LISTING_PHOTO_R2_BUCKET",
        )),
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("listing photo upload")
                && reason.contains("LISTING_PHOTO_R2_BUCKET"))
    );
}

#[test]
fn production_rejects_missing_listing_photo_object_verifier_r2_config() {
    let result = build_photo_object_verifier_from_config_result(
        true,
        Err(ListingPhotoUploadConfigError::MissingEnv(
            "LISTING_PHOTO_R2_BUCKET",
        )),
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("listing photo object verifier")
                && reason.contains("LISTING_PHOTO_R2_BUCKET"))
    );
}

#[test]
fn non_production_allows_disabled_listing_photo_object_verifier() {
    let result = build_photo_object_verifier_from_config_result(
        false,
        Err(ListingPhotoUploadConfigError::MissingEnv(
            "LISTING_PHOTO_R2_BUCKET",
        )),
    );

    assert!(result.is_ok());
}

#[test]
fn production_rejects_missing_platform_core_building_base_url() {
    let result = build_building_reader_from_platform_core_base_url(true, None, None);

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_API_BASE_URL"))
    );
}

#[test]
fn non_production_allows_missing_platform_core_building_base_url() {
    let result = build_building_reader_from_platform_core_base_url(false, None, None);

    assert!(result.is_ok());
}

#[test]
fn accepts_platform_core_building_base_url() {
    let result = build_building_reader_from_platform_core_base_url_with_service_auth_metadata(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        None,
        Some("platform-core-service-token-32-valid".to_owned()),
        service_auth_metadata_fixture(),
    );

    assert!(result.is_ok());
}

#[test]
fn production_rejects_platform_core_building_token_without_metadata() {
    let result = build_building_reader_from_platform_core_base_url(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        Some("platform-core-service-token-32-valid".to_owned()),
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_SERVICE_TOKEN_SCOPE"))
    );
}

#[test]
fn production_rejects_missing_platform_core_service_token_for_building_reader() {
    let result = build_building_reader_from_platform_core_base_url(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        None,
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_SERVICE_TOKEN"))
    );
}

#[test]
fn production_rejects_missing_platform_core_parcel_base_url() {
    let result = build_parcel_lookup_from_platform_core_base_url(true, None, None);

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_API_BASE_URL"))
    );
}

#[test]
fn non_production_allows_missing_platform_core_parcel_base_url() {
    let result = build_parcel_lookup_from_platform_core_base_url(false, None, None);

    assert!(result.is_ok());
}

#[test]
fn accepts_platform_core_parcel_base_url() {
    let result = build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        None,
        Some("platform-core-service-token-32-valid".to_owned()),
        service_auth_metadata_fixture(),
    );

    assert!(result.is_ok());
}

#[test]
fn production_accepts_platform_core_parcel_workload_identity_token_file() {
    let token_file = write_workload_identity_token_file("workload-identity-token-32-valid");
    let result = build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        Some(token_file.to_string_lossy().into_owned()),
        None,
        PlatformCoreServiceAuthMetadataConfig::default(),
    );

    let _ = std::fs::remove_file(token_file);
    assert!(result.is_ok());
}

#[test]
fn production_rejects_ambiguous_platform_core_token_sources() {
    let token_file = write_workload_identity_token_file("workload-identity-token-32-valid");
    let result = build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        Some(token_file.to_string_lossy().into_owned()),
        Some("platform-core-service-token-32-valid".to_owned()),
        service_auth_metadata_fixture(),
    );

    let _ = std::fs::remove_file(token_file);
    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE")
                && reason.contains("PLATFORM_CORE_SERVICE_TOKEN"))
    );
}

#[test]
fn production_rejects_platform_core_parcel_token_without_metadata() {
    let result = build_parcel_lookup_from_platform_core_base_url(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        Some("platform-core-service-token-32-valid".to_owned()),
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_SERVICE_TOKEN_SCOPE"))
    );
}

#[test]
fn production_rejects_missing_platform_core_service_token_for_parcel_lookup() {
    let result = build_parcel_lookup_from_platform_core_base_url(
        true,
        Some("http://127.0.0.1:18080".to_owned()),
        None,
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("PLATFORM_CORE_SERVICE_TOKEN"))
    );
}

#[test]
fn production_rejects_missing_listing_photo_download_r2_config() {
    let result = build_photo_download_issuer_from_config_result(
        true,
        Err(ListingPhotoUploadConfigError::MissingEnv(
            "LISTING_PHOTO_R2_BUCKET",
        )),
    );

    assert!(
        matches!(result, Err(StartupError::ProductionConfig { reason })
            if reason.contains("listing photo download")
                && reason.contains("LISTING_PHOTO_R2_BUCKET"))
    );
}

fn service_auth_metadata_fixture() -> PlatformCoreServiceAuthMetadataConfig {
    let issued_at = Utc::now() - Duration::days(1);
    let expires_at = Utc::now() + Duration::days(30);
    PlatformCoreServiceAuthMetadataConfig {
        scope: Some("catalog:read".to_owned()),
        issued_at: Some(issued_at.to_rfc3339()),
        expires_at: Some(expires_at.to_rfc3339()),
        rotation_owner: Some("platform-security".to_owned()),
    }
}

fn write_workload_identity_token_file(token: &str) -> std::path::PathBuf {
    let token_file = std::env::temp_dir().join(format!(
        "gongzzang-startup-token-{}-{}.txt",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::write(&token_file, token).expect("write workload identity token file");
    token_file
}
