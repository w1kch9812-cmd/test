//! `ListingReportReason` — 매물 신고 사유 (6값).
//!
//! Spec § 5.5 `listing_report.reason` `CHECK` enum 6값:
//! `fake_listing`, `wrong_price`, `wrong_location`, `inappropriate_content`,
//! `spam`, `other`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// 매물 신고 사유 (6값, DB `varchar(50)` 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingReportReason {
    /// 허위 매물 (실제 존재하지 않거나 등록자가 권한 없는 매물).
    FakeListing,
    /// 가격 정보 오류 (실거래/현장 가격과 크게 차이).
    WrongPrice,
    /// 위치 정보 오류 (지도/주소 불일치).
    WrongLocation,
    /// 부적절한 컨텐츠 (욕설/혐오/타 매물 비방 등).
    InappropriateContent,
    /// 스팸 (반복 등록/광고성 게시물).
    Spam,
    /// 기타 (위 5개 분류에 해당하지 않음).
    Other,
}

impl ListingReportReason {
    /// DB CHECK 제약과 동일한 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::FakeListing => "fake_listing",
            Self::WrongPrice => "wrong_price",
            Self::WrongLocation => "wrong_location",
            Self::InappropriateContent => "inappropriate_content",
            Self::Spam => "spam",
            Self::Other => "other",
        }
    }

    /// DB 문자열을 enum 으로 파싱. 미지원 값이면 `None`.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "fake_listing" => Some(Self::FakeListing),
            "wrong_price" => Some(Self::WrongPrice),
            "wrong_location" => Some(Self::WrongLocation),
            "inappropriate_content" => Some(Self::InappropriateContent),
            "spam" => Some(Self::Spam),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

impl fmt::Display for ListingReportReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_str())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_db_str_matches_spec_for_each_variant() {
        assert_eq!(ListingReportReason::FakeListing.as_db_str(), "fake_listing");
        assert_eq!(ListingReportReason::WrongPrice.as_db_str(), "wrong_price");
        assert_eq!(
            ListingReportReason::WrongLocation.as_db_str(),
            "wrong_location"
        );
        assert_eq!(
            ListingReportReason::InappropriateContent.as_db_str(),
            "inappropriate_content"
        );
        assert_eq!(ListingReportReason::Spam.as_db_str(), "spam");
        assert_eq!(ListingReportReason::Other.as_db_str(), "other");
    }

    #[test]
    fn from_db_str_parses_each_variant() {
        assert_eq!(
            ListingReportReason::from_db_str("fake_listing"),
            Some(ListingReportReason::FakeListing)
        );
        assert_eq!(
            ListingReportReason::from_db_str("wrong_price"),
            Some(ListingReportReason::WrongPrice)
        );
        assert_eq!(
            ListingReportReason::from_db_str("wrong_location"),
            Some(ListingReportReason::WrongLocation)
        );
        assert_eq!(
            ListingReportReason::from_db_str("inappropriate_content"),
            Some(ListingReportReason::InappropriateContent)
        );
        assert_eq!(
            ListingReportReason::from_db_str("spam"),
            Some(ListingReportReason::Spam)
        );
        assert_eq!(
            ListingReportReason::from_db_str("other"),
            Some(ListingReportReason::Other)
        );
    }

    #[test]
    fn from_db_str_rejects_unknown() {
        assert_eq!(ListingReportReason::from_db_str("FakeListing"), None);
        assert_eq!(ListingReportReason::from_db_str(""), None);
        assert_eq!(ListingReportReason::from_db_str("scam"), None);
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            ListingReportReason::FakeListing,
            ListingReportReason::WrongPrice,
            ListingReportReason::WrongLocation,
            ListingReportReason::InappropriateContent,
            ListingReportReason::Spam,
            ListingReportReason::Other,
        ] {
            assert_eq!(ListingReportReason::from_db_str(v.as_db_str()), Some(v));
        }
    }

    #[test]
    fn display_matches_db_str() {
        assert_eq!(
            format!("{}", ListingReportReason::InappropriateContent),
            "inappropriate_content"
        );
        assert_eq!(format!("{}", ListingReportReason::Other), "other");
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = ListingReportReason::FakeListing;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""fake_listing""#);
        let back: ListingReportReason = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_roundtrip_all_6_variants() {
        for v in [
            ListingReportReason::FakeListing,
            ListingReportReason::WrongPrice,
            ListingReportReason::WrongLocation,
            ListingReportReason::InappropriateContent,
            ListingReportReason::Spam,
            ListingReportReason::Other,
        ] {
            let json = serde_json::to_string(&v).expect("serialize");
            let back: ListingReportReason = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, v);
        }
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = ListingReportReason::Spam;
        let b = a; // Copy
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(ListingReportReason::FakeListing);
        set.insert(ListingReportReason::Spam);
        set.insert(ListingReportReason::Spam); // dedup
        assert_eq!(set.len(), 2);
    }
}
