use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::TypeError;

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
