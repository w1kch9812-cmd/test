use auth::verifier::Verifier;

use crate::photo_upload::ListingPhotoUploadConfigError;

use super::{
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
