//! `AdminAction` Aggregate (immutable, append-only).
//!
//! 어드민이 수행한 운영 액션 1건. **불변** — 도메인 모델에 mutation 메서드가
//! *없어요* (`AuditLog` 와 같은 설계). `try_new` 후 모든 필드 readonly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{AdminActionMarker, Id, UserMarker};

use crate::errors::AdminActionError;

/// `action_kind` 최대 길이 (spec § 5.5 `varchar(50)`).
const MAX_ACTION_KIND_LEN: usize = 50;
/// `target_kind` 최대 길이 (spec § 5.5 `varchar(30)`).
const MAX_TARGET_KIND_LEN: usize = 30;
/// `target_id` 최대 길이 (spec § 5.5 `varchar(50)`).
const MAX_TARGET_ID_LEN: usize = 50;
/// `correlation_id` 최대 길이 (spec § 5.5 `varchar(30)`).
const MAX_CORRELATION_ID_LEN: usize = 30;

/// 어드민 운영 액션 1건. **Immutable** — 생성 후 변경 불가.
///
/// `target_kind` 와 `target_id` 는 *둘 다 `Some` 또는 둘 다 `None`* 이어야 해요
/// (도메인 invariant). 한쪽만 `Some` 이면 [`AdminActionError::MismatchedTarget`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminAction {
    /// 식별자 (`ada_<26 ULID>`).
    pub id: Id<AdminActionMarker>,
    /// 어드민 사용자 (FK → `user.id`).
    pub admin_id: Id<UserMarker>,
    /// 액션 종류 (≤50자, 비어있지 않음). 예: `verify_business`, `approve_listing`,
    /// `force_pipeline_run`.
    pub action_kind: String,
    /// 대상 리소스 종류 (≤30자). `target_id` 와 동시에 `Some`/`None`.
    pub target_kind: Option<String>,
    /// 대상 리소스 식별자 (≤50자). `target_kind` 와 동시에 `Some`/`None`.
    pub target_id: Option<String>,
    /// 액션 payload (`JSONB`). 기본값은 호출자가 `serde_json::json!({})` 로 전달.
    pub payload: serde_json::Value,
    /// 분산 추적 `correlation_id` (≤30자, 비어있지 않음).
    pub correlation_id: String,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl AdminAction {
    /// 검증 후 새 [`AdminAction`] 생성.
    ///
    /// *No mutation methods* — append-only invariant (admin 액션은 immutable).
    ///
    /// # Errors
    ///
    /// - `action_kind` 빈 (trim 후) → [`AdminActionError::EmptyActionKind`].
    /// - `action_kind` 50자 초과 → [`AdminActionError::ActionKindTooLong`].
    /// - `target_kind` 가 `Some` 인데 30자 초과 → [`AdminActionError::TargetKindTooLong`].
    /// - `target_id` 가 `Some` 인데 50자 초과 → [`AdminActionError::TargetIdTooLong`].
    /// - `target_kind` 와 `target_id` 가 한쪽만 `Some` → [`AdminActionError::MismatchedTarget`].
    /// - `correlation_id` 빈 → [`AdminActionError::EmptyCorrelationId`].
    /// - `correlation_id` 30자 초과 → [`AdminActionError::CorrelationIdTooLong`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<AdminActionMarker>,
        admin_id: Id<UserMarker>,
        action_kind: &str,
        target_kind: Option<&str>,
        target_id: Option<&str>,
        payload: serde_json::Value,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, AdminActionError> {
        let action_kind = action_kind.trim().to_owned();
        if action_kind.is_empty() {
            return Err(AdminActionError::EmptyActionKind);
        }
        if action_kind.chars().count() > MAX_ACTION_KIND_LEN {
            return Err(AdminActionError::ActionKindTooLong {
                actual: action_kind.chars().count(),
            });
        }

        let target_kind_owned = target_kind.map(|t| t.trim().to_owned());
        let target_id_owned = target_id.map(|t| t.trim().to_owned());

        // Invariant: 둘 다 Some 또는 둘 다 None.
        match (&target_kind_owned, &target_id_owned) {
            (Some(_), None) | (None, Some(_)) => {
                return Err(AdminActionError::MismatchedTarget);
            }
            _ => {}
        }

        if let Some(ref tk) = target_kind_owned {
            if tk.chars().count() > MAX_TARGET_KIND_LEN {
                return Err(AdminActionError::TargetKindTooLong {
                    actual: tk.chars().count(),
                });
            }
        }
        if let Some(ref ti) = target_id_owned {
            if ti.chars().count() > MAX_TARGET_ID_LEN {
                return Err(AdminActionError::TargetIdTooLong {
                    actual: ti.chars().count(),
                });
            }
        }

        let correlation_id = correlation_id.trim().to_owned();
        if correlation_id.is_empty() {
            return Err(AdminActionError::EmptyCorrelationId);
        }
        if correlation_id.chars().count() > MAX_CORRELATION_ID_LEN {
            return Err(AdminActionError::CorrelationIdTooLong {
                actual: correlation_id.chars().count(),
            });
        }

        Ok(Self {
            id,
            admin_id,
            action_kind,
            target_kind: target_kind_owned,
            target_id: target_id_owned,
            payload,
            correlation_id,
            created_at: now,
        })
    }

    /// `target_kind` 와 `target_id` 가 둘 다 `Some` 인지 (도메인 invariant 상
    /// 둘 다 `Some` 또는 둘 다 `None` 이므로, 한쪽만 검사하면 충분해요).
    #[must_use]
    pub const fn has_target(&self) -> bool {
        self.target_kind.is_some()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn sample_payload() -> serde_json::Value {
        serde_json::json!({"reason": "manual override", "ticket": "OPS-42"})
    }

    fn make_full(
        action_kind: &str,
        target_kind: Option<&str>,
        target_id: Option<&str>,
        correlation_id: &str,
    ) -> Result<AdminAction, AdminActionError> {
        AdminAction::try_new(
            Id::new(),
            Id::new(),
            action_kind,
            target_kind,
            target_id,
            sample_payload(),
            correlation_id,
            Utc::now(),
        )
    }

    #[test]
    fn happy_path_with_target_both_some() {
        let action = make_full(
            "approve_listing",
            Some("listing"),
            Some("lst_01HXY3NK0Z9F6S1B2C3D4E5F6G"),
            "corr_01HXY3NK0Z9F6S1B2C3D4",
        )
        .expect("valid");
        assert_eq!(action.action_kind, "approve_listing");
        assert_eq!(action.target_kind.as_deref(), Some("listing"));
        assert_eq!(
            action.target_id.as_deref(),
            Some("lst_01HXY3NK0Z9F6S1B2C3D4E5F6G")
        );
        assert!(action.has_target());
    }

    #[test]
    fn happy_path_without_target_both_none() {
        let action = make_full("force_pipeline_run", None, None, "corr_x").expect("valid");
        assert_eq!(action.action_kind, "force_pipeline_run");
        assert!(action.target_kind.is_none());
        assert!(action.target_id.is_none());
        assert!(!action.has_target());
    }

    #[test]
    fn rejects_target_kind_some_target_id_none() {
        let err = make_full("verify_business", Some("user"), None, "corr_x").unwrap_err();
        assert!(matches!(err, AdminActionError::MismatchedTarget));
    }

    #[test]
    fn rejects_target_kind_none_target_id_some() {
        let err = make_full("verify_business", None, Some("usr_x"), "corr_x").unwrap_err();
        assert!(matches!(err, AdminActionError::MismatchedTarget));
    }

    #[test]
    fn rejects_empty_action_kind() {
        let err = make_full("", None, None, "corr_x").unwrap_err();
        assert!(matches!(err, AdminActionError::EmptyActionKind));
    }

    #[test]
    fn rejects_action_kind_over_50_chars() {
        let long = "X".repeat(51);
        let err = make_full(&long, None, None, "corr_x").unwrap_err();
        assert!(matches!(
            err,
            AdminActionError::ActionKindTooLong { actual: 51 }
        ));
    }

    #[test]
    fn rejects_target_kind_over_30_chars() {
        let long = "X".repeat(31);
        let err = make_full("approve_listing", Some(&long), Some("lst_x"), "corr_x").unwrap_err();
        assert!(matches!(
            err,
            AdminActionError::TargetKindTooLong { actual: 31 }
        ));
    }

    #[test]
    fn rejects_target_id_over_50_chars() {
        let long = "X".repeat(51);
        let err = make_full("approve_listing", Some("listing"), Some(&long), "corr_x").unwrap_err();
        assert!(matches!(
            err,
            AdminActionError::TargetIdTooLong { actual: 51 }
        ));
    }

    #[test]
    fn rejects_empty_correlation_id() {
        let err = make_full("approve_listing", Some("listing"), Some("lst_x"), "").unwrap_err();
        assert!(matches!(err, AdminActionError::EmptyCorrelationId));
    }

    #[test]
    fn rejects_correlation_id_over_30_chars() {
        let long = "X".repeat(31);
        let err = make_full("approve_listing", None, None, &long).unwrap_err();
        assert!(matches!(
            err,
            AdminActionError::CorrelationIdTooLong { actual: 31 }
        ));
    }

    #[test]
    fn has_target_true_when_both_some() {
        let action =
            make_full("approve_listing", Some("listing"), Some("lst_x"), "corr_x").expect("valid");
        assert!(action.has_target());
    }

    #[test]
    fn has_target_false_when_both_none() {
        let action = make_full("force_pipeline_run", None, None, "corr_x").expect("valid");
        assert!(!action.has_target());
    }

    #[test]
    fn serde_roundtrip_with_target() {
        let action = make_full(
            "approve_listing",
            Some("listing"),
            Some("lst_01HXY3NK0Z9F6S1B2C3D4E5F6G"),
            "corr_01HXY3NK0Z9F6S1B2C3D4",
        )
        .expect("valid");
        let json = serde_json::to_string(&action).expect("serialize");
        let back: AdminAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(action, back);
        assert!(back.has_target());
    }

    #[test]
    fn serde_roundtrip_without_target() {
        let action = make_full("force_pipeline_run", None, None, "corr_x").expect("valid");
        let json = serde_json::to_string(&action).expect("serialize");
        let back: AdminAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(action, back);
        assert!(!back.has_target());
    }

    #[test]
    fn payload_jsonb_roundtrip() {
        let payload = serde_json::json!({
            "reason": "manual override",
            "old_status": "pending",
            "new_status": "approved",
            "extras": [1, 2, 3]
        });
        let action = AdminAction::try_new(
            Id::new(),
            Id::new(),
            "approve_listing",
            Some("listing"),
            Some("lst_x"),
            payload.clone(),
            "corr_x",
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&action).expect("serialize");
        let back: AdminAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.payload, payload);
    }

    #[test]
    fn trim_normalizes_action_kind_and_correlation_id() {
        let action = make_full(
            "  approve_listing  ",
            Some("  listing  "),
            Some("  lst_x  "),
            "  corr_x  ",
        )
        .expect("valid");
        assert_eq!(action.action_kind, "approve_listing");
        assert_eq!(action.target_kind.as_deref(), Some("listing"));
        assert_eq!(action.target_id.as_deref(), Some("lst_x"));
        assert_eq!(action.correlation_id, "corr_x");
    }

    #[test]
    fn whitespace_only_action_kind_rejected_as_empty() {
        let err = make_full("    ", None, None, "corr_x").unwrap_err();
        assert!(matches!(err, AdminActionError::EmptyActionKind));
    }

    #[test]
    fn whitespace_only_correlation_id_rejected_as_empty() {
        let err = make_full("approve_listing", None, None, "    ").unwrap_err();
        assert!(matches!(err, AdminActionError::EmptyCorrelationId));
    }

    #[test]
    fn boundary_action_kind_exactly_50_chars_accepted() {
        let exactly = "X".repeat(50);
        let action = make_full(&exactly, None, None, "corr_x").expect("50 ok");
        assert_eq!(action.action_kind.chars().count(), 50);
    }

    #[test]
    fn boundary_target_kind_exactly_30_chars_accepted() {
        let exactly = "X".repeat(30);
        let action =
            make_full("approve_listing", Some(&exactly), Some("lst_x"), "corr_x").expect("30 ok");
        assert_eq!(
            action.target_kind.as_ref().map(|t| t.chars().count()),
            Some(30)
        );
    }

    #[test]
    fn boundary_target_id_exactly_50_chars_accepted() {
        let exactly = "X".repeat(50);
        let action =
            make_full("approve_listing", Some("listing"), Some(&exactly), "corr_x").expect("50 ok");
        assert_eq!(
            action.target_id.as_ref().map(|t| t.chars().count()),
            Some(50)
        );
    }

    #[test]
    fn boundary_correlation_id_exactly_30_chars_accepted() {
        let exactly = "X".repeat(30);
        let action = make_full("approve_listing", None, None, &exactly).expect("30 ok");
        assert_eq!(action.correlation_id.chars().count(), 30);
    }

    #[test]
    fn created_at_matches_now_argument() {
        let now = Utc::now();
        let action = AdminAction::try_new(
            Id::new(),
            Id::new(),
            "approve_listing",
            None,
            None,
            sample_payload(),
            "corr_x",
            now,
        )
        .expect("valid");
        assert_eq!(action.created_at, now);
    }

    #[test]
    fn empty_payload_object_accepted() {
        let action = AdminAction::try_new(
            Id::new(),
            Id::new(),
            "approve_listing",
            None,
            None,
            serde_json::json!({}),
            "corr_x",
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(action.payload, serde_json::json!({}));
    }
}
