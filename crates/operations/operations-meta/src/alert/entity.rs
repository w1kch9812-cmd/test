//! `SystemAlert` Aggregate (no OCC, acknowledge / resolve 1회 워크플로우).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, SystemAlertMarker, UserMarker};

use crate::alert::errors::SystemAlertError;
use crate::alert::severity::SystemAlertSeverity;

/// `source` 최대 길이 (spec § 5.5 `varchar(50)`).
const MAX_SOURCE_LEN: usize = 50;
/// `title` 최대 길이 (spec § 5.5 `varchar(200)`).
const MAX_TITLE_LEN: usize = 200;
/// `detail` 최대 길이 (도메인 sanity bound — DB 는 `text`).
const MAX_DETAIL_LEN: usize = 4000;

/// 시스템 알림 1건. acknowledge / resolve 1회 워크플로우 + JSONB metadata.
///
/// 10 필드 — spec § 5.5 `system_alert` 매핑. `version` 컬럼 없음.
///
/// ## 워크플로우
///
/// - **생성** — `try_new` 시 `acknowledged_at = None`, `acknowledged_by = None`,
///   `resolved_at = None`.
/// - **`acknowledge(by, at)`** — 한 번만. `acknowledged_at` 가 이미 `Some` 이면
///   `AlreadyAcknowledged` 에러. (Idempotent X — 워크플로우적 명시.)
/// - **`resolve(at)`** — 한 번만. `resolved_at` 가 이미 `Some` 이면 `AlreadyResolved`
///   에러. 사전 acknowledge 불필요 — 시스템 자동 복구로 acknowledge 생략 가능.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemAlert {
    /// 식별자 (`sal_<26 ULID>`).
    pub id: Id<SystemAlertMarker>,
    /// 심각도 (4값).
    pub severity: SystemAlertSeverity,
    /// 알림 source (어떤 시스템/모듈이 발생시켰는지, `varchar(50)`, 비어있지 않음).
    pub source: String,
    /// 알림 제목 (`varchar(200)`, 비어있지 않음).
    pub title: String,
    /// 알림 상세 (text, 선택, 4000자 sanity bound).
    pub detail: Option<String>,
    /// 부가 metadata (JSONB, 호출자가 자유롭게 채움, default `{}`).
    pub metadata: serde_json::Value,
    /// acknowledge 시각 (`Some` 이면 acknowledge 됨).
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// acknowledge 한 어드민 (`acknowledged_at` 와 동기).
    pub acknowledged_by: Option<Id<UserMarker>>,
    /// resolve 시각 (`Some` 이면 resolve 됨).
    pub resolved_at: Option<DateTime<Utc>>,
    /// 알림 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl SystemAlert {
    /// 검증 후 새 `SystemAlert` 생성. ID 자동 생성 (`sal_…`),
    /// `acknowledged_*` / `resolved_at = None`.
    ///
    /// # Errors
    ///
    /// - `source` 가 trim 후 빈 문자열 → [`SystemAlertError::EmptySource`].
    /// - `source` 가 50자 초과 → [`SystemAlertError::SourceTooLong`].
    /// - `title` 가 trim 후 빈 문자열 → [`SystemAlertError::EmptyTitle`].
    /// - `title` 가 200자 초과 → [`SystemAlertError::TitleTooLong`].
    /// - `detail` 가 `Some` 이고 4000자 초과 → [`SystemAlertError::DetailTooLong`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자 — spec column 매핑.
    pub fn try_new(
        severity: SystemAlertSeverity,
        source: &str,
        title: &str,
        detail: Option<&str>,
        metadata: serde_json::Value,
        created_at: DateTime<Utc>,
    ) -> Result<Self, SystemAlertError> {
        let source = validate_source(source)?;
        let title = validate_title(title)?;
        let detail = match detail {
            Some(d) => {
                let len = d.chars().count();
                if len > MAX_DETAIL_LEN {
                    return Err(SystemAlertError::DetailTooLong { actual: len });
                }
                Some(d.to_owned())
            }
            None => None,
        };
        Ok(Self {
            id: Id::new(),
            severity,
            source,
            title,
            detail,
            metadata,
            acknowledged_at: None,
            acknowledged_by: None,
            resolved_at: None,
            created_at,
        })
    }

    /// acknowledge 처리 — `acknowledged_at` + `acknowledged_by` 설정.
    ///
    /// # Errors
    ///
    /// 이미 acknowledge 된 경우 [`SystemAlertError::AlreadyAcknowledged`]. (Idempotent X.)
    pub fn acknowledge(
        &mut self,
        by: Id<UserMarker>,
        at: DateTime<Utc>,
    ) -> Result<(), SystemAlertError> {
        if self.acknowledged_at.is_some() {
            return Err(SystemAlertError::AlreadyAcknowledged);
        }
        self.acknowledged_at = Some(at);
        self.acknowledged_by = Some(by);
        Ok(())
    }

    /// resolve 처리 — `resolved_at` 설정. acknowledge 없이도 호출 가능
    /// (시스템 자동 복구 시).
    ///
    /// # Errors
    ///
    /// 이미 resolve 된 경우 [`SystemAlertError::AlreadyResolved`].
    pub const fn resolve(&mut self, at: DateTime<Utc>) -> Result<(), SystemAlertError> {
        if self.resolved_at.is_some() {
            return Err(SystemAlertError::AlreadyResolved);
        }
        self.resolved_at = Some(at);
        Ok(())
    }

    /// acknowledge 됐는지.
    #[must_use]
    pub const fn is_acknowledged(&self) -> bool {
        self.acknowledged_at.is_some()
    }

    /// resolve 됐는지.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        self.resolved_at.is_some()
    }

    /// 활성 (resolve 안 된) 알림인지 — `!is_resolved`.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !self.is_resolved()
    }
}

fn validate_source(value: &str) -> Result<String, SystemAlertError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(SystemAlertError::EmptySource);
    }
    let len = trimmed.chars().count();
    if len > MAX_SOURCE_LEN {
        return Err(SystemAlertError::SourceTooLong { actual: len });
    }
    Ok(trimmed)
}

fn validate_title(value: &str) -> Result<String, SystemAlertError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(SystemAlertError::EmptyTitle);
    }
    let len = trimmed.chars().count();
    if len > MAX_TITLE_LEN {
        return Err(SystemAlertError::TitleTooLong { actual: len });
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Duration;
    use serde_json::json;

    fn make_alert(at: DateTime<Utc>) -> SystemAlert {
        SystemAlert::try_new(
            SystemAlertSeverity::Error,
            "vworld_client",
            "V-World API rate limit exceeded",
            Some("threshold=1000/hour, current=1500"),
            json!({"region": "seoul"}),
            at,
        )
        .expect("valid alert")
    }

    // ── try_new ───────────────────────────────────────────────────

    #[test]
    fn try_new_happy_path() {
        let now = Utc::now();
        let a = make_alert(now);
        assert_eq!(a.severity, SystemAlertSeverity::Error);
        assert_eq!(a.source, "vworld_client");
        assert_eq!(a.title, "V-World API rate limit exceeded");
        assert_eq!(
            a.detail.as_deref(),
            Some("threshold=1000/hour, current=1500")
        );
        assert_eq!(a.metadata, json!({"region": "seoul"}));
        assert!(a.acknowledged_at.is_none());
        assert!(a.acknowledged_by.is_none());
        assert!(a.resolved_at.is_none());
        assert_eq!(a.created_at, now);
    }

    #[test]
    fn try_new_id_has_sal_prefix() {
        let now = Utc::now();
        let a = make_alert(now);
        assert!(a.id.as_str().starts_with("sal_"));
        assert_eq!(a.id.as_str().len(), 30);
    }

    #[test]
    fn try_new_with_empty_source_errors() {
        let err = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            "   ",
            "title",
            None,
            json!({}),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, SystemAlertError::EmptySource);
    }

    #[test]
    fn try_new_with_51_char_source_errors() {
        let too_long = "X".repeat(51);
        let err = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            &too_long,
            "title",
            None,
            json!({}),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, SystemAlertError::SourceTooLong { actual: 51 });
    }

    #[test]
    fn try_new_with_empty_title_errors() {
        let err = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            "src",
            "   ",
            None,
            json!({}),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, SystemAlertError::EmptyTitle);
    }

    #[test]
    fn try_new_with_201_char_title_errors() {
        let too_long = "X".repeat(201);
        let err = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            "src",
            &too_long,
            None,
            json!({}),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, SystemAlertError::TitleTooLong { actual: 201 });
    }

    #[test]
    fn try_new_with_4001_char_detail_errors() {
        let too_long = "X".repeat(4001);
        let err = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            "src",
            "title",
            Some(&too_long),
            json!({}),
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(err, SystemAlertError::DetailTooLong { actual: 4001 });
    }

    #[test]
    fn try_new_with_no_detail_accepted() {
        let a = SystemAlert::try_new(
            SystemAlertSeverity::Info,
            "src",
            "ok",
            None,
            json!({}),
            Utc::now(),
        )
        .expect("ok");
        assert!(a.detail.is_none());
    }

    // ── acknowledge ───────────────────────────────────────────────

    #[test]
    fn acknowledge_happy_path() {
        let now = Utc::now();
        let mut a = make_alert(now);
        let admin = Id::<UserMarker>::new();
        let later = now + Duration::minutes(5);
        a.acknowledge(admin.clone(), later).expect("ack ok");
        assert_eq!(a.acknowledged_at, Some(later));
        assert_eq!(a.acknowledged_by, Some(admin));
        assert!(a.is_acknowledged());
        assert!(!a.is_resolved());
        assert!(a.is_active());
    }

    #[test]
    fn acknowledge_already_errors() {
        let now = Utc::now();
        let mut a = make_alert(now);
        a.acknowledge(Id::new(), now + Duration::minutes(1))
            .expect("first ok");
        let err = a
            .acknowledge(Id::new(), now + Duration::minutes(2))
            .unwrap_err();
        assert_eq!(err, SystemAlertError::AlreadyAcknowledged);
    }

    // ── resolve ───────────────────────────────────────────────────

    #[test]
    fn resolve_happy_path_no_prior_ack() {
        let now = Utc::now();
        let mut a = make_alert(now);
        let later = now + Duration::minutes(10);
        a.resolve(later).expect("resolve ok");
        assert_eq!(a.resolved_at, Some(later));
        assert!(a.is_resolved());
        assert!(!a.is_active());
        // 사전 ack 없이도 resolve 가능.
        assert!(!a.is_acknowledged());
    }

    #[test]
    fn resolve_happy_path_after_ack() {
        let now = Utc::now();
        let mut a = make_alert(now);
        a.acknowledge(Id::new(), now + Duration::minutes(1))
            .expect("ack ok");
        a.resolve(now + Duration::minutes(5)).expect("resolve ok");
        assert!(a.is_acknowledged());
        assert!(a.is_resolved());
    }

    #[test]
    fn resolve_already_errors() {
        let now = Utc::now();
        let mut a = make_alert(now);
        a.resolve(now + Duration::minutes(1)).expect("first ok");
        let err = a.resolve(now + Duration::minutes(2)).unwrap_err();
        assert_eq!(err, SystemAlertError::AlreadyResolved);
    }

    // ── status helpers ────────────────────────────────────────────

    #[test]
    fn is_acknowledged_false_when_fresh() {
        let now = Utc::now();
        let a = make_alert(now);
        assert!(!a.is_acknowledged());
    }

    #[test]
    fn is_resolved_false_when_fresh_then_true_after() {
        let now = Utc::now();
        let mut a = make_alert(now);
        assert!(!a.is_resolved());
        a.resolve(now + Duration::minutes(1)).expect("ok");
        assert!(a.is_resolved());
    }

    #[test]
    fn is_active_inverts_is_resolved() {
        let now = Utc::now();
        let mut a = make_alert(now);
        assert!(a.is_active());
        a.resolve(now + Duration::minutes(1)).expect("ok");
        assert!(!a.is_active());
    }

    // ── serde ─────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_fresh() {
        let now = Utc::now();
        let a = make_alert(now);
        let json = serde_json::to_string(&a).expect("serialize");
        let back: SystemAlert = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(a, back);
    }

    #[test]
    fn serde_roundtrip_after_ack_and_resolve() {
        let now = Utc::now();
        let mut a = make_alert(now);
        a.acknowledge(Id::new(), now + Duration::minutes(1))
            .expect("ack ok");
        a.resolve(now + Duration::minutes(5)).expect("resolve ok");
        let json = serde_json::to_string(&a).expect("serialize");
        let back: SystemAlert = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(a, back);
    }

    // ── trim ──────────────────────────────────────────────────────

    #[test]
    fn try_new_trims_source_and_title() {
        let a = SystemAlert::try_new(
            SystemAlertSeverity::Warning,
            "  src  ",
            "  title  ",
            None,
            json!({}),
            Utc::now(),
        )
        .expect("ok");
        assert_eq!(a.source, "src");
        assert_eq!(a.title, "title");
    }
}
