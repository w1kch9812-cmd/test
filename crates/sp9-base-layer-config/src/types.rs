//! SSS-grade newtypes — invalid state 를 *컴파일 시점* 에 차단.
//!
//! 본 모듈은 SP9 base layer 의 *값-객체* (value object) 들을 박제. 모든 newtype 은:
//! - **생성자에서 검증** — `new(s)` 가 invalid input 을 `Err` 로 거부, panic 0.
//! - **`Display` / `AsRef<str>`** — 호출자가 `format!` / `&str` API 양쪽에 자연 흘려보냄.
//! - **`Serialize` / `Deserialize`** — JSON manifest 에 그대로 박제 (검증 round-trip).
//! - **`Debug` / `Clone` / `Eq` / `Hash`** — collection / 로깅 친화.
//!
//! 사용 정책: ETL pipeline 의 *모든* path 가 본 newtype 들을 직접 받아야 하며 `String` /
//! `&str` 으로의 fallback 은 금지. 환경변수 / CLI 인자 / config 파일 어느 origin 이든
//! `new()` 한 번만 통과해 *internal type* 이 된 후 사용한다.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
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

/// Gold 빌드 결과의 **버전 라벨** — `gold/v<N>/...` R2 prefix 의 `<N>` 부분.
///
/// 형식: `^v[a-z0-9_-]+$` (e.g. `v3`, `v_2026_05`, `v-dryrun`). 64자 이내.
///
/// # Why newtype
///
/// version 라벨이 R2 path / TileJSON URL / manifest 안에 *다중* 박제됨. 한 곳에서
/// 잘못 만들어진 라벨은 모든 path 에 전파 — 라벨이 *생성 시점에* 검증되면 그 이후
/// 어느 모듈도 `String::from("V3")` (대문자) 같은 변종을 만들 수 없다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Version(String);

impl Version {
    /// 검증된 라벨 생성. invalid input 은 `Err`.
    ///
    /// # Errors
    ///
    /// - 빈 문자열 → [`TypeError::VersionEmpty`]
    /// - 64자 초과 → [`TypeError::VersionTooLong`]
    /// - 형식 위반 → [`TypeError::VersionFormat`]
    pub fn new(raw: impl Into<String>) -> Result<Self, TypeError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(TypeError::VersionEmpty);
        }
        if raw.len() > 64 {
            return Err(TypeError::VersionTooLong(raw));
        }
        if !is_valid_version(&raw) {
            return Err(TypeError::VersionFormat(raw));
        }
        Ok(Self(raw))
    }

    /// 내부 `&str` 접근.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid_version(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != 'v' {
        return false;
    }
    let mut has_payload = false;
    for c in chars {
        has_payload = true;
        if !is_version_payload_char(c) {
            return false;
        }
    }
    has_payload
}

const fn is_version_payload_char(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '_' | '-')
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Version {
    type Err = TypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// **공간 좌표계** 식별자 — `EPSG:<digits>` 형식.
///
/// 사용처: ogr2ogr `-s_srs` / `-t_srs`, manifest lineage `source_srs` 등. SRID
/// 미지정 공간 쿼리는 AGENTS.md § 1 절대 규칙 — 본 newtype 으로 *반드시 명시* 강제.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Srs(String);

impl Srs {
    /// 검증된 SRS 생성.
    ///
    /// # Errors
    ///
    /// - 빈 문자열 → [`TypeError::SrsEmpty`]
    /// - `EPSG:<digits>` 형식 위반 → [`TypeError::SrsFormat`]
    pub fn new(raw: impl Into<String>) -> Result<Self, TypeError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(TypeError::SrsEmpty);
        }
        if !is_valid_srs(&raw) {
            return Err(TypeError::SrsFormat(raw));
        }
        Ok(Self(raw))
    }

    /// 내부 `&str` 접근.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `EPSG:` prefix 를 떼고 숫자 부분만 (e.g. `4326`).
    ///
    /// 매우 큰 EPSG 번호 (`u32::MAX` 초과) 는 입력 검증 시 통과하므로 `None` 반환.
    /// 정상 EPSG 코드 (4326, 5186 등) 는 항상 `Some`.
    #[must_use]
    pub fn epsg_code(&self) -> Option<u32> {
        // is_valid_srs 가 통과하면 prefix + digits 구조 보장 — parse 만 fallible.
        let digits = &self.0["EPSG:".len()..];
        digits.parse::<u32>().ok()
    }
}

fn is_valid_srs(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("EPSG:") else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

impl fmt::Display for Srs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Srs {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Srs {
    type Err = TypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for Srs {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// R2 의 **public CDN base URL** — `https://r2.gongzzang.dev` 형식.
///
/// `TileJSON` / manifest 의 `tiles` URL 에 prefix 로 박제. invalid scheme / host 누락
/// 시 클라이언트가 fetch 못 함 → 생성 시점에 거부.
///
/// 끝의 `/` 는 *허용* — 직렬화는 *원본 그대로* (호출자가 trailing slash 정책 결정).
/// 이는 정책 단순화 — 끝 `/` 추가/제거는 호출자가 판단 (R2Config helper 에 이미 통합).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct R2PublicBase(String);

impl R2PublicBase {
    /// 검증된 base URL 생성.
    ///
    /// # Errors
    ///
    /// - 빈 문자열 → [`TypeError::R2PublicBaseEmpty`]
    /// - scheme 위반 → [`TypeError::R2PublicBaseScheme`] (http/https 만 허용)
    /// - host 부재 → [`TypeError::R2PublicBaseHost`]
    pub fn new(raw: impl Into<String>) -> Result<Self, TypeError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(TypeError::R2PublicBaseEmpty);
        }
        // 표준 라이브러리 only — `url` crate 의존 회피 (newtype 한정).
        let lower = raw.to_ascii_lowercase();
        let after_scheme = if let Some(rest) = lower.strip_prefix("https://") {
            rest
        } else if let Some(rest) = lower.strip_prefix("http://") {
            rest
        } else {
            return Err(TypeError::R2PublicBaseScheme(raw));
        };
        // host 는 첫 `/` 또는 `?` 또는 `#` 까지. 비어있으면 거부.
        let host_end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];
        if host.is_empty() {
            return Err(TypeError::R2PublicBaseHost(raw));
        }
        Ok(Self(raw))
    }

    /// 내부 `&str` 접근.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for R2PublicBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for R2PublicBase {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for R2PublicBase {
    type Err = TypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for R2PublicBase {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

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
        let raw = std::env::var("ETL_ENVIRONMENT")
            .map_err(|_| EnvironmentParseError::Unset)?;
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
    /// **Backward-compat (1 sprint, ADR 0030 에서 제거)**: `ETL_ENVIRONMENT` 미설정 시
    /// 기존 `ETL_BUILD_ENV` env 도 검사 — 점진 migration 허용.
    #[must_use]
    pub fn is_production_from_env() -> bool {
        // primary path — ADR 0029 SSOT.
        if let Ok(v) = std::env::var("ETL_ENVIRONMENT") {
            if v.trim().eq_ignore_ascii_case("production") || v.trim().eq_ignore_ascii_case("prod") {
                return true;
            }
            // ETL_ENVIRONMENT 가 *명시 set* 됐으면 그것만 신뢰 — fallback 안 함.
            return false;
        }
        // backward-compat — `ETL_BUILD_ENV` 가 set 됐고 production 인 경우.
        // ADR 0030 에서 본 fallback 제거.
        std::env::var("ETL_BUILD_ENV")
            .ok()
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case("production"))
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;

    // Version

    #[test]
    fn version_valid_inputs() {
        for s in ["v3", "v0", "v_2026_05", "v-dryrun", "v_2026-05", "v3a"] {
            Version::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
        }
    }

    #[test]
    fn version_rejects_invalid() {
        for s in ["", "V3", "3", "v", "v3!", "vA", "v 3", " v3", "v3 "] {
            assert!(
                Version::new(s).is_err(),
                "must reject {s:?} — got Ok unexpectedly"
            );
        }
    }

    #[test]
    fn version_too_long() {
        let s = format!("v{}", "a".repeat(64));
        assert!(matches!(
            Version::new(s),
            Err(TypeError::VersionTooLong(_))
        ));
    }

    #[test]
    fn version_display_and_asref() {
        let v = Version::new("v3").unwrap();
        assert_eq!(v.to_string(), "v3");
        assert_eq!(v.as_ref(), "v3");
        assert_eq!(v.as_str(), "v3");
    }

    #[test]
    fn version_serde_round_trip() {
        let v = Version::new("v_2026_05").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"v_2026_05\"");
        let parsed: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn version_deserialize_rejects_invalid() {
        let err = serde_json::from_str::<Version>("\"V3\"").unwrap_err();
        assert!(err.to_string().contains("v[a-z0-9_-]+"));
    }

    // Srs

    #[test]
    fn srs_valid_inputs() {
        for s in ["EPSG:4326", "EPSG:5179", "EPSG:5186", "EPSG:3857"] {
            Srs::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
        }
    }

    #[test]
    fn srs_rejects_invalid() {
        for s in [
            "",
            "epsg:4326",
            "EPSG:",
            "EPSG:abc",
            "EPSG: 4326",
            "EPSG4326",
            "WGS84",
        ] {
            assert!(Srs::new(s).is_err(), "must reject {s:?}");
        }
    }

    #[test]
    fn srs_epsg_code() {
        assert_eq!(Srs::new("EPSG:4326").unwrap().epsg_code(), Some(4326));
        assert_eq!(Srs::new("EPSG:5186").unwrap().epsg_code(), Some(5186));
    }

    #[test]
    fn srs_epsg_code_overflow_returns_none() {
        // is_valid_srs 가 임의 길이 digits 통과하지만 epsg_code 는 u32 한계 → None.
        let huge = format!("EPSG:{}", "9".repeat(20));
        assert_eq!(Srs::new(huge).unwrap().epsg_code(), None);
    }

    #[test]
    fn srs_serde() {
        let s = Srs::new("EPSG:4326").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"EPSG:4326\"");
        let parsed: Srs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    // R2PublicBase

    #[test]
    fn r2_public_base_valid() {
        for s in [
            "https://r2.gongzzang.dev",
            "https://r2.gongzzang.dev/",
            "https://r2.example.com/path",
            "http://localhost:9000",
            "http://localhost:9000/bucket",
        ] {
            R2PublicBase::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
        }
    }

    #[test]
    fn r2_public_base_rejects_invalid_scheme() {
        for s in ["", "ftp://r2.example.com", "r2.example.com", "//r2.example.com"] {
            assert!(R2PublicBase::new(s).is_err(), "must reject {s:?}");
        }
    }

    #[test]
    fn r2_public_base_rejects_missing_host() {
        for s in ["https://", "https:///path", "http://?query"] {
            assert!(R2PublicBase::new(s).is_err(), "must reject {s:?}");
        }
    }

    #[test]
    fn r2_public_base_serde() {
        let b = R2PublicBase::new("https://r2.example.com").unwrap();
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"https://r2.example.com\"");
        let parsed: R2PublicBase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, b);
    }

    #[test]
    fn from_str_works() {
        assert_eq!(
            "v3".parse::<Version>().unwrap(),
            Version::new("v3").unwrap()
        );
        assert_eq!(
            "EPSG:4326".parse::<Srs>().unwrap(),
            Srs::new("EPSG:4326").unwrap()
        );
        assert_eq!(
            "https://r2.example.com".parse::<R2PublicBase>().unwrap(),
            R2PublicBase::new("https://r2.example.com").unwrap()
        );
    }
}
