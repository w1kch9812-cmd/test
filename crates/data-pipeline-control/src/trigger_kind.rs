//! `TriggerKind` — `PipelineRun` 트리거 종류 (3값).
//!
//! Spec § 5.4 `pipeline_run.triggered_by` CHECK enum:
//! `schedule`, `manual`, `event`.

#![allow(clippy::module_name_repetitions)]

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `PipelineRun` 트리거 종류 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// cron 스케줄에 의한 자동 실행.
    Schedule,
    /// 어드민이 UI 에서 수동 실행.
    Manual,
    /// 외부 이벤트 트리거.
    Event,
}

/// `TriggerKind` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TriggerKindError {
    /// 미지원 값.
    #[error("unknown trigger_kind: '{0}' (expected: schedule, manual, event)")]
    Unknown(String),
}

impl TriggerKind {
    /// 정규화된 `snake_case` 문자열 반환 (DB `varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Manual => "manual",
            Self::Event => "event",
        }
    }
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TriggerKind {
    type Err = TriggerKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "schedule" => Ok(Self::Schedule),
            "manual" => Ok(Self::Manual),
            "event" => Ok(Self::Event),
            other => Err(TriggerKindError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(TriggerKind::Schedule.as_str(), "schedule");
        assert_eq!(TriggerKind::Manual.as_str(), "manual");
        assert_eq!(TriggerKind::Event.as_str(), "event");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(TriggerKind::from_str("schedule"), Ok(TriggerKind::Schedule));
        assert_eq!(TriggerKind::from_str("manual"), Ok(TriggerKind::Manual));
        assert_eq!(TriggerKind::from_str("event"), Ok(TriggerKind::Event));
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = TriggerKind::from_str("cron").unwrap_err();
        assert!(matches!(err, TriggerKindError::Unknown(s) if s == "cron"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = TriggerKind::from_str("").unwrap_err();
        assert!(matches!(err, TriggerKindError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", TriggerKind::Manual), "manual");
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            TriggerKind::Schedule,
            TriggerKind::Manual,
            TriggerKind::Event,
        ] {
            assert_eq!(TriggerKind::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = TriggerKind::Manual;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""manual""#);
        let back: TriggerKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = TriggerKind::Schedule;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(TriggerKind::Schedule);
        set.insert(TriggerKind::Manual);
        set.insert(TriggerKind::Event);
        assert_eq!(set.len(), 3);
    }
}
