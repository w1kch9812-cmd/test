use std::fmt;

use serde::Serialize;

use super::EnvironmentParseError;

/// ETL 실행 환경 — Round 5+ (ADR 0029) 의 명시 분리 SSOT.
///
/// `ETL_ENVIRONMENT` env 가 *명시* 선언 필수 (미설정 시 fail-fast). 각 env 별
/// secret namespace 격리 — local 이 prod credential 자동 활성 차단.
///
/// 추론 (R2_* 자격 존재 여부 만으로 활성) 같은 trick 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// 개발자 머신 / smoke / unit test. R2 자동 활성 0 (`R2_LOCAL_*` 명시 set 시만).
    Local,
    /// staging — production-like 환경, 별도 R2 bucket. `R2_STAGING_*` namespace.
    Staging,
    /// production — GH Actions cron + workflow_dispatch. `R2_PRODUCTION_*` namespace.
    Production,
}

impl Environment {
    /// `ETL_ENVIRONMENT` env → typed [`Environment`]. 미설정 또는 invalid 값 = `Err`.
    ///
    /// SSS-grade fail-fast — 호출자가 의도 박제 안 했으면 즉시 abort.
    ///
    /// # Errors
    ///
    /// - env 미설정 → [`EnvironmentParseError::Unset`]
    /// - 빈 문자열 → [`EnvironmentParseError::Empty`]
    /// - 알 수 없는 값 → [`EnvironmentParseError::Invalid`]
    pub fn from_env_required() -> Result<Self, EnvironmentParseError> {
        let raw = std::env::var("ETL_ENVIRONMENT").map_err(|_| EnvironmentParseError::Unset)?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(EnvironmentParseError::Empty);
        }
        match trimmed.to_ascii_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(EnvironmentParseError::Invalid(trimmed.to_owned())),
        }
    }

    /// 본 env 의 *secret namespace prefix* (e.g. `"R2_PRODUCTION_"`). 모든 R2 자격
    /// 변수가 본 prefix 를 통과해야 함. namespace 격리의 SSOT.
    #[must_use]
    pub const fn r2_secret_prefix(self) -> &'static str {
        match self {
            Self::Local => "R2_LOCAL_",
            Self::Staging => "R2_STAGING_",
            Self::Production => "R2_PRODUCTION_",
        }
    }

    /// 본 env 가 *production-grade safety* 적용 대상인지. CDN purge fail-fast 등.
    #[must_use]
    pub const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    /// `ETL_ENVIRONMENT` env 만 보고 production 여부 판단. `Config` 인스턴스 없이도
    /// 호출 가능한 callsite (e.g. `preflight_cdn_config` 같은 free function, Sentry
    /// init 시점) 가 본 helper 통과.
    ///
    /// **ADR 0035**: `ETL_BUILD_ENV` backward-compat 완전 제거 — `ETL_ENVIRONMENT` 만 SSOT.
    /// env 미설정 = 명시 의도 부재 = false (production 으로 추측 0).
    #[must_use]
    pub fn is_production_from_env() -> bool {
        std::env::var("ETL_ENVIRONMENT")
            .ok()
            .as_deref()
            .map(str::trim)
            .is_some_and(|v| v.eq_ignore_ascii_case("production") || v.eq_ignore_ascii_case("prod"))
    }

    /// 사람-가독 이름.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
