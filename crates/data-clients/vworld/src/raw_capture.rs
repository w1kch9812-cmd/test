//! `RawCapture` trait — 외부 API raw_response 보존 hook.
//!
//! 우리 정책: 모든 외부 응답을 raw 그대로 보존 (감사·재현·분쟁 시 증빙).
//! v1 (SP4-ii) 은 `NoOpRawCapture` (`tracing::info!` 로 메타데이터만 발행) — DB
//! `parcel_external_data` 테이블 저장 구현은 SP4-iii (FU 27).

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::info;

use crate::error::RawCaptureError;

/// 외부 API raw_response 보존 인터페이스.
///
/// 모든 구현체는 *멱등성* 권장 — 재발행/재시도 시 동일 (pnu, source) 가
/// 두번 도착할 수 있음.
#[async_trait]
pub trait RawCapture: Send + Sync {
    /// 단일 raw_response 보존.
    ///
    /// # Errors
    ///
    /// 저장소 통신 실패 시 [`RawCaptureError::Sink`].
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError>;
}

/// 기본 구현 — `tracing::info!` 로 메타데이터만 발행 (raw payload 미노출).
///
/// SP4-iii 에서 DB 저장 구현체로 교체 예정.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpRawCapture;

impl NoOpRawCapture {
    /// 새 [`NoOpRawCapture`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RawCapture for NoOpRawCapture {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        let bytes = raw.to_string().len();
        info!(
            target: "vworld.raw",
            pnu = %pnu,
            source = %source,
            bytes,
            fetched_at = %fetched_at,
            "raw_response captured (no-op sink — SP4-iii 에서 DB 저장 예정)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn noop_raw_capture_returns_ok() {
        let capture = NoOpRawCapture::new();
        let result = capture
            .capture(
                "1111010100100010000",
                "vworld",
                &serde_json::json!({"k": "v"}),
                Utc::now(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn raw_capture_error_display() {
        let e = RawCaptureError::Sink("db down".to_owned());
        assert_eq!(e.to_string(), "raw capture sink failure: db down");
    }
}
