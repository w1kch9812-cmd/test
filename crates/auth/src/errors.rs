//! `AuthError` — `401`/`403`/`500` 매핑.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// 인증/인가 실패 종류.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    /// `Authorization` 헤더가 없어요.
    #[error("missing Authorization header")]
    MissingToken,
    /// `Authorization` 헤더가 `Bearer ` 접두사로 시작하지 않거나 토큰 본문이 비어있어요.
    #[error("invalid Authorization format")]
    InvalidFormat,
    /// `JWT` 파싱 실패 (`base64`/`JSON` 깨짐).
    #[error("malformed token")]
    MalformedToken,
    /// `kid` 헤더에 매칭되는 공개키가 `JWKS` 에 없어요.
    #[error("unknown signing key (kid not found)")]
    UnknownKey,
    /// 서명 검증 실패.
    #[error("invalid signature")]
    InvalidSignature,
    /// `exp` 만료.
    #[error("token expired")]
    Expired,
    /// `nbf` 미도래.
    #[error("token not yet valid")]
    NotYetValid,
    /// `iss` 불일치.
    #[error("invalid issuer")]
    InvalidIssuer,
    /// `aud` 불일치.
    #[error("invalid audience")]
    InvalidAudience,
    /// `sub` claim 누락.
    #[error("missing subject claim")]
    MissingSubject,
    /// `User` 자동 생성 실패 (`DB` 또는 도메인 검증).
    #[error("user provisioning failed: {0}")]
    UserProvisioningFailed(String),
    /// 역할 부족.
    #[error("insufficient role")]
    InsufficientRole,
    /// `JWKS` 페치 실패.
    #[error("JWKS fetch failed: {0}")]
    JwksFetchFailed(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error_code: &'static str,
    message: &'static str,
}

impl AuthError {
    /// 응답 코드 (spec § 6.1).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingToken => "AUTH_MISSING_TOKEN",
            Self::InvalidFormat => "AUTH_INVALID_FORMAT",
            Self::MalformedToken => "AUTH_MALFORMED_TOKEN",
            Self::UnknownKey => "AUTH_UNKNOWN_KEY",
            Self::InvalidSignature | Self::JwksFetchFailed(_) => "AUTH_INVALID_SIGNATURE",
            Self::Expired => "AUTH_TOKEN_EXPIRED",
            Self::NotYetValid => "AUTH_TOKEN_NOT_YET_VALID",
            Self::InvalidIssuer => "AUTH_INVALID_ISSUER",
            Self::InvalidAudience => "AUTH_INVALID_AUDIENCE",
            Self::MissingSubject => "AUTH_MISSING_SUBJECT",
            Self::UserProvisioningFailed(_) => "AUTH_USER_PROVISION_FAILED",
            Self::InsufficientRole => "AUTH_INSUFFICIENT_ROLE",
        }
    }

    /// 한국어 해요체 메시지 (spec § 6.1).
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::MissingToken => "인증 토큰이 필요해요",
            Self::InvalidFormat => "토큰 형식이 잘못됐어요",
            Self::MalformedToken => "토큰을 해석할 수 없어요",
            Self::UnknownKey => "토큰 서명 키를 찾을 수 없어요",
            Self::InvalidSignature | Self::JwksFetchFailed(_) => "토큰이 유효하지 않아요",
            Self::Expired => "토큰이 만료됐어요. 다시 로그인해 주세요",
            Self::NotYetValid => "토큰이 아직 사용할 수 없어요",
            Self::InvalidIssuer => "토큰 발급자가 일치하지 않아요",
            Self::InvalidAudience => "토큰 대상이 일치하지 않아요",
            Self::MissingSubject => "토큰에 사용자 정보가 없어요",
            Self::UserProvisioningFailed(_) => {
                "사용자 등록에 실패했어요. 잠시 후 다시 시도해 주세요"
            }
            Self::InsufficientRole => "이 작업을 수행할 권한이 부족해요",
        }
    }

    /// `HTTP` 상태 코드.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::InsufficientRole => StatusCode::FORBIDDEN,
            Self::UserProvisioningFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::UNAUTHORIZED,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error_code: self.code(),
            message: self.message(),
        };
        (self.status(), Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn code_maps_each_variant() {
        assert_eq!(AuthError::MissingToken.code(), "AUTH_MISSING_TOKEN");
        assert_eq!(AuthError::Expired.code(), "AUTH_TOKEN_EXPIRED");
        assert_eq!(AuthError::InsufficientRole.code(), "AUTH_INSUFFICIENT_ROLE");
        assert_eq!(
            AuthError::UserProvisioningFailed("db".into()).code(),
            "AUTH_USER_PROVISION_FAILED"
        );
        assert_eq!(
            AuthError::JwksFetchFailed("net".into()).code(),
            "AUTH_INVALID_SIGNATURE"
        );
    }

    #[test]
    fn status_403_only_for_role() {
        assert_eq!(AuthError::InsufficientRole.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            AuthError::UserProvisioningFailed("x".into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(AuthError::Expired.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::MissingToken.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn message_uses_haeyo() {
        assert_eq!(
            AuthError::Expired.message(),
            "토큰이 만료됐어요. 다시 로그인해 주세요"
        );
        assert_eq!(AuthError::MissingToken.message(), "인증 토큰이 필요해요");
    }

    #[tokio::test]
    async fn into_response_shape() {
        let resp = AuthError::Expired.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(resp.into_body(), 1024).await.expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(parsed["error_code"], "AUTH_TOKEN_EXPIRED");
        assert_eq!(parsed["message"], "토큰이 만료됐어요. 다시 로그인해 주세요");
    }

    #[tokio::test]
    async fn into_response_role_is_403() {
        let resp = AuthError::InsufficientRole.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn code_for_all_variants() {
        assert_eq!(AuthError::MissingToken.code(), "AUTH_MISSING_TOKEN");
        assert_eq!(AuthError::InvalidFormat.code(), "AUTH_INVALID_FORMAT");
        assert_eq!(AuthError::MalformedToken.code(), "AUTH_MALFORMED_TOKEN");
        assert_eq!(AuthError::UnknownKey.code(), "AUTH_UNKNOWN_KEY");
        assert_eq!(AuthError::InvalidSignature.code(), "AUTH_INVALID_SIGNATURE");
        assert_eq!(AuthError::Expired.code(), "AUTH_TOKEN_EXPIRED");
        assert_eq!(AuthError::NotYetValid.code(), "AUTH_TOKEN_NOT_YET_VALID");
        assert_eq!(AuthError::InvalidIssuer.code(), "AUTH_INVALID_ISSUER");
        assert_eq!(AuthError::InvalidAudience.code(), "AUTH_INVALID_AUDIENCE");
        assert_eq!(AuthError::MissingSubject.code(), "AUTH_MISSING_SUBJECT");
        assert_eq!(
            AuthError::UserProvisioningFailed("x".into()).code(),
            "AUTH_USER_PROVISION_FAILED"
        );
        assert_eq!(AuthError::InsufficientRole.code(), "AUTH_INSUFFICIENT_ROLE");
        assert_eq!(
            AuthError::JwksFetchFailed("y".into()).code(),
            "AUTH_INVALID_SIGNATURE"
        );
    }

    #[test]
    fn message_for_all_variants() {
        assert_eq!(AuthError::MissingToken.message(), "인증 토큰이 필요해요");
        assert_eq!(AuthError::InvalidFormat.message(), "토큰 형식이 잘못됐어요");
        assert_eq!(
            AuthError::MalformedToken.message(),
            "토큰을 해석할 수 없어요"
        );
        assert_eq!(
            AuthError::UnknownKey.message(),
            "토큰 서명 키를 찾을 수 없어요"
        );
        assert_eq!(
            AuthError::InvalidSignature.message(),
            "토큰이 유효하지 않아요"
        );
        assert_eq!(
            AuthError::Expired.message(),
            "토큰이 만료됐어요. 다시 로그인해 주세요"
        );
        assert_eq!(
            AuthError::NotYetValid.message(),
            "토큰이 아직 사용할 수 없어요"
        );
        assert_eq!(
            AuthError::InvalidIssuer.message(),
            "토큰 발급자가 일치하지 않아요"
        );
        assert_eq!(
            AuthError::InvalidAudience.message(),
            "토큰 대상이 일치하지 않아요"
        );
        assert_eq!(
            AuthError::MissingSubject.message(),
            "토큰에 사용자 정보가 없어요"
        );
        assert_eq!(
            AuthError::UserProvisioningFailed("e".into()).message(),
            "사용자 등록에 실패했어요. 잠시 후 다시 시도해 주세요"
        );
        assert_eq!(
            AuthError::InsufficientRole.message(),
            "이 작업을 수행할 권한이 부족해요"
        );
        assert_eq!(
            AuthError::JwksFetchFailed("net".into()).message(),
            "토큰이 유효하지 않아요"
        );
    }

    #[test]
    fn status_for_all_variants() {
        // 401 cases — 모든 토큰 검증 실패는 UNAUTHORIZED
        assert_eq!(AuthError::MissingToken.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::InvalidFormat.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::MalformedToken.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::UnknownKey.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            AuthError::InvalidSignature.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AuthError::Expired.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::NotYetValid.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::InvalidIssuer.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            AuthError::InvalidAudience.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AuthError::MissingSubject.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            AuthError::JwksFetchFailed("x".into()).status(),
            StatusCode::UNAUTHORIZED
        );
        // 403
        assert_eq!(AuthError::InsufficientRole.status(), StatusCode::FORBIDDEN);
        // 500
        assert_eq!(
            AuthError::UserProvisioningFailed("db".into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn into_response_for_all_status_codes() {
        // 401 sample (provisioning-unrelated branch)
        let resp = AuthError::MissingToken.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let resp = AuthError::JwksFetchFailed("network".into()).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // 403
        let resp = AuthError::InsufficientRole.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // 500
        let resp = AuthError::UserProvisioningFailed("db".into()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
