//! `RawCapture` wrapping composers — `SanitizingRawCapture` + `DualTierCapture` (T3).
//!
//! Spec §3.3, §3.5. 시그니처는 [`RawCapture::capture`] 와 정확히 일치 —
//! `(pnu, source, raw, fetched_at) -> Result<RawCaptureReceipt, RawCaptureError>`.

use crate::{RawCapture, RawCaptureError, RawCaptureReceipt, RawSanitizer};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;

/// `RawCapture` 를 wrap 하여 INSERT 전에 `RawSanitizer` 로 정제. drift 발생 시
/// `tracing::warn!(target = "raw.capture.schema_drift", ...)` 발행.
///
/// inner sink 의 `RawCaptureReceipt` 가 그대로 전파됨 — wrapper 는 sanitization
/// 만 책임.
pub struct SanitizingRawCapture<C: RawCapture> {
    inner: C,
    sanitizer: Arc<dyn RawSanitizer>,
}

impl<C: RawCapture> SanitizingRawCapture<C> {
    /// 새 `SanitizingRawCapture`.
    pub fn new(inner: C, sanitizer: Arc<dyn RawSanitizer>) -> Self {
        Self { inner, sanitizer }
    }
}

#[async_trait]
impl<C: RawCapture + Send + Sync> RawCapture for SanitizingRawCapture<C> {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        let sanitized = self.sanitizer.sanitize(raw);
        if sanitized.dropped_count > 0 {
            tracing::warn!(
                target: "raw.capture.schema_drift",
                pnu = %pnu,
                source = %source,
                schema_hash = %sanitized.schema_hash,
                dropped_count = sanitized.dropped_count,
                "raw_response sanitizer dropped unknown fields"
            );
        }
        let sanitized_value = sanitized.value;
        self.inner
            .capture(pnu, source, &sanitized_value, fetched_at)
            .await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use crate::sanitizer::AllowlistSanitizer;
    use crate::NoOpRawCapture;

    #[tokio::test]
    async fn sanitizing_wrap_forwards_to_inner() {
        let sanitizer = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer);
        let raw = serde_json::json!({"keep": "ok", "drop_me": "secret"});
        let receipt = wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await
            .expect("capture must succeed");
        assert!(!receipt.object_key.is_empty());
    }

    /// drift warn 발행 trigger 조건 검증 — sanitizer 의 dropped_count > 0.
    /// 실 `tracing::warn!` emission 자체는 production observability + manual
    /// log 검증 (tracing-test 의 logs_contain 가 format 의존이라 fragile).
    #[tokio::test]
    async fn drift_signal_triggers_when_unknown_fields() {
        let sanitizer_inner: Arc<dyn RawSanitizer> = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let raw = serde_json::json!({"keep": "ok", "drop_this": "secret"});
        let s = sanitizer_inner.sanitize(&raw);
        assert!(s.dropped_count > 0, "wrapper 가 warn 발행할 trigger 조건");
        assert_eq!(s.dropped_count, 1);

        // wrapper 호출도 정상 — capture path 가 panic 없음
        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer_inner);
        let result = wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn no_drift_when_all_fields_allowed() {
        let sanitizer_inner: Arc<dyn RawSanitizer> = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let raw = serde_json::json!({"keep": "ok"});
        let s = sanitizer_inner.sanitize(&raw);
        assert_eq!(s.dropped_count, 0, "no drift signal");

        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer_inner);
        let result = wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await;
        assert!(result.is_ok());
    }
}
