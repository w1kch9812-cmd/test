//! 공짱 외부 API raw_response 보존 표준 — `RawCapture` trait + `NoOpRawCapture`.
//!
//! 모든 외부 API 클라이언트 (`vworld-client`, `data-go-kr-client`, `korean-law-client`
//! 등) 가 이 trait 을 통해 raw 응답을 보존해요. `NoOpRawCapture` 는 메타데이터만
//! tracing event 로 발행 — 진짜 DB 저장은 `crates/db/src/raw_capture.rs` 의
//! `PgRawCapture` 가 담당.
//!
//! # 정책
//! - 모든 외부 응답은 *raw 그대로* 보존 (감사·재현·분쟁 시 증빙)
//! - `(pnu, source)` 합성 PK — 같은 필지 같은 source 는 단일 row (UPSERT)
//! - source 는 enum-like 문자열 (마이그 V003_05 의 CHECK 제약 참조)
//!
//! # 사용
//! ```ignore
//! use raw_capture_client::{RawCapture, NoOpRawCapture};
//! let capture: Arc<dyn RawCapture> = Arc::new(NoOpRawCapture::new());
//! capture.capture("1111010100100010000", "vworld", &raw_json, Utc::now()).await?;
//! ```

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::info;

/// `RawCapture::capture` 실패. 정상 흐름엔 영향 없음 (caller 가 warn 후 진행).
#[derive(Debug, Error)]
pub enum RawCaptureError {
    /// 저장소 통신 실패.
    #[error("raw capture sink failure: {0}")]
    Sink(String),
}

/// 외부 API raw_response 보존 인터페이스.
///
/// 모든 구현체는 *멱등성* 권장 — 재발행 / 재시도 시 동일 (pnu, source) 가
/// 두 번 도착해도 OK 이어야 해요 (UPSERT).
#[async_trait]
pub trait RawCapture: Send + Sync {
    /// 단일 raw_response 보존.
    ///
    /// # Errors
    /// 저장소 통신 실패 시 [`RawCaptureError::Sink`].
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError>;
}

/// 기본 구현 — `tracing::info!` (`target = "raw.capture"`) 로 메타데이터만 발행.
///
/// raw payload 는 미노출 (PII 위험). 진짜 DB 저장은 `PgRawCapture` (별도 crate).
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
            target: "raw.capture",
            pnu = %pnu,
            source = %source,
            bytes,
            fetched_at = %fetched_at,
            "raw_response captured (no-op sink — DB 저장은 PgRawCapture)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn noop_returns_ok() {
        let c = NoOpRawCapture::new();
        let result = c
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

    #[test]
    fn noop_default() {
        let _: NoOpRawCapture = NoOpRawCapture;
    }
}
