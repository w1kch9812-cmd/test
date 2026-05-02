//! 도메인 ID — `<3-char prefix>_<26 char ULID>` 형식, 총 30자.
//!
//! Phantom-typed marker로 BC 간 ID 혼선을 컴파일 타임에 차단해요.

// `IdPrefix`, `IdError`, `Id` 처럼 모듈명을 반복하는 건 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// 도메인 ID prefix marker.
///
/// 각 BC는 고유 marker type을 선언하고 `IdPrefix`를 구현해요.
pub trait IdPrefix {
    /// 3자 prefix (예: `"usr"`, `"lst"`).
    const PREFIX: &'static str;
}

/// User aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserMarker;
impl IdPrefix for UserMarker {
    const PREFIX: &'static str = "usr";
}

/// Listing aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListingMarker;
impl IdPrefix for ListingMarker {
    const PREFIX: &'static str = "lst";
}

/// `ListingPhoto` aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListingPhotoMarker;
impl IdPrefix for ListingPhotoMarker {
    const PREFIX: &'static str = "lph";
}

/// `BookmarkExternal` aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BookmarkExternalMarker;
impl IdPrefix for BookmarkExternalMarker {
    const PREFIX: &'static str = "bme";
}

/// `SearchHistory` aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SearchHistoryMarker;
impl IdPrefix for SearchHistoryMarker {
    const PREFIX: &'static str = "srh";
}

/// `AnalysisReport` aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisReportMarker;
impl IdPrefix for AnalysisReportMarker {
    const PREFIX: &'static str = "rpt";
}

/// `Notification` aggregate ID marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationMarker;
impl IdPrefix for NotificationMarker {
    const PREFIX: &'static str = "ntf";
}

/// `AuditLog` aggregate ID marker.
///
/// Append-only — V002 immutable trigger blocks `UPDATE`/`DELETE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuditLogMarker;
impl IdPrefix for AuditLogMarker {
    const PREFIX: &'static str = "aud";
}

/// `OutboxEvent` aggregate ID marker.
///
/// Outbox pattern — `DomainEvent` 가 application layer 에서 `OutboxEvent` 로 wrap 되어
/// 같은 트랜잭션에 INSERT 됩니다. Publisher 워커 (sub-project 4) 가 미발행 row 를 fetch
/// 후 외부 시스템에 발행해요. PREFIX 는 spec § 5.3 `outbox_event.id` inline comment
/// (`evt_...`) 를 따릅니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutboxEventMarker;
impl IdPrefix for OutboxEventMarker {
    const PREFIX: &'static str = "evt";
}

/// `PipelineSchedule` aggregate ID marker.
///
/// Spec § 5.4 `pipeline_schedule.id` inline comment (`pls_...`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineScheduleMarker;
impl IdPrefix for PipelineScheduleMarker {
    const PREFIX: &'static str = "pls";
}

/// `PipelineRun` aggregate ID marker.
///
/// Spec § 5.4 `pipeline_run.id` inline comment (`plr_...`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineRunMarker;
impl IdPrefix for PipelineRunMarker {
    const PREFIX: &'static str = "plr";
}

/// `AdminAction` aggregate ID marker.
///
/// Spec § 5.5 `admin_action.id` inline comment (`ada_...`). Append-only — admin
/// 액션은 immutable (like `AuditLog`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdminActionMarker;
impl IdPrefix for AdminActionMarker {
    const PREFIX: &'static str = "ada";
}

/// `BusinessVerificationQueue` aggregate ID marker.
///
/// Spec § 5.5 `business_verification_queue.id` inline comment (`bvq_...`).
/// 사업자 인증 큐 — 4-status workflow (`pending`/`approved`/`rejected`/
/// `needs_more_info`) + version OCC + 24h SLA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BvqMarker;
impl IdPrefix for BvqMarker {
    const PREFIX: &'static str = "bvq";
}

/// 도메인 ID. 런타임은 30자 String, 타입은 phantom marker로 BC 구분.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<P: IdPrefix> {
    inner: String,
    #[serde(skip)]
    _marker: PhantomData<P>,
}

/// ID 검증 에러.
#[derive(Debug, Error)]
pub enum IdError {
    /// 길이가 30자가 아님.
    #[error("invalid id length: expected 30, got {actual}")]
    InvalidLength {
        /// 실제 길이.
        actual: usize,
    },
    /// '_' 구분자 누락.
    #[error("missing prefix delimiter '_'")]
    MissingDelimiter,
    /// prefix가 marker와 불일치.
    #[error("wrong prefix: expected {expected}, got {actual}")]
    WrongPrefix {
        /// 기대 prefix (`P::PREFIX`).
        expected: &'static str,
        /// 실제 prefix.
        actual: String,
    },
    /// ULID body 파싱 실패.
    #[error("invalid ULID body")]
    InvalidUlid,
}

impl<P: IdPrefix> Id<P> {
    /// 새 ID 생성. `<PREFIX>_<26-char ULID>` 형식, 항상 30자.
    #[must_use]
    pub fn new() -> Self {
        let raw = format!("{}_{}", P::PREFIX, Ulid::new());
        Self {
            inner: raw,
            _marker: PhantomData,
        }
    }

    /// 검증 후 `Id` 래핑.
    ///
    /// # Errors
    ///
    /// - 길이 ≠ 30: [`IdError::InvalidLength`]
    /// - `_` 구분자 누락: [`IdError::MissingDelimiter`]
    /// - prefix 불일치: [`IdError::WrongPrefix`]
    /// - ULID body 파싱 실패: [`IdError::InvalidUlid`]
    pub fn try_from_str(s: &str) -> Result<Self, IdError> {
        if s.len() != 30 {
            return Err(IdError::InvalidLength { actual: s.len() });
        }
        let (prefix, rest) = s.split_once('_').ok_or(IdError::MissingDelimiter)?;
        if prefix != P::PREFIX {
            return Err(IdError::WrongPrefix {
                expected: P::PREFIX,
                actual: prefix.to_owned(),
            });
        }
        Ulid::from_string(rest).map_err(|_| IdError::InvalidUlid)?;
        Ok(Self {
            inner: s.to_owned(),
            _marker: PhantomData,
        })
    }

    /// 내부 30자 문자열 슬라이스.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// 소유권을 포함한 내부 String을 반환해요.
    ///
    /// DB layer에서 owned 문자열이 필요할 때 사용해요.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.inner
    }
}

impl<P: IdPrefix> Default for Id<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: IdPrefix> std::fmt::Display for Id<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner)
    }
}

impl<P: IdPrefix> std::str::FromStr for Id<P> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn new_user_id_has_usr_prefix_and_total_30_chars() {
        let id: Id<UserMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("usr_"));
    }

    #[test]
    fn new_listing_id_has_lst_prefix() {
        let id: Id<ListingMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("lst_"));
    }

    #[test]
    fn new_listing_photo_id_has_lph_prefix() {
        let id: Id<ListingPhotoMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("lph_"));
    }

    #[test]
    fn new_bookmark_external_id_has_bme_prefix() {
        let id: Id<BookmarkExternalMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("bme_"));
    }

    #[test]
    fn new_search_history_id_has_srh_prefix() {
        let id: Id<SearchHistoryMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("srh_"));
    }

    #[test]
    fn new_analysis_report_id_has_rpt_prefix() {
        let id: Id<AnalysisReportMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("rpt_"));
    }

    #[test]
    fn new_notification_id_has_ntf_prefix() {
        let id: Id<NotificationMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("ntf_"));
    }

    #[test]
    fn new_audit_log_id_has_aud_prefix() {
        let id: Id<AuditLogMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("aud_"));
    }

    #[test]
    fn new_outbox_event_id_has_evt_prefix() {
        let id: Id<OutboxEventMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("evt_"));
    }

    #[test]
    fn new_pipeline_schedule_id_has_pls_prefix() {
        let id: Id<PipelineScheduleMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("pls_"));
    }

    #[test]
    fn new_pipeline_run_id_has_plr_prefix() {
        let id: Id<PipelineRunMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("plr_"));
    }

    #[test]
    fn new_admin_action_id_has_ada_prefix() {
        let id: Id<AdminActionMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("ada_"));
    }

    #[test]
    fn new_bvq_id_has_bvq_prefix() {
        let id: Id<BvqMarker> = Id::new();
        assert_eq!(id.as_str().len(), 30);
        assert!(id.as_str().starts_with("bvq_"));
    }

    #[test]
    fn two_new_ids_are_distinct() {
        let a: Id<UserMarker> = Id::new();
        let b: Id<UserMarker> = Id::new();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn parse_valid_id_roundtrips() {
        // 30 chars: 3 prefix + 1 underscore + 26 ULID base32 (Crockford alphabet).
        let raw = "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        assert_eq!(raw.len(), 30);
        let id = Id::<UserMarker>::try_from_str(raw).expect("valid format + valid ULID");
        assert_eq!(id.as_str(), raw);
    }

    #[test]
    fn parse_wrong_prefix_fails() {
        let raw = "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let err = Id::<UserMarker>::try_from_str(raw).unwrap_err();
        assert!(matches!(err, IdError::WrongPrefix { .. }));
    }

    #[test]
    fn parse_wrong_length_fails() {
        let err = Id::<UserMarker>::try_from_str("usr_short").unwrap_err();
        assert!(matches!(err, IdError::InvalidLength { actual: 9 }));
    }

    #[test]
    fn parse_invalid_ulid_body_fails() {
        // Right length and prefix, but `~` is not valid Crockford base32.
        let raw = "usr_~~~~~~~~~~~~~~~~~~~~~~~~~~";
        assert_eq!(raw.len(), 30);
        let err = Id::<UserMarker>::try_from_str(raw).unwrap_err();
        assert!(matches!(err, IdError::InvalidUlid));
    }

    #[test]
    fn parse_no_delimiter_fails() {
        // Length=30, prefix-like start, but no '_' anywhere.
        let raw = "usrXX01HXY3NK0Z9F6S1B2C3D4E5F6";
        assert_eq!(raw.len(), 30);
        let err = Id::<UserMarker>::try_from_str(raw).unwrap_err();
        assert!(matches!(err, IdError::MissingDelimiter));
    }

    #[test]
    fn display_renders_inner() {
        use std::str::FromStr;
        let raw = "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let id = Id::<UserMarker>::from_str(raw).expect("valid");
        assert_eq!(format!("{id}"), raw);
    }

    #[test]
    fn into_inner_yields_owned_string() {
        let raw = "usr_01HXY3NK0Z9F6S1B2C3D4E5F6G";
        let id = Id::<UserMarker>::try_from_str(raw).expect("valid");
        let owned: String = id.into_inner();
        assert_eq!(owned, raw);
    }
}
