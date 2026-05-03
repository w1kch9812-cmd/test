//! `MutationContext` — 모든 audit/outbox transactional save 의 입력.

#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::domain_event::DomainEvent;
use crate::id::{Id, UserMarker};

/// 모든 mutation 의 audit/outbox 컨텍스트.
///
/// 호출자 (application layer) 가 누가/왜/무엇을 명시. `PgRepository` 가 트랜잭션
/// 안에서 `audit_log` / `outbox_event` `INSERT` 를 자동 수행해요.
///
/// 시스템 mutation (pipeline/scheduler) 은 [`Self::new_system_action`] 사용 —
/// `actor_id = None`.
#[derive(Debug, Clone)]
pub struct MutationContext {
    /// 누가 (`None` = system action — pipeline scheduler 등).
    pub actor_id: Option<Id<UserMarker>>,
    /// `HTTP` 요청 `ID` 또는 pipeline run `ID` (구조적 로그 / `Tempo` 연결).
    pub correlation_id: String,
    /// 도메인 의미 (예: `"create"`, `"update"`, `"approve"`, `"reject"`,
    /// `"acknowledge"`). `"save"` 같은 무의미 값 금지.
    pub action: String,
    /// 추가 메타데이터 — `audit_log.after_state` `JSONB` 로 매핑.
    pub metadata: Option<Value>,
    /// 본 mutation 이 발행하는 도메인 이벤트들 (`Outbox` 로 전파).
    pub events: Vec<Arc<dyn DomainEvent>>,
    /// 클라이언트 `IP` (`HTTP` 요청 시).
    pub client_ip: Option<String>,
    /// 클라이언트 `User-Agent`.
    pub user_agent: Option<String>,
    /// mutation 발생 시각. `None` 이면 `PgRepository` 가 `Utc::now()` 사용.
    pub occurred_at: Option<DateTime<Utc>>,
}

impl MutationContext {
    /// 인증된 사용자가 일으킨 mutation.
    #[must_use]
    pub fn new_user_action(
        actor_id: Id<UserMarker>,
        correlation_id: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            actor_id: Some(actor_id),
            correlation_id: correlation_id.into(),
            action: action.into(),
            metadata: None,
            events: Vec::new(),
            client_ip: None,
            user_agent: None,
            occurred_at: None,
        }
    }

    /// 시스템 (pipeline / scheduler / cron) 이 일으킨 mutation.
    #[must_use]
    pub fn new_system_action(correlation_id: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            actor_id: None,
            correlation_id: correlation_id.into(),
            action: action.into(),
            metadata: None,
            events: Vec::new(),
            client_ip: None,
            user_agent: None,
            occurred_at: None,
        }
    }

    /// 추가 메타데이터 부여 (builder).
    #[must_use]
    pub fn with_metadata(mut self, m: Value) -> Self {
        self.metadata = Some(m);
        self
    }

    /// 도메인 이벤트 부여 (builder).
    #[must_use]
    pub fn with_events(mut self, events: Vec<Arc<dyn DomainEvent>>) -> Self {
        self.events = events;
        self
    }

    /// 클라이언트 정보 부여 (builder).
    #[must_use]
    pub fn with_client_info(mut self, ip: impl Into<String>, ua: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self.user_agent = Some(ua.into());
        self
    }

    /// 발생 시각 부여 (builder, 테스트 결정성용).
    #[must_use]
    pub const fn with_occurred_at(mut self, at: DateTime<Utc>) -> Self {
        self.occurred_at = Some(at);
        self
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn new_user_action_sets_actor() {
        let actor: Id<UserMarker> = Id::new();
        let ctx = MutationContext::new_user_action(actor.clone(), "req-1", "approve");
        assert_eq!(
            ctx.actor_id.as_ref().map(|i| i.as_str()),
            Some(actor.as_str())
        );
        assert_eq!(ctx.correlation_id, "req-1");
        assert_eq!(ctx.action, "approve");
        assert!(ctx.events.is_empty());
        assert!(ctx.metadata.is_none());
    }

    #[test]
    fn new_system_action_no_actor() {
        let ctx = MutationContext::new_system_action("plr-1", "create");
        assert!(ctx.actor_id.is_none());
        assert_eq!(ctx.action, "create");
    }

    #[test]
    fn with_metadata_chainable() {
        let ctx = MutationContext::new_system_action("c", "update")
            .with_metadata(serde_json::json!({"reason": "test"}));
        assert!(ctx.metadata.is_some());
    }

    #[test]
    fn with_client_info_sets_both() {
        let ctx = MutationContext::new_system_action("c", "create")
            .with_client_info("10.0.0.1", "Mozilla/5.0");
        assert_eq!(ctx.client_ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(ctx.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    #[test]
    fn with_occurred_at_sets_time() {
        let now = Utc::now();
        let ctx = MutationContext::new_system_action("c", "create").with_occurred_at(now);
        assert_eq!(ctx.occurred_at, Some(now));
    }

    #[test]
    fn with_events_replaces_vec() {
        let ctx = MutationContext::new_system_action("c", "create").with_events(vec![]);
        assert!(ctx.events.is_empty());
    }
}
