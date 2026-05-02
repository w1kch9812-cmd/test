//! `FeaturedContent` Aggregate (no OCC, V003_03 invariant `ends_at > starts_at`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{FeaturedContentMarker, Id, UserMarker};

use crate::featured::errors::FeaturedContentError;
use crate::featured::feature_kind::FeaturedContentFeatureKind;
use crate::featured::target_kind::FeaturedContentTargetKind;

/// `target_id` 최대 길이 (spec § 5.5 `varchar(50)`).
const MAX_TARGET_ID_LEN: usize = 50;

/// 홈페이지 추천/광고/스폰서 노출 콘텐츠.
///
/// 12 필드 — spec § 5.5 `featured_content` 매핑. `version` 컬럼 없음.
///
/// ## V003_03 invariant
///
/// `ends_at > starts_at` — DB CHECK `featured_content_time_bound_chk` 와 동일하게
/// Aggregate 검증에서 차단. 구간 길이 0 (동일 시각) 도 거부.
///
/// ## 카운터
///
/// `record_impression` / `record_click` 은 `i64::MAX` 에서 saturate. admin 레이어의
/// 동시성 race 는 *비즈니스적으로* 허용 (광고 카운트는 정확성 < 비용).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturedContent {
    /// 식별자 (`fea_<26 ULID>`).
    pub id: Id<FeaturedContentMarker>,
    /// 대상 종류 (3값).
    pub target_kind: FeaturedContentTargetKind,
    /// 대상 ID (varchar(50), polymorphic — `Listing` 의 30자 ID 또는 외부 ID).
    pub target_id: String,
    /// 노출 슬롯 종류 (4값).
    pub feature_kind: FeaturedContentFeatureKind,
    /// 노출 가중치 (`>=0`, default 1).
    pub weight: i32,
    /// 노출 시작.
    pub starts_at: DateTime<Utc>,
    /// 노출 종료 (V003_03: `> starts_at`).
    pub ends_at: DateTime<Utc>,
    /// 결제한 사용자 (FK → `user.id`). Phase 2+ 결제 연동 전엔 `None`.
    pub purchased_by: Option<Id<UserMarker>>,
    /// 노출(`impression`) 누적 카운트 (`>=0`, saturating).
    pub impression_count: i64,
    /// 클릭 누적 카운트 (`>=0`, saturating).
    pub click_count: i64,
    /// 등록 시각.
    pub created_at: DateTime<Utc>,
}

impl FeaturedContent {
    /// 검증 후 새 `FeaturedContent` 생성. ID 자동 생성 (`fea_…`),
    /// `impression_count = 0, click_count = 0`.
    ///
    /// # Errors
    ///
    /// - `target_id` 가 trim 후 빈 문자열 → [`FeaturedContentError::EmptyTargetId`].
    /// - `target_id` 가 50자 초과 → [`FeaturedContentError::TargetIdTooLong`].
    /// - `weight < 0` → [`FeaturedContentError::NegativeWeight`].
    /// - `ends_at <= starts_at` → [`FeaturedContentError::InvalidTimeBound`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자 — spec column 매핑.
    pub fn try_new(
        target_kind: FeaturedContentTargetKind,
        target_id: String,
        feature_kind: FeaturedContentFeatureKind,
        weight: i32,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
        purchased_by: Option<Id<UserMarker>>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, FeaturedContentError> {
        let trimmed = target_id.trim().to_owned();
        if trimmed.is_empty() {
            return Err(FeaturedContentError::EmptyTargetId);
        }
        let len = trimmed.chars().count();
        if len > MAX_TARGET_ID_LEN {
            return Err(FeaturedContentError::TargetIdTooLong { actual: len });
        }
        if weight < 0 {
            return Err(FeaturedContentError::NegativeWeight { actual: weight });
        }
        if ends_at <= starts_at {
            return Err(FeaturedContentError::InvalidTimeBound);
        }
        Ok(Self {
            id: Id::new(),
            target_kind,
            target_id: trimmed,
            feature_kind,
            weight,
            starts_at,
            ends_at,
            purchased_by,
            impression_count: 0,
            click_count: 0,
            created_at,
        })
    }

    /// 시점 `t` 에 활성인지 — `starts_at <= t < ends_at` (half-open interval).
    #[must_use]
    pub fn is_active_at(&self, t: DateTime<Utc>) -> bool {
        self.starts_at <= t && t < self.ends_at
    }

    /// `impression_count` 1 증가 (saturating). 실패 없음.
    pub const fn record_impression(&mut self) {
        self.impression_count = self.impression_count.saturating_add(1);
    }

    /// `click_count` 1 증가 (saturating). 실패 없음.
    pub const fn record_click(&mut self) {
        self.click_count = self.click_count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Duration;

    fn make_fc(starts: DateTime<Utc>, ends: DateTime<Utc>) -> FeaturedContent {
        FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            5,
            starts,
            ends,
            None,
            starts,
        )
        .expect("valid fc")
    }

    // ── try_new happy + ID ────────────────────────────────────────

    #[test]
    fn try_new_happy_path() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(24));
        assert_eq!(fc.weight, 5);
        assert_eq!(fc.target_kind, FeaturedContentTargetKind::Listing);
        assert_eq!(
            fc.feature_kind,
            FeaturedContentFeatureKind::HomepageFeatured
        );
        assert_eq!(fc.impression_count, 0);
        assert_eq!(fc.click_count, 0);
        assert!(fc.purchased_by.is_none());
        assert_eq!(fc.created_at, now);
    }

    #[test]
    fn try_new_id_has_fea_prefix() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(1));
        assert!(fc.id.as_str().starts_with("fea_"));
        assert_eq!(fc.id.as_str().len(), 30);
    }

    #[test]
    fn try_new_trims_target_id() {
        let now = Utc::now();
        let fc = FeaturedContent::try_new(
            FeaturedContentTargetKind::Manufacturer,
            "  mfg-123  ".to_owned(),
            FeaturedContentFeatureKind::Newsletter,
            1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .expect("ok");
        assert_eq!(fc.target_id, "mfg-123");
    }

    // ── try_new validations ───────────────────────────────────────

    #[test]
    fn try_new_with_empty_target_id_errors() {
        let now = Utc::now();
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            String::new(),
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::EmptyTargetId);
    }

    #[test]
    fn try_new_with_whitespace_target_id_errors() {
        let now = Utc::now();
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "   \t\n".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::EmptyTargetId);
    }

    #[test]
    fn try_new_with_51_char_target_id_errors() {
        let now = Utc::now();
        let too_long = "X".repeat(51);
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            too_long,
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::TargetIdTooLong { actual: 51 });
    }

    #[test]
    fn try_new_with_50_char_target_id_accepted() {
        let now = Utc::now();
        let exactly = "X".repeat(50);
        let fc = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            exactly.clone(),
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .expect("50 ok");
        assert_eq!(fc.target_id, exactly);
    }

    #[test]
    fn try_new_with_negative_weight_errors() {
        let now = Utc::now();
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "lst-x".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            -1,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::NegativeWeight { actual: -1 });
    }

    #[test]
    fn try_new_with_zero_weight_accepted() {
        let now = Utc::now();
        let fc = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "lst-x".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            0,
            now,
            now + Duration::hours(1),
            None,
            now,
        )
        .expect("zero ok");
        assert_eq!(fc.weight, 0);
    }

    // ── V003_03 invariant ─────────────────────────────────────────

    #[test]
    fn try_new_with_equal_start_end_errors() {
        let now = Utc::now();
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "lst-x".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now,
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::InvalidTimeBound);
    }

    #[test]
    fn try_new_with_end_before_start_errors() {
        let now = Utc::now();
        let err = FeaturedContent::try_new(
            FeaturedContentTargetKind::Listing,
            "lst-x".to_owned(),
            FeaturedContentFeatureKind::HomepageFeatured,
            1,
            now,
            now - Duration::seconds(1),
            None,
            now,
        )
        .unwrap_err();
        assert_eq!(err, FeaturedContentError::InvalidTimeBound);
    }

    // ── is_active_at ──────────────────────────────────────────────

    #[test]
    fn is_active_at_before_start_false() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(1));
        assert!(!fc.is_active_at(now - Duration::seconds(1)));
    }

    #[test]
    fn is_active_at_exact_start_true() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(1));
        assert!(fc.is_active_at(now));
    }

    #[test]
    fn is_active_at_mid_true() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(2));
        assert!(fc.is_active_at(now + Duration::hours(1)));
    }

    #[test]
    fn is_active_at_exact_end_false() {
        let now = Utc::now();
        let end = now + Duration::hours(1);
        let fc = make_fc(now, end);
        // half-open: ends_at 자체는 비활성.
        assert!(!fc.is_active_at(end));
    }

    #[test]
    fn is_active_at_after_end_false() {
        let now = Utc::now();
        let fc = make_fc(now, now + Duration::hours(1));
        assert!(!fc.is_active_at(now + Duration::hours(2)));
    }

    // ── record_impression / record_click ──────────────────────────

    #[test]
    fn record_impression_bumps_count() {
        let now = Utc::now();
        let mut fc = make_fc(now, now + Duration::hours(1));
        fc.record_impression();
        fc.record_impression();
        fc.record_impression();
        assert_eq!(fc.impression_count, 3);
        assert_eq!(fc.click_count, 0);
    }

    #[test]
    fn record_click_bumps_count() {
        let now = Utc::now();
        let mut fc = make_fc(now, now + Duration::hours(1));
        fc.record_click();
        fc.record_click();
        assert_eq!(fc.click_count, 2);
        assert_eq!(fc.impression_count, 0);
    }

    #[test]
    fn record_impression_saturates_at_max() {
        let now = Utc::now();
        let mut fc = make_fc(now, now + Duration::hours(1));
        fc.impression_count = i64::MAX - 1;
        fc.record_impression();
        assert_eq!(fc.impression_count, i64::MAX);
        fc.record_impression();
        assert_eq!(fc.impression_count, i64::MAX);
    }

    #[test]
    fn record_click_saturates_at_max() {
        let now = Utc::now();
        let mut fc = make_fc(now, now + Duration::hours(1));
        fc.click_count = i64::MAX;
        fc.record_click();
        assert_eq!(fc.click_count, i64::MAX);
    }

    // ── serde ─────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let now = Utc::now();
        let mut fc = make_fc(now, now + Duration::hours(1));
        fc.record_impression();
        fc.record_click();
        let json = serde_json::to_string(&fc).expect("serialize");
        let back: FeaturedContent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(fc, back);
    }

    #[test]
    fn serde_roundtrip_with_purchaser() {
        let now = Utc::now();
        let buyer = Id::<UserMarker>::new();
        let fc = FeaturedContent::try_new(
            FeaturedContentTargetKind::IndustrialComplex,
            "idc-001".to_owned(),
            FeaturedContentFeatureKind::SponsoredMarker,
            10,
            now,
            now + Duration::days(7),
            Some(buyer.clone()),
            now,
        )
        .expect("ok");
        let json = serde_json::to_string(&fc).expect("serialize");
        let back: FeaturedContent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.purchased_by, Some(buyer));
    }
}
