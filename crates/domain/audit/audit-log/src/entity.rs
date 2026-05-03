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
#[path = "entity_tests.rs"]
mod tests;
