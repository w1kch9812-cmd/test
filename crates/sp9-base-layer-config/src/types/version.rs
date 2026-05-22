use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::TypeError;

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
