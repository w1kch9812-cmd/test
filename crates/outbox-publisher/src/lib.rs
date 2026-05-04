//! 공짱 outbox publisher — `OutboxRepository` 를 폴링해 `Sink` 로 발행해요.
//!
//! 사용처:
//! - 라이브러리: [`tick`] 호출 + [`Sink`] 구현체 제공 (테스트 / 커스텀 sink)
//! - daemon: `services/outbox-publisher` 가 [`tick`] 을 interval loop 안에서 호출
//!
//! 본 crate 는 외부 시스템 통합 의존 0 — `LoggingSink` 가 default.
//! 진짜 Kafka/Webhook/SQS sink 는 후속 sub-project 에서 같은 [`Sink`] trait
//! 구현체로 추가해요.

pub mod publisher;
pub mod sink;

pub use publisher::{tick, PublisherError, TickReport};
pub use sink::{CountingSink, LoggingSink, Sink, SinkError};
