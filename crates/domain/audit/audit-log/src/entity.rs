//! `AuditLog` Aggregate (immutable, append-only).
//!
//! V002 immutable trigger 가 DB 레벨에서 `UPDATE`/`DELETE` 를 차단해요.
//! 도메인 모델에도 mutation 메서드가 *없어요* — `try_new` 후 모든 필드 readonly.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{AuditLogMarker, Id, UserMarker};

use crate::errors::AuditLogError;

/// `action` 최대 길이 (spec § 5.3 `varchar(100)`).
const MAX_ACTION_LEN: usize = 100;
/// `resource_kind` 최대 길이 (spec § 5.3 `varchar(50)`).
const MAX_RESOURCE_KIND_LEN: usize = 50;
/// `resource_id` 최대 길이 (spec § 5.3 `varchar(50)`).
const MAX_RESOURCE_ID_LEN: usize = 50;
/// `correlation_id` 최대 길이 (spec § 5.3 `varchar(30)`).
const MAX_CORRELATION_ID_LEN: usize = 30;
/// `user_agent` 최대 길이 (도메인 합리적 상한 — DB는 `text`).
const MAX_USER_AGENT_LEN: usize = 500;

/// 감사 로그 1건. **Immutable** — 생성 후 변경 불가, V002 트리거가 `UPDATE`/`DELETE` 차단.
///
/// `actor_id` `None` = 시스템 행위 (system action — 배치/워커 등).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLog {
    /// 식별자 (`aud_<26 ULID>`).
    pub id: Id<AuditLogMarker>,
    /// 행위자 — `None` 이면 시스템 행위.
    pub actor_id: Option<Id<UserMarker>>,
    /// 행위 식별 문자열 (≤100자, 비어있지 않음).
    pub action: String,
    /// 대상 리소스 종류 (≤50자, 비어있지 않음).
    pub resource_kind: String,
    /// 대상 리소스 식별자 (≤50자, 비어있지 않음).
    pub resource_id: String,
    /// 변경 전 상태 (`JSONB`).
    pub before_state: Option<serde_json::Value>,
    /// 변경 후 상태 (`JSONB`).
    pub after_state: Option<serde_json::Value>,
    /// 행위자 IP 주소 (`IpAddr` — `IPv4`/`IPv6` 모두 지원).
    pub ip_address: Option<IpAddr>,
    /// User-Agent 문자열 (≤500자).
    pub user_agent: Option<String>,
    /// 분산 추적 `correlation_id` (≤30자, 비어있지 않음).
    pub correlation_id: String,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    /// 검증 후 새 [`AuditLog`] 생성.
    ///
    /// *No mutation methods* — append-only invariant per V002 트리거.
    ///
    /// # Errors
    ///
    /// - `action` 빈 (trim 후) → [`AuditLogError::EmptyAction`].
    /// - `action` 100자 초과 → [`AuditLogError::ActionTooLong`].
    /// - `resource_kind` 빈 → [`AuditLogError::EmptyResourceKind`].
    /// - `resource_kind` 50자 초과 → [`AuditLogError::ResourceKindTooLong`].
    /// - `resource_id` 빈 → [`AuditLogError::EmptyResourceId`].
    /// - `resource_id` 50자 초과 → [`AuditLogError::ResourceIdTooLong`].
    /// - `correlation_id` 빈 → [`AuditLogError::EmptyCorrelationId`].
    /// - `correlation_id` 30자 초과 → [`AuditLogError::CorrelationIdTooLong`].
    /// - `user_agent` 가 `Some` 인데 500자 초과 → [`AuditLogError::UserAgentTooLong`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<AuditLogMarker>,
        actor_id: Option<Id<UserMarker>>,
        action: &str,
        resource_kind: &str,
        resource_id: &str,
        before_state: Option<serde_json::Value>,
        after_state: Option<serde_json::Value>,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, AuditLogError> {
        let action = action.trim().to_owned();
        if action.is_empty() {
            return Err(AuditLogError::EmptyAction);
        }
        if action.chars().count() > MAX_ACTION_LEN {
            return Err(AuditLogError::ActionTooLong {
                actual: action.chars().count(),
            });
        }

        let resource_kind = resource_kind.trim().to_owned();
        if resource_kind.is_empty() {
            return Err(AuditLogError::EmptyResourceKind);
        }
        if resource_kind.chars().count() > MAX_RESOURCE_KIND_LEN {
            return Err(AuditLogError::ResourceKindTooLong {
                actual: resource_kind.chars().count(),
            });
        }

        let resource_id = resource_id.trim().to_owned();
        if resource_id.is_empty() {
            return Err(AuditLogError::EmptyResourceId);
        }
        if resource_id.chars().count() > MAX_RESOURCE_ID_LEN {
            return Err(AuditLogError::ResourceIdTooLong {
                actual: resource_id.chars().count(),
            });
        }

        let correlation_id = correlation_id.trim().to_owned();
        if correlation_id.is_empty() {
            return Err(AuditLogError::EmptyCorrelationId);
        }
        if correlation_id.chars().count() > MAX_CORRELATION_ID_LEN {
            return Err(AuditLogError::CorrelationIdTooLong {
                actual: correlation_id.chars().count(),
            });
        }

        if let Some(ref ua) = user_agent {
            if ua.chars().count() > MAX_USER_AGENT_LEN {
                return Err(AuditLogError::UserAgentTooLong {
                    actual: ua.chars().count(),
                });
            }
        }

        Ok(Self {
            id,
            actor_id,
            action,
            resource_kind,
            resource_id,
            before_state,
            after_state,
            ip_address,
            user_agent,
            correlation_id,
            created_at: now,
        })
    }

    /// 시스템 행위 (`actor_id` 가 `None`) 인지 여부.
    #[must_use]
    pub const fn is_system_action(&self) -> bool {
        self.actor_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::*;

    fn sample_before() -> serde_json::Value {
        serde_json::json!({"status": "draft", "title": "before"})
    }

    fn sample_after() -> serde_json::Value {
        serde_json::json!({"status": "published", "title": "after"})
    }

    fn make_full(
        action: &str,
        resource_kind: &str,
        resource_id: &str,
        correlation_id: &str,
    ) -> Result<AuditLog, AuditLogError> {
        AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            action,
            resource_kind,
            resource_id,
            Some(sample_before()),
            Some(sample_after()),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))),
            Some("Mozilla/5.0 (Test)".to_owned()),
            correlation_id,
            Utc::now(),
        )
    }

    #[test]
    fn happy_path_full_fields_populated() {
        let log = make_full(
            "listing.published",
            "listing",
            "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
            "corr_01HXY3NK0Z9F6S1B2C3D4",
        )
        .expect("valid");
        assert_eq!(log.action, "listing.published");
        assert_eq!(log.resource_kind, "listing");
        assert_eq!(log.resource_id, "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G");
        assert!(log.actor_id.is_some());
        assert!(log.before_state.is_some());
        assert!(log.after_state.is_some());
        assert!(log.ip_address.is_some());
        assert!(log.user_agent.is_some());
    }

    #[test]
    fn happy_path_system_action_actor_id_none() {
        let log = AuditLog::try_new(
            Id::new(),
            None, // system action
            "batch.cleanup_expired",
            "notification",
            "ntf_01HXY3NK0Z9F6S1B2C3D4E5F6G",
            None,
            None,
            None,
            None,
            "sys_01HXY3NK0Z9F6S1B2C3D4",
            Utc::now(),
        )
        .expect("valid");
        assert!(log.is_system_action());
        assert!(log.actor_id.is_none());
    }

    #[test]
    fn rejects_empty_action() {
        let err = make_full("", "listing", "lst_x", "corr_x").unwrap_err();
        assert!(matches!(err, AuditLogError::EmptyAction));
    }

    #[test]
    fn rejects_action_over_100_chars() {
        let long = "X".repeat(101);
        let err = make_full(&long, "listing", "lst_x", "corr_x").unwrap_err();
        assert!(matches!(err, AuditLogError::ActionTooLong { actual: 101 }));
    }

    #[test]
    fn rejects_empty_resource_kind() {
        let err = make_full("listing.published", "", "lst_x", "corr_x").unwrap_err();
        assert!(matches!(err, AuditLogError::EmptyResourceKind));
    }

    #[test]
    fn rejects_resource_kind_over_50_chars() {
        let long = "X".repeat(51);
        let err = make_full("listing.published", &long, "lst_x", "corr_x").unwrap_err();
        assert!(matches!(
            err,
            AuditLogError::ResourceKindTooLong { actual: 51 }
        ));
    }

    #[test]
    fn rejects_empty_resource_id() {
        let err = make_full("listing.published", "listing", "", "corr_x").unwrap_err();
        assert!(matches!(err, AuditLogError::EmptyResourceId));
    }

    #[test]
    fn rejects_resource_id_over_50_chars() {
        let long = "X".repeat(51);
        let err = make_full("listing.published", "listing", &long, "corr_x").unwrap_err();
        assert!(matches!(
            err,
            AuditLogError::ResourceIdTooLong { actual: 51 }
        ));
    }

    #[test]
    fn rejects_empty_correlation_id() {
        let err = make_full("listing.published", "listing", "lst_x", "").unwrap_err();
        assert!(matches!(err, AuditLogError::EmptyCorrelationId));
    }

    #[test]
    fn rejects_correlation_id_over_30_chars() {
        let long = "X".repeat(31);
        let err = make_full("listing.published", "listing", "lst_x", &long).unwrap_err();
        assert!(matches!(
            err,
            AuditLogError::CorrelationIdTooLong { actual: 31 }
        ));
    }

    #[test]
    fn rejects_user_agent_over_500_chars() {
        let long_ua = "X".repeat(501);
        let err = AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            "listing.published",
            "listing",
            "lst_x",
            None,
            None,
            None,
            Some(long_ua),
            "corr_x",
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AuditLogError::UserAgentTooLong { actual: 501 }
        ));
    }

    #[test]
    fn is_system_action_true_when_actor_none() {
        let log = AuditLog::try_new(
            Id::new(),
            None,
            "system.tick",
            "system",
            "tick",
            None,
            None,
            None,
            None,
            "corr_sys",
            Utc::now(),
        )
        .expect("valid");
        assert!(log.is_system_action());
    }

    #[test]
    fn is_system_action_false_when_actor_some() {
        let log = make_full("listing.published", "listing", "lst_x", "corr_x").expect("valid");
        assert!(!log.is_system_action());
    }

    #[test]
    fn serde_roundtrip_full() {
        let log = make_full(
            "listing.published",
            "listing",
            "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
            "corr_01HXY3NK0Z9F6S1B2C3D4",
        )
        .expect("valid");
        let json = serde_json::to_string(&log).expect("serialize");
        let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(log, back);
    }

    #[test]
    fn serde_roundtrip_system_action_with_nones() {
        let log = AuditLog::try_new(
            Id::new(),
            None,
            "batch.cleanup",
            "notification",
            "ntf_x",
            None,
            None,
            None,
            None,
            "sys_x",
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&log).expect("serialize");
        let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(log, back);
        assert!(back.is_system_action());
    }

    #[test]
    fn ip_address_v4_preserved() {
        let v4 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 42));
        let log = AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            "user.login",
            "user",
            "usr_x",
            None,
            None,
            Some(v4),
            None,
            "corr_x",
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(log.ip_address, Some(v4));
    }

    #[test]
    fn ip_address_v6_preserved() {
        let v6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let log = AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            "user.login",
            "user",
            "usr_x",
            None,
            None,
            Some(v6),
            None,
            "corr_x",
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(log.ip_address, Some(v6));
    }

    #[test]
    fn ip_address_none_preserved() {
        let log = AuditLog::try_new(
            Id::new(),
            None,
            "system.tick",
            "system",
            "tick",
            None,
            None,
            None,
            None,
            "sys_x",
            Utc::now(),
        )
        .expect("valid");
        assert!(log.ip_address.is_none());
    }

    #[test]
    fn before_after_state_jsonb_roundtrip() {
        let before = serde_json::json!({"price": 100, "status": "draft"});
        let after = serde_json::json!({"price": 200, "status": "published", "extra": [1, 2, 3]});
        let log = AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            "listing.update",
            "listing",
            "lst_x",
            Some(before.clone()),
            Some(after.clone()),
            None,
            None,
            "corr_x",
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&log).expect("serialize");
        let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.before_state, Some(before));
        assert_eq!(back.after_state, Some(after));
    }

    #[test]
    fn trim_normalizes_action_resource_kind_resource_id_correlation_id() {
        let log = make_full(
            "  listing.published  ",
            "  listing  ",
            "  lst_x  ",
            "  corr_x  ",
        )
        .expect("valid");
        assert_eq!(log.action, "listing.published");
        assert_eq!(log.resource_kind, "listing");
        assert_eq!(log.resource_id, "lst_x");
        assert_eq!(log.correlation_id, "corr_x");
    }

    #[test]
    fn whitespace_only_action_rejected_as_empty() {
        let err = make_full("    ", "listing", "lst_x", "corr_x").unwrap_err();
        assert!(matches!(err, AuditLogError::EmptyAction));
    }

    #[test]
    fn boundary_action_exactly_100_chars_accepted() {
        let exactly = "X".repeat(100);
        let log = make_full(&exactly, "listing", "lst_x", "corr_x").expect("100 ok");
        assert_eq!(log.action.chars().count(), 100);
    }

    #[test]
    fn boundary_user_agent_exactly_500_chars_accepted() {
        let exactly = "X".repeat(500);
        let log = AuditLog::try_new(
            Id::new(),
            Some(Id::new()),
            "user.login",
            "user",
            "usr_x",
            None,
            None,
            None,
            Some(exactly.clone()),
            "corr_x",
            Utc::now(),
        )
        .expect("500 ok");
        assert_eq!(log.user_agent.as_deref().map(str::len), Some(500));
        assert_eq!(log.user_agent, Some(exactly));
    }

    #[test]
    fn created_at_matches_now_argument() {
        let now = Utc::now();
        let log = AuditLog::try_new(
            Id::new(),
            None,
            "system.tick",
            "system",
            "tick",
            None,
            None,
            None,
            None,
            "sys_x",
            now,
        )
        .expect("valid");
        assert_eq!(log.created_at, now);
    }
}
