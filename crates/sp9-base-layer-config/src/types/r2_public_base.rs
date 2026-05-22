use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::TypeError;

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
