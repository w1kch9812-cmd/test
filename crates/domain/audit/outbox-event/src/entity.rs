//! `OutboxEvent` Aggregate (transactional outbox 패턴).
//!
//! `Aggregate` 도메인 메서드가 emit 한 `DomainEvent` 를 application layer 에서
//! `OutboxEvent` 로 wrap 해 같은 트랜잭션 안에 INSERT 해요. Publisher 워커가
//! 미발행 row 를 fetch 해 외부 시스템에 발행 후 `mark_published` 호출.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::id::{Id, OutboxEventMarker};

use crate::errors::OutboxEventError;

/// `aggregate_kind` 최대 길이 (spec § 5.3 `varchar(30)`).
const MAX_AGGREGATE_KIND_LEN: usize = 30;
/// `correlation_id` 최대 길이 (spec § 5.3 `varchar(30)`).
const MAX_CORRELATION_ID_LEN: usize = 30;

/// `OutboxEvent` 1건. Transactional outbox row.
///
/// 8 필드 — spec § 5.3 `outbox_event` 매핑.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEvent {
    /// 식별자 (`evt_<26 ULID>`).
    pub id: Id<OutboxEventMarker>,
    /// 이벤트 종류 (`<aggregate>.<verb>`, 예: `"listing.approved"`). `DomainEvent::event_type` 로부터.
    pub event_type: String,
    /// 소속 `Aggregate` 종류 (≤30자, 비어있지 않음, 예: `"listing"`).
    pub aggregate_kind: String,
    /// 소속 `Aggregate` 식별자 (≤50자, `DomainEvent::aggregate_id` 로부터 trust).
    pub aggregate_id: String,
    /// 이벤트 페이로드 (`JSONB`).
    pub payload: serde_json::Value,
    /// 이벤트 발생 시각 — `DomainEvent::occurred_at` 로부터.
    pub occurred_at: DateTime<Utc>,
    /// Publisher 가 외부 발행 완료한 시각. `None` = 미발행.
    pub published_at: Option<DateTime<Utc>>,
    /// 분산 추적 `correlation_id` (≤30자, 비어있지 않음).
    pub correlation_id: String,
}

impl OutboxEvent {
    /// `DomainEvent` 로부터 새 [`OutboxEvent`] 생성. `published_at = None`.
    ///
    /// `event_type` / `aggregate_id` 는 `DomainEvent` trait 에서 오므로 trust — 길이/형식
    /// 검증을 호출 측에서 보장해요.
    ///
    /// # Errors
    ///
    /// - `aggregate_kind` 빈 (trim 후) → [`OutboxEventError::EmptyAggregateKind`].
    /// - `aggregate_kind` 30자 초과 → [`OutboxEventError::AggregateKindTooLong`].
    /// - `correlation_id` 빈 (trim 후) → [`OutboxEventError::EmptyCorrelationId`].
    /// - `correlation_id` 30자 초과 → [`OutboxEventError::CorrelationIdTooLong`].
    pub fn from_domain<E: DomainEvent + ?Sized>(
        id: Id<OutboxEventMarker>,
        event: &E,
        aggregate_kind: &str,
        correlation_id: &str,
    ) -> Result<Self, OutboxEventError> {
        let aggregate_kind = aggregate_kind.trim().to_owned();
        if aggregate_kind.is_empty() {
            return Err(OutboxEventError::EmptyAggregateKind);
        }
        if aggregate_kind.chars().count() > MAX_AGGREGATE_KIND_LEN {
            return Err(OutboxEventError::AggregateKindTooLong {
                actual: aggregate_kind.chars().count(),
            });
        }

        let correlation_id = correlation_id.trim().to_owned();
        if correlation_id.is_empty() {
            return Err(OutboxEventError::EmptyCorrelationId);
        }
        if correlation_id.chars().count() > MAX_CORRELATION_ID_LEN {
            return Err(OutboxEventError::CorrelationIdTooLong {
                actual: correlation_id.chars().count(),
            });
        }

        Ok(Self {
            id,
            event_type: event.event_type().to_owned(),
            aggregate_kind,
            aggregate_id: event.aggregate_id(),
            payload: event.payload(),
            occurred_at: event.occurred_at(),
            published_at: None,
            correlation_id,
        })
    }

    /// 발행 완료 마킹. **Idempotent** — 이미 발행된 경우 무시.
    ///
    /// Publisher 워커가 외부 시스템 발행 성공 후 호출해요.
    pub const fn mark_published(&mut self, at: DateTime<Utc>) {
        if self.published_at.is_none() {
            self.published_at = Some(at);
        }
    }

    /// 발행 여부.
    #[must_use]
    pub const fn is_published(&self) -> bool {
        self.published_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use chrono::TimeZone;
    use serde_json::{json, Value};

    use super::*;

    /// Sample `DomainEvent` — `Listing` approval (테스트 전용).
    #[derive(Debug)]
    struct TestEvent {
        listing_id: String,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    }

    impl TestEvent {
        fn sample() -> Self {
            Self {
                listing_id: "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
                approved_by: "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
                occurred_at: Utc
                    .with_ymd_and_hms(2026, 5, 2, 12, 0, 0)
                    .single()
                    .expect("valid"),
            }
        }
    }

    impl DomainEvent for TestEvent {
        fn event_type(&self) -> &'static str {
            "listing.approved"
        }
        fn occurred_at(&self) -> DateTime<Utc> {
            self.occurred_at
        }
        fn aggregate_id(&self) -> String {
            self.listing_id.clone()
        }
        fn payload(&self) -> Value {
            json!({ "approved_by": self.approved_by })
        }
    }

    #[test]
    fn from_domain_happy_path() {
        let event = TestEvent::sample();
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        assert_eq!(outbox.event_type, "listing.approved");
        assert_eq!(outbox.aggregate_kind, "listing");
        assert_eq!(outbox.aggregate_id, "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G");
        assert_eq!(outbox.correlation_id, "corr_abc");
        assert!(outbox.published_at.is_none());
        assert!(!outbox.is_published());
    }

    #[test]
    fn from_domain_id_starts_with_evt() {
        let event = TestEvent::sample();
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        assert!(outbox.id.as_str().starts_with("evt_"));
        assert_eq!(outbox.id.as_str().len(), 30);
    }

    #[test]
    fn from_domain_payload_preserved() {
        let event = TestEvent::sample();
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        assert_eq!(
            outbox.payload["approved_by"],
            "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G"
        );
    }

    #[test]
    fn from_domain_occurred_at_copied_from_event() {
        let event = TestEvent::sample();
        let expected = event.occurred_at;
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        assert_eq!(outbox.occurred_at, expected);
    }

    #[test]
    fn from_domain_rejects_empty_aggregate_kind() {
        let event = TestEvent::sample();
        let err = OutboxEvent::from_domain(Id::new(), &event, "", "corr_abc").unwrap_err();
        assert!(matches!(err, OutboxEventError::EmptyAggregateKind));
    }

    #[test]
    fn from_domain_rejects_whitespace_only_aggregate_kind() {
        let event = TestEvent::sample();
        let err = OutboxEvent::from_domain(Id::new(), &event, "   ", "corr_abc").unwrap_err();
        assert!(matches!(err, OutboxEventError::EmptyAggregateKind));
    }

    #[test]
    fn from_domain_rejects_aggregate_kind_over_30_chars() {
        let event = TestEvent::sample();
        let long = "X".repeat(31);
        let err = OutboxEvent::from_domain(Id::new(), &event, &long, "corr_abc").unwrap_err();
        assert!(matches!(
            err,
            OutboxEventError::AggregateKindTooLong { actual: 31 }
        ));
    }

    #[test]
    fn from_domain_rejects_empty_correlation_id() {
        let event = TestEvent::sample();
        let err = OutboxEvent::from_domain(Id::new(), &event, "listing", "").unwrap_err();
        assert!(matches!(err, OutboxEventError::EmptyCorrelationId));
    }

    #[test]
    fn from_domain_rejects_correlation_id_over_30_chars() {
        let event = TestEvent::sample();
        let long = "X".repeat(31);
        let err = OutboxEvent::from_domain(Id::new(), &event, "listing", &long).unwrap_err();
        assert!(matches!(
            err,
            OutboxEventError::CorrelationIdTooLong { actual: 31 }
        ));
    }

    #[test]
    fn from_domain_boundary_aggregate_kind_exactly_30_chars_accepted() {
        let event = TestEvent::sample();
        let exactly = "X".repeat(30);
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, &exactly, "corr_abc").expect("30 ok");
        assert_eq!(outbox.aggregate_kind.chars().count(), 30);
    }

    #[test]
    fn from_domain_boundary_correlation_id_exactly_30_chars_accepted() {
        let event = TestEvent::sample();
        let exactly = "X".repeat(30);
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", &exactly).expect("30 ok");
        assert_eq!(outbox.correlation_id.chars().count(), 30);
    }

    #[test]
    fn from_domain_trims_aggregate_kind_and_correlation_id() {
        let event = TestEvent::sample();
        let outbox = OutboxEvent::from_domain(Id::new(), &event, "  listing  ", "  corr_abc  ")
            .expect("valid");
        assert_eq!(outbox.aggregate_kind, "listing");
        assert_eq!(outbox.correlation_id, "corr_abc");
    }

    #[test]
    fn mark_published_sets_published_at() {
        let event = TestEvent::sample();
        let mut outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        let at = Utc.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).single().unwrap();
        outbox.mark_published(at);
        assert_eq!(outbox.published_at, Some(at));
        assert!(outbox.is_published());
    }

    #[test]
    fn mark_published_is_idempotent() {
        let event = TestEvent::sample();
        let mut outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        let first = Utc.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).single().unwrap();
        let second = Utc.with_ymd_and_hms(2026, 5, 2, 14, 0, 0).single().unwrap();
        outbox.mark_published(first);
        outbox.mark_published(second);
        // Idempotent — second call ignored, published_at unchanged.
        assert_eq!(outbox.published_at, Some(first));
    }

    #[test]
    fn is_published_false_when_published_at_none() {
        let event = TestEvent::sample();
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        assert!(!outbox.is_published());
    }

    #[test]
    fn is_published_true_after_mark_published() {
        let event = TestEvent::sample();
        let mut outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        outbox.mark_published(Utc::now());
        assert!(outbox.is_published());
    }

    #[test]
    fn serde_roundtrip_unpublished() {
        let event = TestEvent::sample();
        let outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        let json = serde_json::to_string(&outbox).expect("serialize");
        let back: OutboxEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(outbox, back);
    }

    #[test]
    fn serde_roundtrip_published() {
        let event = TestEvent::sample();
        let mut outbox =
            OutboxEvent::from_domain(Id::new(), &event, "listing", "corr_abc").expect("valid");
        outbox.mark_published(Utc.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).single().unwrap());
        let json = serde_json::to_string(&outbox).expect("serialize");
        let back: OutboxEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(outbox, back);
        assert!(back.is_published());
    }

    #[test]
    fn from_domain_works_with_dyn_domain_event() {
        // 다형 dispatch 가능 — `Box<dyn DomainEvent>` 도 받을 수 있어야 해요.
        let event: Box<dyn DomainEvent> = Box::new(TestEvent::sample());
        let outbox = OutboxEvent::from_domain(Id::new(), event.as_ref(), "listing", "corr_abc")
            .expect("valid");
        assert_eq!(outbox.event_type, "listing.approved");
    }
}
