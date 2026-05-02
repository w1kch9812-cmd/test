//! 도메인 이벤트 trait — `Aggregate` 도메인 메서드가 emit하는 이벤트의 공통 형태.
//!
//! `Outbox` 패턴의 첫 단계. 다음 흐름:
//!
//! 1. `Aggregate` 도메인 메서드 (`Listing::approve`, `User::verify_business` 등)가 상태 변경 후
//!    `Vec<Box<dyn DomainEvent>>` 또는 BC별 enum wrapper로 이벤트 반환
//! 2. Application layer (sub-project 5 handler)가 이벤트를 받아 `OutboxRepository::save`로 저장
//! 3. Outbox publisher worker (sub-project 4)가 미배포 이벤트를 fetch + 외부 시스템 발행

use chrono::{DateTime, Utc};
use serde_json::Value;

/// 도메인 이벤트.
///
/// 모든 도메인 이벤트는 이 trait을 구현해요. `Box<dyn DomainEvent>`로 다형 처리 가능.
///
/// # 예시
///
/// ```
/// use chrono::{DateTime, Utc};
/// use serde_json::{json, Value};
/// use shared_kernel::domain_event::DomainEvent;
///
/// #[derive(Debug)]
/// struct ListingApproved {
///     listing_id: String,
///     approved_by: String,
///     occurred_at: DateTime<Utc>,
/// }
///
/// impl DomainEvent for ListingApproved {
///     fn event_type(&self) -> &'static str { "listing.approved" }
///     fn occurred_at(&self) -> DateTime<Utc> { self.occurred_at }
///     fn aggregate_id(&self) -> String { self.listing_id.clone() }
///     fn payload(&self) -> Value {
///         json!({ "approved_by": self.approved_by })
///     }
/// }
/// ```
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// 이벤트 종류 (`<aggregate>.<verb>` 패턴).
    ///
    /// 예: `"listing.approved"`, `"user.business_verified"`, `"bookmark.added"`.
    fn event_type(&self) -> &'static str;

    /// 이벤트 발생 시각 (UTC).
    fn occurred_at(&self) -> DateTime<Utc>;

    /// 관련 `Aggregate` 식별자 (string 표현).
    ///
    /// 다른 BC에서 이 이벤트의 *소속 `Aggregate`*를 찾을 수 있게 해요.
    fn aggregate_id(&self) -> String;

    /// 이벤트 페이로드 (JSON).
    ///
    /// 외부 시스템에 발행될 때 직렬화돼 전송돼요. 민감 정보 포함 금지 (PII는 식별자로 대체).
    fn payload(&self) -> Value;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    /// Sample event — `Listing` approval.
    #[derive(Debug)]
    struct ListingApproved {
        listing_id: String,
        approved_by: String,
        occurred_at: DateTime<Utc>,
    }

    impl DomainEvent for ListingApproved {
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

    /// Sample event — `User` business verification.
    #[derive(Debug)]
    struct UserBusinessVerified {
        user_id: String,
        business_number: String,
        occurred_at: DateTime<Utc>,
    }

    impl DomainEvent for UserBusinessVerified {
        fn event_type(&self) -> &'static str {
            "user.business_verified"
        }
        fn occurred_at(&self) -> DateTime<Utc> {
            self.occurred_at
        }
        fn aggregate_id(&self) -> String {
            self.user_id.clone()
        }
        fn payload(&self) -> Value {
            json!({ "business_number": self.business_number })
        }
    }

    #[test]
    fn listing_approved_event_shape() {
        let now = Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).single().unwrap();
        let event = ListingApproved {
            listing_id: "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
            approved_by: "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
            occurred_at: now,
        };
        assert_eq!(event.event_type(), "listing.approved");
        assert_eq!(event.occurred_at(), now);
        assert_eq!(event.aggregate_id(), "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G");
        let payload = event.payload();
        assert_eq!(payload["approved_by"], "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G");
    }

    #[test]
    fn user_business_verified_event_shape() {
        let now = Utc::now();
        let event = UserBusinessVerified {
            user_id: "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
            business_number: "1234567891".to_owned(),
            occurred_at: now,
        };
        assert_eq!(event.event_type(), "user.business_verified");
        assert_eq!(event.aggregate_id(), "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G");
        assert_eq!(event.payload()["business_number"], "1234567891");
    }

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check: can we make a Box<dyn DomainEvent>?
        let event: Box<dyn DomainEvent> = Box::new(ListingApproved {
            listing_id: "lst_test".to_owned(),
            approved_by: "usr_test".to_owned(),
            occurred_at: Utc::now(),
        });
        assert_eq!(event.event_type(), "listing.approved");
    }

    #[test]
    fn vec_of_dyn_events_works() {
        let events: Vec<Box<dyn DomainEvent>> = vec![
            Box::new(ListingApproved {
                listing_id: "lst_1".to_owned(),
                approved_by: "usr_1".to_owned(),
                occurred_at: Utc::now(),
            }),
            Box::new(UserBusinessVerified {
                user_id: "usr_1".to_owned(),
                business_number: "1234567891".to_owned(),
                occurred_at: Utc::now(),
            }),
        ];
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type(), "listing.approved");
        assert_eq!(events[1].event_type(), "user.business_verified");
    }
}
