//! 인증 이벤트 → `audit_log` writer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// 인증 흐름에서 발생하는 6 종 이벤트.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event")]
pub enum AuthEvent {
    /// 첫 로그인 또는 새 세션 발급.
    Login {
        /// Zitadel subject.
        user_sub: String,
        /// `JWT` ID.
        jti: String,
        /// 만료 epoch seconds.
        exp: i64,
    },
    /// 로그아웃 (back-channel).
    Logout {
        /// Zitadel subject.
        user_sub: String,
        /// `JWT` ID.
        jti: String,
    },
    /// Refresh 성공 (`jti` rotation 포함).
    RefreshSucceeded {
        /// Zitadel subject.
        user_sub: String,
        /// 이전 `JWT` ID.
        prev_jti: String,
        /// 새 `JWT` ID.
        new_jti: String,
        /// 새 토큰 만료 epoch seconds.
        exp: i64,
    },
    /// Refresh 실패.
    RefreshFailed {
        /// Zitadel subject.
        user_sub: String,
        /// `JWT` ID.
        jti: String,
    },
    /// 권한 가드 거부 (role mismatch 등).
    RoleGuardDenied {
        /// Zitadel subject.
        user_sub: String,
        /// 필요한 role.
        required_role: String,
        /// 실제 role.
        actual_role: String,
        /// 요청 path.
        path: String,
    },
    /// Role 변경 — 모든 활성 `jti` 가 denylist 추가됨.
    RoleChanged {
        /// Zitadel subject.
        user_sub: String,
        /// 이전 role.
        prev_role: String,
        /// 새 role.
        new_role: String,
        /// 무효화된 `JTI` 수.
        invalidated_jti_count: u32,
    },
}

impl AuthEvent {
    /// `audit_log.action` 컬럼에 들어갈 dotted name.
    #[must_use]
    pub const fn action(&self) -> &'static str {
        match self {
            Self::Login { .. } => "auth.login",
            Self::Logout { .. } => "auth.logout",
            Self::RefreshSucceeded { .. } => "auth.refresh.succeeded",
            Self::RefreshFailed { .. } => "auth.refresh.failed",
            Self::RoleGuardDenied { .. } => "auth.role_guard.denied",
            Self::RoleChanged { .. } => "auth.role.changed",
        }
    }

    /// 추적용 `user_sub` 추출.
    #[must_use]
    pub const fn user_sub(&self) -> &str {
        match self {
            Self::Login { user_sub, .. }
            | Self::Logout { user_sub, .. }
            | Self::RefreshSucceeded { user_sub, .. }
            | Self::RefreshFailed { user_sub, .. }
            | Self::RoleGuardDenied { user_sub, .. }
            | Self::RoleChanged { user_sub, .. } => user_sub.as_str(),
        }
    }
}

/// `audit_log` 에 인증 이벤트를 기록해요.
///
/// `actor_id` 는 NULL — frontend 가 emit 하는 `user_sub` (Zitadel) 는 `users.zitadel_sub`
/// 와 매칭 후 `users.id` 로 바꿀 수 있지만 본 함수는 시스템 이벤트 처리만 함.
/// `after_state` 에 `user_sub` + `jti` + payload 저장 → role 변경 시 활성 `jti` 조회 가능.
///
/// # Errors
///
/// Postgres `INSERT` 실패 시 `sqlx::Error` 반환.
///
/// # Panics
///
/// `AuthEvent` JSON 직렬화 실패 시 panic — `AuthEvent` 는 항상 직렬화 가능하므로 실제 발생 없음.
pub async fn write(
    pool: &PgPool,
    event: &AuthEvent,
    correlation_id: &str,
) -> Result<(), sqlx::Error> {
    let id = format!("aud_{}", generate_id());
    // AuthEvent 는 항상 직렬화 가능 — 실패 시 Null 폴백.
    let payload = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);

    sqlx::query(
        r"
        INSERT INTO audit_log
          (id, actor_id, action, resource_kind, resource_id,
           before_state, after_state, correlation_id, created_at)
        VALUES ($1, NULL, $2, 'user', $3, NULL, $4, $5, $6)
        ",
    )
    .bind(&id)
    .bind(event.action())
    .bind(event.user_sub())
    .bind(&payload)
    .bind(correlation_id)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// 26-char alphanumeric ID (nanos hex 좌측 0 패딩, 정확히 26 char).
/// `aud_` / `cor_` prefix 와 함께 사용.
#[must_use]
pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = u128::from(std::process::id());
    // pid(10 hex) + nanos(32 hex) = 42 char → 첫 26 char 추출.
    // 짧아지는 경우 방지: 0 패딩으로 항상 42 char 이상 보장.
    let raw = format!("{pid:010x}{nanos:032x}");
    raw.chars().take(26).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn action_name_matches() {
        let e = AuthEvent::Login {
            user_sub: "u".into(),
            jti: "j".into(),
            exp: 1000,
        };
        assert_eq!(e.action(), "auth.login");
        assert_eq!(e.user_sub(), "u");
    }

    #[test]
    fn role_changed_action() {
        let e = AuthEvent::RoleChanged {
            user_sub: "u".into(),
            prev_role: "Buyer".into(),
            new_role: "Broker".into(),
            invalidated_jti_count: 3,
        };
        assert_eq!(e.action(), "auth.role.changed");
    }

    #[test]
    fn round_trip_serde() {
        let e = AuthEvent::RefreshSucceeded {
            user_sub: "u".into(),
            prev_jti: "j1".into(),
            new_jti: "j2".into(),
            exp: 1000,
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: AuthEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }

    #[test]
    fn generate_id_returns_26_chars() {
        let id = generate_id();
        assert_eq!(id.len(), 26);
    }

    #[test]
    fn all_six_actions_distinct() {
        use std::collections::HashSet;
        let actions = [
            AuthEvent::Login {
                user_sub: "u".into(),
                jti: "j".into(),
                exp: 0,
            }
            .action(),
            AuthEvent::Logout {
                user_sub: "u".into(),
                jti: "j".into(),
            }
            .action(),
            AuthEvent::RefreshSucceeded {
                user_sub: "u".into(),
                prev_jti: "p".into(),
                new_jti: "n".into(),
                exp: 0,
            }
            .action(),
            AuthEvent::RefreshFailed {
                user_sub: "u".into(),
                jti: "j".into(),
            }
            .action(),
            AuthEvent::RoleGuardDenied {
                user_sub: "u".into(),
                required_role: "a".into(),
                actual_role: "b".into(),
                path: "/x".into(),
            }
            .action(),
            AuthEvent::RoleChanged {
                user_sub: "u".into(),
                prev_role: "a".into(),
                new_role: "b".into(),
                invalidated_jti_count: 0,
            }
            .action(),
        ];
        let set: HashSet<_> = actions.iter().collect();
        assert_eq!(set.len(), 6);
    }
}
