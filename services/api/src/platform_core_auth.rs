//! Service-to-service authentication for Gongzzang -> Platform Core calls.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

const REQUIRED_SCOPE: &str = "catalog:read";
const POLICY_ID_HEADER: &str = "x-gongzzang-service-auth-policy-id";
const SOURCE_SERVICE_HEADER: &str = "x-gongzzang-service-auth-source";
const TARGET_SERVICE_HEADER: &str = "x-gongzzang-service-auth-target";
const ALLOWED_CALL_ID_HEADER: &str = "x-gongzzang-allowed-call-id";
const SCOPE_HEADER: &str = "x-gongzzang-service-auth-scope";
const ISSUED_AT_HEADER: &str = "x-gongzzang-service-auth-issued-at";
const EXPIRES_AT_HEADER: &str = "x-gongzzang-service-auth-expires-at";
const ROTATION_OWNER_HEADER: &str = "x-gongzzang-service-auth-rotation-owner";
const POLICY_ID: &str = "gongzzang_api_to_platform_core_api";
const SOURCE_SERVICE: &str = "gongzzang-api";
const TARGET_SERVICE: &str = "platform-core-api";
const ALLOWED_CALL_ID: &str = "gongzzang_api_to_platform_core_catalog_read";
const WORKLOAD_IDENTITY_REFRESH_BEHAVIOR: &str = "read_before_each_request";
const MAX_TOKEN_TTL_DAYS: i64 = 90;

/// Redacted bearer-token auth used for Platform Core service calls.
#[derive(Clone)]
pub struct PlatformCoreServiceAuth {
    token_source: PlatformCoreServiceAuthTokenSource,
    metadata: Option<PlatformCoreServiceAuthMetadata>,
}

/// Source for the outbound Platform Core bearer credential.
#[derive(Clone)]
enum PlatformCoreServiceAuthTokenSource {
    Static(Arc<str>),
    WorkloadIdentityTokenFile(Arc<PathBuf>),
}

/// Operator-managed metadata for the current Platform Core service token.
#[derive(Clone)]
struct PlatformCoreServiceAuthMetadata {
    scope: Arc<str>,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    rotation_owner: Arc<str>,
}

/// Environment-sourced Platform Core service token metadata.
#[derive(Clone, Debug, Default)]
pub struct PlatformCoreServiceAuthMetadataConfig {
    /// Token scope. Production requires `catalog:read`.
    pub scope: Option<String>,
    /// RFC3339 timestamp when the token was issued.
    pub issued_at: Option<String>,
    /// RFC3339 timestamp when the token expires.
    pub expires_at: Option<String>,
    /// Team or operator responsible for rotation.
    pub rotation_owner: Option<String>,
}

impl PlatformCoreServiceAuth {
    /// Build service auth from a secret token.
    ///
    /// # Errors
    ///
    /// Returns an error when the token is blank or too short to be an
    /// operator-managed secret.
    #[cfg(test)]
    pub fn new(token: &str) -> Result<Self, PlatformCoreServiceAuthConfigError> {
        let token = validate_token(token)?;
        Ok(Self {
            token_source: PlatformCoreServiceAuthTokenSource::Static(Arc::from(token)),
            metadata: None,
        })
    }

    /// Build service auth from a short-lived workload identity token file.
    ///
    /// The token file is read before each outgoing request so service-mesh or
    /// cloud workload identity rotation can take effect without restarting the
    /// API process.
    pub fn new_from_workload_identity_token_file(
        token_file: impl AsRef<Path>,
    ) -> Result<Self, PlatformCoreServiceAuthConfigError> {
        tracing::debug!(
            refresh_behavior = WORKLOAD_IDENTITY_REFRESH_BEHAVIOR,
            "platform core workload identity token file configured"
        );
        let token_file = normalize_workload_identity_token_file_path(token_file.as_ref())?;
        let token = read_workload_identity_token(&token_file)?;
        validate_token(&token)?;
        Ok(Self {
            token_source: PlatformCoreServiceAuthTokenSource::WorkloadIdentityTokenFile(Arc::new(
                token_file,
            )),
            metadata: None,
        })
    }

    /// Build service auth with metadata enforcement for the current runtime.
    ///
    /// Production requires scope, issued-at, expires-at, and rotation-owner
    /// metadata so static bearer tokens cannot become unbounded credentials.
    pub fn new_for_environment(
        token: &str,
        metadata: PlatformCoreServiceAuthMetadataConfig,
        is_production: bool,
    ) -> Result<Self, PlatformCoreServiceAuthConfigError> {
        Self::new_for_environment_at(token, metadata, is_production, Utc::now())
    }

    fn new_for_environment_at(
        token: &str,
        metadata: PlatformCoreServiceAuthMetadataConfig,
        is_production: bool,
        now: DateTime<Utc>,
    ) -> Result<Self, PlatformCoreServiceAuthConfigError> {
        let token = validate_token(token)?;
        let metadata = PlatformCoreServiceAuthMetadata::from_config(metadata, is_production, now)?;
        Ok(Self {
            token_source: PlatformCoreServiceAuthTokenSource::Static(Arc::from(token)),
            metadata,
        })
    }

    /// Apply the service token to an outgoing Platform Core HTTP request.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured workload identity token file cannot
    /// be read or contains an invalid token.
    pub fn apply(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, PlatformCoreServiceAuthConfigError> {
        let token = self.token_for_request()?;
        let request = request
            .bearer_auth(token)
            .header(POLICY_ID_HEADER, POLICY_ID)
            .header(SOURCE_SERVICE_HEADER, SOURCE_SERVICE)
            .header(TARGET_SERVICE_HEADER, TARGET_SERVICE)
            .header(ALLOWED_CALL_ID_HEADER, ALLOWED_CALL_ID);
        if let Some(metadata) = &self.metadata {
            return Ok(request
                .header(SCOPE_HEADER, metadata.scope.as_ref())
                .header(ISSUED_AT_HEADER, metadata.issued_at.to_rfc3339())
                .header(EXPIRES_AT_HEADER, metadata.expires_at.to_rfc3339())
                .header(ROTATION_OWNER_HEADER, metadata.rotation_owner.as_ref()));
        }
        Ok(request)
    }

    fn token_for_request(&self) -> Result<String, PlatformCoreServiceAuthConfigError> {
        match &self.token_source {
            PlatformCoreServiceAuthTokenSource::Static(token) => Ok(token.to_string()),
            PlatformCoreServiceAuthTokenSource::WorkloadIdentityTokenFile(token_file) => {
                let token = read_workload_identity_token(token_file)?;
                validate_token(&token)?;
                Ok(token)
            }
        }
    }
}

impl PlatformCoreServiceAuthMetadata {
    fn from_config(
        config: PlatformCoreServiceAuthMetadataConfig,
        is_production: bool,
        now: DateTime<Utc>,
    ) -> Result<Option<Self>, PlatformCoreServiceAuthConfigError> {
        let any_metadata = config.scope.is_some()
            || config.issued_at.is_some()
            || config.expires_at.is_some()
            || config.rotation_owner.is_some();
        if !is_production && !any_metadata {
            return Ok(None);
        }

        let scope = required_metadata(
            config.scope,
            "PLATFORM_CORE_SERVICE_TOKEN_SCOPE",
            is_production,
        )?;
        let issued_at = required_metadata(
            config.issued_at,
            "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT",
            is_production,
        )?;
        let expires_at = required_metadata(
            config.expires_at,
            "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT",
            is_production,
        )?;
        let rotation_owner = required_metadata(
            config.rotation_owner,
            "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER",
            is_production,
        )?;

        if scope != REQUIRED_SCOPE {
            return Err(PlatformCoreServiceAuthConfigError::UnsupportedScope { scope });
        }
        let issued_at =
            parse_metadata_timestamp("PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT", &issued_at)?;
        let expires_at =
            parse_metadata_timestamp("PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT", &expires_at)?;
        if issued_at >= expires_at {
            return Err(PlatformCoreServiceAuthConfigError::MetadataIssuedAfterExpiry);
        }
        if issued_at > now {
            return Err(PlatformCoreServiceAuthConfigError::MetadataIssuedInFuture);
        }
        if expires_at <= now {
            return Err(PlatformCoreServiceAuthConfigError::ExpiredTokenMetadata);
        }
        if expires_at - issued_at > Duration::days(MAX_TOKEN_TTL_DAYS) {
            return Err(PlatformCoreServiceAuthConfigError::MetadataTtlTooLong);
        }

        Ok(Some(Self {
            scope: Arc::from(scope),
            issued_at,
            expires_at,
            rotation_owner: Arc::from(rotation_owner),
        }))
    }
}

fn validate_token(token: &str) -> Result<&str, PlatformCoreServiceAuthConfigError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(PlatformCoreServiceAuthConfigError::EmptyToken);
    }
    if token.len() < 16 {
        return Err(PlatformCoreServiceAuthConfigError::TokenTooShort);
    }
    Ok(token)
}

fn normalize_workload_identity_token_file_path(
    token_file: &Path,
) -> Result<PathBuf, PlatformCoreServiceAuthConfigError> {
    let path = token_file.as_os_str().to_string_lossy().trim().to_owned();
    if path.is_empty() {
        return Err(PlatformCoreServiceAuthConfigError::EmptyWorkloadIdentityTokenFilePath);
    }
    Ok(PathBuf::from(path))
}

fn read_workload_identity_token(
    token_file: &Path,
) -> Result<String, PlatformCoreServiceAuthConfigError> {
    std::fs::read_to_string(token_file)
        .map(|token| token.trim().to_owned())
        .map_err(
            |source| PlatformCoreServiceAuthConfigError::ReadWorkloadIdentityTokenFile {
                path: token_file.display().to_string(),
                source,
            },
        )
}

fn required_metadata(
    value: Option<String>,
    name: &'static str,
    is_production: bool,
) -> Result<String, PlatformCoreServiceAuthConfigError> {
    let Some(value) = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return if is_production {
            Err(PlatformCoreServiceAuthConfigError::MissingMetadata { name })
        } else {
            Err(PlatformCoreServiceAuthConfigError::PartialMetadata { name })
        };
    };
    Ok(value)
}

fn parse_metadata_timestamp(
    name: &'static str,
    value: &str,
) -> Result<DateTime<Utc>, PlatformCoreServiceAuthConfigError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(
            |error| PlatformCoreServiceAuthConfigError::InvalidMetadata {
                name,
                detail: error.to_string(),
            },
        )
}

impl std::fmt::Debug for PlatformCoreServiceAuth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformCoreServiceAuth")
            .field("token_source", &self.token_source)
            .field("metadata", &self.metadata.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl std::fmt::Debug for PlatformCoreServiceAuthTokenSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(_) => formatter.write_str("Static(<redacted>)"),
            Self::WorkloadIdentityTokenFile(path) => formatter
                .debug_tuple("WorkloadIdentityTokenFile")
                .field(path)
                .finish(),
        }
    }
}

impl std::fmt::Debug for PlatformCoreServiceAuthMetadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformCoreServiceAuthMetadata")
            .field("scope", &self.scope)
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .field("rotation_owner", &"<redacted>")
            .finish()
    }
}

/// Configuration error for Platform Core service auth.
#[derive(Debug, Error)]
pub enum PlatformCoreServiceAuthConfigError {
    /// Token value is present but blank.
    #[error("PLATFORM_CORE_SERVICE_TOKEN must not be empty")]
    EmptyToken,
    /// Token value is too short for production service identity.
    #[error("PLATFORM_CORE_SERVICE_TOKEN must be at least 16 characters")]
    TokenTooShort,
    /// Workload identity token file path is blank.
    #[error("PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE must not be empty")]
    EmptyWorkloadIdentityTokenFilePath,
    /// Workload identity token file could not be read.
    #[error("PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE could not be read: {path}")]
    ReadWorkloadIdentityTokenFile {
        /// Token file path.
        path: String,
        /// Underlying file read error.
        #[source]
        source: std::io::Error,
    },
    /// Production token metadata is missing.
    #[error("{name} must be set with PLATFORM_CORE_SERVICE_TOKEN in production")]
    MissingMetadata {
        /// Environment variable name.
        name: &'static str,
    },
    /// Partial token metadata was supplied outside production.
    #[error("{name} must be set when any Platform Core service token metadata is configured")]
    PartialMetadata {
        /// Environment variable name.
        name: &'static str,
    },
    /// Token metadata is malformed.
    #[error("{name} is invalid: {detail}")]
    InvalidMetadata {
        /// Environment variable name.
        name: &'static str,
        /// Parse or validation detail.
        detail: String,
    },
    /// Token scope is not the only supported Platform Core read scope.
    #[error("PLATFORM_CORE_SERVICE_TOKEN_SCOPE must be catalog:read, got {scope}")]
    UnsupportedScope {
        /// Configured scope.
        scope: String,
    },
    /// Token issue timestamp is after or equal to expiry.
    #[error("PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT must be before PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT")]
    MetadataIssuedAfterExpiry,
    /// Token issue timestamp is ahead of the API service clock.
    #[error("PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT must not be in the future")]
    MetadataIssuedInFuture,
    /// Token metadata TTL exceeds the rotation policy.
    #[error("PLATFORM_CORE_SERVICE_TOKEN metadata TTL must be 90 days or lower")]
    MetadataTtlTooLong,
    /// Token metadata has expired.
    #[error("PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT is expired")]
    ExpiredTokenMetadata,
}

#[cfg(test)]
mod tests {
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

        std::fs::write(&token_file, "workload-identity-token-32-second")
            .expect("rotate token file");
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
}
