//! `BuildingStructureCode` — 한국 건축물대장 구조 (8종).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 건물 구조 (8값).
///
/// 산업용 건물에서 자주 등장하는 구조 8종을 추렸어요.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingStructureCode {
    /// 철근콘크리트.
    ReinforcedConcrete,
    /// 철골.
    Steel,
    /// 철골철근콘크리트 (`SRC`).
    SteelReinforcedConcrete,
    /// 벽돌.
    Brick,
    /// 블록.
    Block,
    /// 목조.
    Wood,
    /// 경량철골.
    LightSteel,
    /// 기타.
    Other,
}

/// `BuildingStructureCode` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildingStructureCodeError {
    /// 정의되지 않은 코드 문자열.
    #[error("unknown building_structure_code: '{0}'")]
    Unknown(String),
}

impl BuildingStructureCode {
    /// 정규화된 `snake_case` 문자열 (`R2` 데이터 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReinforcedConcrete => "reinforced_concrete",
            Self::Steel => "steel",
            Self::SteelReinforcedConcrete => "steel_reinforced_concrete",
            Self::Brick => "brick",
            Self::Block => "block",
            Self::Wood => "wood",
            Self::LightSteel => "light_steel",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for BuildingStructureCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BuildingStructureCode {
    type Err = BuildingStructureCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "reinforced_concrete" => Ok(Self::ReinforcedConcrete),
            "steel" => Ok(Self::Steel),
            "steel_reinforced_concrete" => Ok(Self::SteelReinforcedConcrete),
            "brick" => Ok(Self::Brick),
            "block" => Ok(Self::Block),
            "wood" => Ok(Self::Wood),
            "light_steel" => Ok(Self::LightSteel),
            "other" => Ok(Self::Other),
            other => Err(BuildingStructureCodeError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{BuildingStructureCode, BuildingStructureCodeError};
    use std::str::FromStr;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(
            BuildingStructureCode::ReinforcedConcrete.as_str(),
            "reinforced_concrete"
        );
        assert_eq!(BuildingStructureCode::Steel.as_str(), "steel");
        assert_eq!(
            BuildingStructureCode::SteelReinforcedConcrete.as_str(),
            "steel_reinforced_concrete"
        );
        assert_eq!(BuildingStructureCode::Brick.as_str(), "brick");
        assert_eq!(BuildingStructureCode::Block.as_str(), "block");
        assert_eq!(BuildingStructureCode::Wood.as_str(), "wood");
        assert_eq!(BuildingStructureCode::LightSteel.as_str(), "light_steel");
        assert_eq!(BuildingStructureCode::Other.as_str(), "other");
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            BuildingStructureCode::ReinforcedConcrete,
            BuildingStructureCode::Steel,
            BuildingStructureCode::SteelReinforcedConcrete,
            BuildingStructureCode::Brick,
            BuildingStructureCode::Block,
            BuildingStructureCode::Wood,
            BuildingStructureCode::LightSteel,
            BuildingStructureCode::Other,
        ] {
            assert_eq!(BuildingStructureCode::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = BuildingStructureCode::from_str("concrete").unwrap_err();
        assert!(matches!(err, BuildingStructureCodeError::Unknown(s) if s == "concrete"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = BuildingStructureCode::from_str("").unwrap_err();
        assert!(matches!(err, BuildingStructureCodeError::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn from_str_rejects_uppercase() {
        let err = BuildingStructureCode::from_str("STEEL").unwrap_err();
        assert!(matches!(err, BuildingStructureCodeError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(
            format!("{}", BuildingStructureCode::ReinforcedConcrete),
            "reinforced_concrete"
        );
        assert_eq!(
            format!("{}", BuildingStructureCode::SteelReinforcedConcrete),
            "steel_reinforced_concrete"
        );
    }

    #[test]
    fn serde_roundtrip() {
        let v = BuildingStructureCode::Steel;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""steel""#);
        let back: BuildingStructureCode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_snake_case_for_compound_variant() {
        let v = BuildingStructureCode::SteelReinforcedConcrete;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""steel_reinforced_concrete""#);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = BuildingStructureCode::Steel;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(BuildingStructureCode::Steel);
        set.insert(BuildingStructureCode::Brick);
        set.insert(BuildingStructureCode::Steel);
        assert_eq!(set.len(), 2);
    }
}
