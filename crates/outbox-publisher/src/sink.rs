//! `Sink` trait — outbox event 의 발행 대상 추상화.
//!
//! v1 은 [`LoggingSink`] (tracing event 발행) 만 default. 외부 시스템 통합
//! (Kafka / Webhook / SQS / NATS) 은 후속 sub-project 에서 같은 trait 구현체로
//! 추가해요.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use outbox_event_domain::entity::OutboxEvent;
use thiserror::Error;
use tracing::info;

/// Sink 발행 실패.
///
/// 개별 이벤트 발행 실패는 publisher 가 흡수해 다음 tick 에 재시도해요
/// (sink 멱등성 의무 — at-least-once 전제).
#[derive(Debug, Error)]
pub enum SinkError {
    /// 발행 실패 — 메시지에 원인 보존.
    #[error("sink publish failed: {0}")]
    Publish(String),
}

/// 외부 시스템에 outbox event 를 발행하는 추상화.
///
/// 모든 구현체는 *멱등성* 을 보장해야 해요 — publisher 는 at-least-once 발행이라
/// 같은 event 가 두 번 도착할 수 있어요.
#[async_trait]
pub trait Sink: Send + Sync {
    /// 이벤트 1 개 발행.
    ///
    /// # Errors
    /// 발행 실패 시 [`SinkError::Publish`].
    async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError>;
}

/// 기본 sink — `tracing::info!` 로 구조화 event 발행.
///
/// target = `"outbox.publish"`. 운영 시 Loki / Grafana 가 해당 target 필터로
/// 발행 흐름 모니터링.
#[derive(Debug, Default, Clone, Copy)]
pub struct LoggingSink;

impl LoggingSink {
    /// 새 [`LoggingSink`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Sink for LoggingSink {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), SinkError> {
        info!(
            target: "outbox.publish",
            event_id = %event.id.as_str(),
            event_type = %event.event_type,
            aggregate_kind = %event.aggregate_kind,
            aggregate_id = %event.aggregate_id,
            correlation_id = %event.correlation_id,
            occurred_at = %event.occurred_at,
            "outbox event published"
        );
        Ok(())
    }
}

/// 테스트용 sink — `publish` 호출 횟수만 카운트.
///
/// 실패 동작 검증은 inline `FailingSink` 등 별도 sink 로 — 본 sink 는 항상 성공.
#[derive(Debug, Default)]
pub struct CountingSink {
    count: AtomicU64,
}

impl CountingSink {
    /// 새 [`CountingSink`] (count = 0).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }

    /// 누적 publish 호출 횟수.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Sink for CountingSink {
    async fn publish(&self, _event: &OutboxEvent) -> Result<(), SinkError> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use chrono::Utc;
    use shared_kernel::id::{Id, OutboxEventMarker};

    use super::*;

    fn sample_event() -> OutboxEvent {
        OutboxEvent {
            id: Id::<OutboxEventMarker>::new(),
            event_type: "test.event".to_owned(),
            aggregate_kind: "test".to_owned(),
            aggregate_id: "agg-1".to_owned(),
            payload: serde_json::json!({"k": "v"}),
            occurred_at: Utc::now(),
            published_at: None,
            correlation_id: "corr-1".to_owned(),
        }
    }

    #[tokio::test]
    async fn logging_sink_publishes_without_panic() {
        let sink = LoggingSink::new();
        let event = sample_event();
        sink.publish(&event).await.expect("logging sink ok");
    }

    #[tokio::test]
    async fn counting_sink_increments_per_publish() {
        let sink = CountingSink::new();
        let event = sample_event();
        for _ in 0..3 {
            sink.publish(&event).await.expect("counting sink ok");
        }
        assert_eq!(sink.count(), 3);
    }

    #[test]
    fn sink_error_display_format() {
        let e = SinkError::Publish("boom".to_owned());
        assert_eq!(e.to_string(), "sink publish failed: boom");
    }
}
