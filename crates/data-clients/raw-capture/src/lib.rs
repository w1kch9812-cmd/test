//! 공짱 외부 API raw_response 보존 표준 — `RawCapture` trait + `NoOpRawCapture`.
//!
//! 모든 외부 API 클라이언트 (`vworld-client`, `data-go-kr-client`, `korean-law-client`
//! 등) 가 이 trait 을 통해 raw 응답을 보존해요. 활성 sink:
//! `services/api/src/r2_raw_capture.rs::R2RawCapture` (ADR 0026 — R2 Bronze 영구 archive).
//!
//! # 정책
//! - 모든 외부 응답은 *raw 그대로* 보존 (감사·재현·분쟁 시 증빙)
//! - sink 가 (R2, 디스크 fallback, NoOp 등) 어디에 적재했는지 [`RawCaptureReceipt`] 로
//!   *호출자에게 반환* — 호출자는 receipt 으로 metadata DB 추적 (`parcel_external_data`
//!   의 `r2_object_key` / `raw_byte_size`) 가능
//! - sink 자체는 metadata DB 를 모름 (단일 책임). decorator 패턴으로 metadata 추적 합성
//!
//! # 사용
//! ```ignore
//! use raw_capture_client::{RawCapture, NoOpRawCapture};
//! let capture: Arc<dyn RawCapture> = Arc::new(NoOpRawCapture::new());
//! let receipt = capture.capture("1111010100100010000", "vworld", &raw_json, Utc::now()).await?;
//! tracing::info!(?receipt, "raw 보존 완료");
//! ```

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

pub mod sanitizer;
pub use sanitizer::{RawSanitizer, SanitizedRaw};

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

/// 적재 종류 — 호출자가 metadata 추적 시 R2 vs fallback vs noop 분기.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawCaptureKind {
    /// R2 PUT 성공.
    R2,
    /// R2 PUT 실패 → 로컬 디스크 fallback 적재 (운영 sync 대기).
    Fallback,
    /// NoOp sink — 적재 안 됨 (dev/테스트).
    NoOp,
}

impl RawCaptureKind {
    /// 짧은 라벨 (로그 / DB 컬럼 prefix 용).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::R2 => "r2",
            Self::Fallback => "fallback",
            Self::NoOp => "noop",
        }
    }
}

/// `RawCapture::capture` 결과 — sink 가 어디에 raw 를 적재했는지 호출자에게 보고.
///
/// 호출자 (보통 `services/api`) 가 본 receipt 로 `parcel_external_data` 메타 row
/// (`r2_object_key`, `raw_byte_size`, `fetched_at`) 를 UPSERT — Bronze 추적 SSOT.
#[derive(Debug, Clone)]
pub struct RawCaptureReceipt {
    /// 적재 위치 — R2 키 또는 fallback 디스크 경로 또는 NoOp 마커.
    /// R2 = `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json`
    /// fallback = `fallback::{디스크 경로}` prefix
    /// noop = `noop::{source}/{pnu}` (placeholder)
    pub object_key: String,
    /// 적재된 raw payload 의 byte 크기 (R2 PUT body / 디스크 file size 동일).
    pub byte_size: i64,
    /// 적재 종류.
    pub kind: RawCaptureKind,
    /// 적재 시각 — 호출자의 `fetched_at` 그대로 echo (audit lineage).
    pub stored_at: DateTime<Utc>,
}

/// 외부 API raw_response 보존 인터페이스.
///
/// 모든 구현체는 *멱등성* 권장 — 재발행 / 재시도 시 동일 (pnu, source) 가
/// 두 번 도착해도 OK 이어야 해요 (timestamp 기반 키 = 자연스럽게 다른 객체).
#[async_trait]
pub trait RawCapture: Send + Sync {
    /// 단일 raw_response 보존. 적재 위치/크기를 [`RawCaptureReceipt`] 로 반환.
    ///
    /// # Errors
    /// 저장소 통신 실패 시 [`RawCaptureError::Sink`].
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError>;
}

/// 기본 구현 — `tracing::info!` (`target = "raw.capture"`) 로 메타데이터만 발행.
///
/// raw payload 는 미노출 (PII 위험). production 적재는 `R2RawCapture` 가 담당.
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
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        let bytes = raw.to_string().len();
        info!(
            target: "raw.capture",
            pnu = %pnu,
            source = %source,
            bytes,
            fetched_at = %fetched_at,
            "raw_response captured (no-op sink — production 은 R2RawCapture)"
        );
        Ok(RawCaptureReceipt {
            object_key: format!("noop::{source}/{pnu}"),
            byte_size: i64::try_from(bytes).unwrap_or(i64::MAX),
            kind: RawCaptureKind::NoOp,
            stored_at: fetched_at,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn noop_returns_receipt() {
        let c = NoOpRawCapture::new();
        let receipt = c
            .capture(
                "1111010100100010000",
                "vworld",
                &serde_json::json!({"k": "v"}),
                Utc::now(),
            )
            .await
            .expect("noop capture ok");
        assert_eq!(receipt.kind, RawCaptureKind::NoOp);
        assert!(receipt.object_key.starts_with("noop::vworld/"));
        assert!(receipt.byte_size > 0);
    }

    #[test]
    fn raw_capture_kind_as_str() {
        assert_eq!(RawCaptureKind::R2.as_str(), "r2");
        assert_eq!(RawCaptureKind::Fallback.as_str(), "fallback");
        assert_eq!(RawCaptureKind::NoOp.as_str(), "noop");
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
