#![allow(clippy::disallowed_types, clippy::expect_used)]

use super::*;

#[test]
fn production_service_auth_rejects_missing_metadata() {
    let result = PlatformCoreServiceAuth::new_for_environment(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig::default(),
        true,
    );

    assert!(
        matches!(result, Err(PlatformCoreServiceAuthConfigError::MissingMetadata { name })
            if name == "PLATFORM_CORE_SERVICE_TOKEN_SCOPE")
    );
}

#[test]
fn production_service_auth_rejects_expired_metadata() {
    let result = PlatformCoreServiceAuth::new_for_environment(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-05-02T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
    );

    assert!(matches!(
        result,
        Err(PlatformCoreServiceAuthConfigError::ExpiredTokenMetadata)
    ));
}

#[test]
fn production_service_auth_rejects_future_issued_metadata() {
    let result = PlatformCoreServiceAuth::new_for_environment_at(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-03T00:00:00Z".to_owned()),
            expires_at: Some("2026-05-04T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
    );

    assert!(matches!(
        result,
        Err(PlatformCoreServiceAuthConfigError::MetadataIssuedInFuture)
    ));
}

#[test]
fn production_service_auth_rejects_metadata_ttl_longer_than_rotation_window() {
    let result = PlatformCoreServiceAuth::new_for_environment_at(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-09-01T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
    );

    assert!(matches!(
        result,
        Err(PlatformCoreServiceAuthConfigError::MetadataTtlTooLong)
    ));
}

#[test]
fn production_service_auth_accepts_scoped_unexpired_metadata() {
    let auth = PlatformCoreServiceAuth::new_for_environment_at(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-06-01T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
    )
    .expect("service auth");

    let request = auth
        .apply(reqwest::Client::new().get("http://127.0.0.1/health"))
        .expect("apply auth")
        .build()
        .expect("request");

    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-scope"),
        Some(&"catalog:read".parse().expect("header"))
    );
    assert_eq!(
        request
            .headers()
            .get("x-gongzzang-service-auth-rotation-owner"),
        Some(&"platform-security".parse().expect("header"))
    );
}

#[test]
fn production_service_auth_applies_default_deny_identity_headers() {
    let auth = PlatformCoreServiceAuth::new_for_environment_at(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-06-01T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
    )
    .expect("service auth");

    let request = auth
        .apply(reqwest::Client::new().get("http://127.0.0.1/health"))
        .expect("apply auth")
        .build()
        .expect("request");

    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-source"),
        Some(&"gongzzang-api".parse().expect("header"))
    );
    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-target"),
        Some(&"platform-core-api".parse().expect("header"))
    );
    assert_eq!(
        request.headers().get("x-gongzzang-allowed-call-id"),
        Some(
            &"gongzzang_api_to_platform_core_catalog_read"
                .parse()
                .expect("header")
        )
    );
}

#[test]
fn production_service_auth_accepts_lakehouse_registry_write_scope_for_worker_call_policy() {
    let auth = PlatformCoreServiceAuth::new_for_environment_at_with_call_policy(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("lakehouse:write".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-06-01T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
        PlatformCoreServiceCallPolicy::gongzzang_worker_lakehouse_registry_write(),
    )
    .expect("service auth");

    let request = auth
        .apply(reqwest::Client::new().get("http://127.0.0.1/health"))
        .expect("apply auth")
        .build()
        .expect("request");

    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-policy-id"),
        Some(
            &"gongzzang_worker_to_platform_core_api"
                .parse()
                .expect("header")
        )
    );
    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-source"),
        Some(&"gongzzang-worker".parse().expect("header"))
    );
    assert_eq!(
        request.headers().get("x-gongzzang-service-auth-scope"),
        Some(&"lakehouse:write".parse().expect("header"))
    );
    assert_eq!(
        request.headers().get("x-gongzzang-allowed-call-id"),
        Some(
            &"gongzzang_pipeline_to_platform_core_lakehouse_registry"
                .parse()
                .expect("header")
        )
    );
}

#[test]
fn production_service_auth_rejects_scope_that_does_not_match_call_policy() {
    let result = PlatformCoreServiceAuth::new_for_environment_at_with_call_policy(
        "platform-core-service-token-32-valid",
        PlatformCoreServiceAuthMetadataConfig {
            scope: Some("catalog:read".to_owned()),
            issued_at: Some("2026-05-01T00:00:00Z".to_owned()),
            expires_at: Some("2026-06-01T00:00:00Z".to_owned()),
            rotation_owner: Some("platform-security".to_owned()),
        },
        true,
        "2026-05-02T00:00:00Z".parse().expect("now"),
        PlatformCoreServiceCallPolicy::gongzzang_worker_lakehouse_registry_write(),
    );

    assert!(matches!(
        result,
        Err(PlatformCoreServiceAuthConfigError::UnsupportedScope {
            scope,
            required_scope
        }) if scope == "catalog:read" && required_scope == "lakehouse:write"
    ));
}

#[test]
fn workload_identity_token_file_is_read_before_each_request() {
    let token_file = std::env::temp_dir().join(format!(
        "gongzzang-platform-core-token-{}-{}.txt",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::write(&token_file, "workload-identity-token-32-first").expect("write token file");

    let auth = PlatformCoreServiceAuth::new_from_workload_identity_token_file(&token_file)
        .expect("workload identity auth");

    let first = auth
        .apply(reqwest::Client::new().get("http://127.0.0.1/health"))
        .expect("apply first token")
        .build()
        .expect("first request");
    assert_eq!(
        first.headers().get(reqwest::header::AUTHORIZATION),
        Some(
            &"Bearer workload-identity-token-32-first"
                .parse()
                .expect("header")
        )
    );

    std::fs::write(&token_file, "workload-identity-token-32-second").expect("rotate token file");
    let second = auth
        .apply(reqwest::Client::new().get("http://127.0.0.1/health"))
        .expect("apply rotated token")
        .build()
        .expect("second request");
    assert_eq!(
        second.headers().get(reqwest::header::AUTHORIZATION),
        Some(
            &"Bearer workload-identity-token-32-second"
                .parse()
                .expect("header")
        )
    );

    let _ = std::fs::remove_file(token_file);
}
