use thiserror::Error;

/// `Version` / `Srs` / `R2PublicBase` 생성 실패 모드.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// version 라벨이 빈 문자열.
    #[error("version must not be empty")]
    VersionEmpty,
    /// version 라벨 길이 초과 (64).
    #[error("version exceeds 64 chars: {0:?}")]
    VersionTooLong(String),
    /// version 라벨 형식 — `^v[a-z0-9_-]+$` 위반.
    #[error("version must match `^v[a-z0-9_-]+$` (e.g. v3, v_2026_05): {0:?}")]
    VersionFormat(String),
    /// SRS 가 빈 문자열.
    #[error("srs must not be empty")]
    SrsEmpty,
    /// SRS 형식 — `^EPSG:<digits>$` 위반.
    #[error("srs must match `^EPSG:<digits>$` (e.g. EPSG:4326): {0:?}")]
    SrsFormat(String),
    /// R2 public base URL 이 빈 문자열.
    #[error("R2 public base url must not be empty")]
    R2PublicBaseEmpty,
    /// R2 public base URL scheme 위반 (http/https 만 허용).
    #[error("R2 public base url must use http(s) scheme: {0:?}")]
    R2PublicBaseScheme(String),
    /// R2 public base URL host 부재.
    #[error("R2 public base url must have a host: {0:?}")]
    R2PublicBaseHost(String),
}

/// `ETL_ENVIRONMENT` 파싱 실패 모드.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EnvironmentParseError {
    /// env 자체 미설정 — operator 가 *명시 박제* 안 함.
    #[error(
        "ETL_ENVIRONMENT env is required (must be one of: local / staging / production). \
         set it in your .env or workflow yml. ADR 0029."
    )]
    Unset,
    /// env 가 빈 문자열.
    #[error("ETL_ENVIRONMENT must not be empty")]
    Empty,
    /// 알 수 없는 값.
    #[error("ETL_ENVIRONMENT={0:?} not recognized — expected one of: local, staging, production")]
    Invalid(String),
}
