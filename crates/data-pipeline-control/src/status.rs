//! `RunStatus` — `PipelineRun` 상태 (5값) + 상태 머신 헬퍼.
//!
//! Spec § 5.4 `pipeline_run.status` CHECK enum:
//! `running`, `success`, `failed`, `skipped_unchanged`, `aborted`.

#![allow(clippy::module_name_repetitions)]

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `PipelineRun` 상태 (5값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// 실행 중 (initial).
    Running,
    /// 성공 (terminal).
    Success,
    /// 실패 (terminal).
    Failed,
    /// 변경 없음으로 스킵 (terminal) — output hash 비교 결과 변경 없을 때.
    SkippedUnchanged,
    /// 중단 (terminal) — 외부에서 강제 중단.
    Aborted,
}

/// `RunStatus` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RunStatusError {
    /// 미지원 값.
    #[error(
        "unknown run_status: '{0}' (expected: running, success, failed, skipped_unchanged, aborted)"
    )]
    Unknown(String),
}

impl RunStatus {
    /// 정규화된 `snake_case` 문자열 반환 (DB `varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::SkippedUnchanged => "skipped_unchanged",
            Self::Aborted => "aborted",
        }
    }

    /// 터미널 상태 여부 — `Running` 외 4 상태 모두 터미널.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunStatus {
    type Err = RunStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "skipped_unchanged" => Ok(Self::SkippedUnchanged),
            "aborted" => Ok(Self::Aborted),
            other => Err(RunStatusError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(RunStatus::Running.as_str(), "running");
        assert_eq!(RunStatus::Success.as_str(), "success");
        assert_eq!(RunStatus::Failed.as_str(), "failed");
        assert_eq!(RunStatus::SkippedUnchanged.as_str(), "skipped_unchanged");
        assert_eq!(RunStatus::Aborted.as_str(), "aborted");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(RunStatus::from_str("running"), Ok(RunStatus::Running));
        assert_eq!(RunStatus::from_str("success"), Ok(RunStatus::Success));
        assert_eq!(RunStatus::from_str("failed"), Ok(RunStatus::Failed));
        assert_eq!(
            RunStatus::from_str("skipped_unchanged"),
            Ok(RunStatus::SkippedUnchanged)
        );
        assert_eq!(RunStatus::from_str("aborted"), Ok(RunStatus::Aborted));
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = RunStatus::from_str("pending").unwrap_err();
        assert!(matches!(err, RunStatusError::Unknown(s) if s == "pending"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = RunStatus::from_str("").unwrap_err();
        assert!(matches!(err, RunStatusError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(
            format!("{}", RunStatus::SkippedUnchanged),
            "skipped_unchanged"
        );
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            RunStatus::Running,
            RunStatus::Success,
            RunStatus::Failed,
            RunStatus::SkippedUnchanged,
            RunStatus::Aborted,
        ] {
            assert_eq!(RunStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn is_terminal_only_running_is_not_terminal() {
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Success.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::SkippedUnchanged.is_terminal());
        assert!(RunStatus::Aborted.is_terminal());
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = RunStatus::SkippedUnchanged;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""skipped_unchanged""#);
        let back: RunStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = RunStatus::Running;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(RunStatus::Running);
        set.insert(RunStatus::Success);
        assert_eq!(set.len(), 2);
    }
}
