//! `HealthCheckRecord` — DB row 도메인 표현 + `NewHealthCheck` `INSERT` 빌더.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::status::HealthStatus;

/// `api_health_check` 테이블 row 의 도메인 표현.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    /// `BIGSERIAL` PK.
    pub id: i64,
    /// 대상 API endpoint 식별자 (예: `data_go_kr.getBrTitleInfo`).
    pub api_name: String,
    /// DB `DEFAULT NOW()` 로 채워진 검사 시각.
    pub checked_at: DateTime<Utc>,
    /// 검사 결과 분류.
    pub status: HealthStatus,
    /// HTTP 응답 코드. `timeout` / `connection_fail` 일 때 `None`.
    pub http_code: Option<u16>,
    /// 마스킹된 에러 디테일 (시크릿 redacted).
    pub error_detail: Option<String>,
    /// `true` = schedule cron, `false` = `workflow_dispatch` 수동 trigger.
    pub cron_run: bool,
    /// 검사 소요 시간 (ms). `>= 0`.
    pub duration_ms: u32,
}

/// `record()` 호출 시 받는 `INSERT` 인자.
///
/// `id` / `checked_at` 은 DB 가 채움.
#[derive(Debug, Clone)]
pub struct NewHealthCheck<'a> {
    /// 대상 API endpoint 식별자.
    pub api_name: &'a str,
    /// 검사 결과 분류.
    pub status: HealthStatus,
    /// HTTP 응답 코드. `timeout` / `connection_fail` 일 때 `None`.
    pub http_code: Option<u16>,
    /// 마스킹된 에러 디테일.
    pub error_detail: Option<&'a str>,
    /// `true` = schedule cron, `false` = `workflow_dispatch`.
    pub cron_run: bool,
    /// 검사 소요 시간 (ms).
    pub duration_ms: u32,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn new_health_check_construction() {
        let new = NewHealthCheck {
            api_name: "data_go_kr.getBrTitleInfo",
            status: HealthStatus::Success,
            http_code: Some(200),
            error_detail: None,
            cron_run: true,
            duration_ms: 1234,
        };
        assert_eq!(new.api_name, "data_go_kr.getBrTitleInfo");
        assert_eq!(new.status, HealthStatus::Success);
        assert_eq!(new.duration_ms, 1234);
    }

    #[test]
    fn record_serde_roundtrip() {
        let record = HealthCheckRecord {
            id: 42,
            api_name: "vworld.getFeature".to_owned(),
            checked_at: Utc::now(),
            status: HealthStatus::Http5xx,
            http_code: Some(502),
            error_detail: Some("upstream timeout".to_owned()),
            cron_run: true,
            duration_ms: 5000,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: HealthCheckRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, record);
    }
}
