//! `CourtAuction` Aggregate (R2 정적, Market BC).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::geometry::PointSrid;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;

use crate::auction_kind::CourtAuctionKind;
use crate::auction_status::CourtAuctionStatus;

/// `CourtAuction` Aggregate. R2 정적 — *read-only*.
///
/// 한국 법원 경매 공개 데이터를 `ETL`해 `R2`에 보관해요. 활성 + 이력 모두 포함.
/// 한 필지(`Pnu`)에 다수 사건이 가능해요.
///
/// `geom`은 `PointSrid` 안에 `f64` (lng/lat)이 있어 `Eq`는 만족하지 못해요 —
/// `PartialEq`만 derive해요.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CourtAuction {
    /// 사건번호 (예: `"2024타경12345"`).
    pub case_number: String,
    /// 대상 필지.
    pub pnu: Pnu,
    /// 경매 유형 (강제/임의/기타).
    pub kind: CourtAuctionKind,
    /// 진행 상태 (예정/진행중/낙찰/취하/유찰).
    pub status: CourtAuctionStatus,
    /// 감정가 (`KRW`).
    pub appraisal_value: MoneyKrw,
    /// 최저입찰가 (`KRW`).
    pub minimum_bid: MoneyKrw,
    /// 유찰 횟수.
    pub bid_count: u8,
    /// 매각기일.
    pub auction_date: Option<NaiveDate>,
    /// 낙찰가 (`Sold`일 때만 `Some`).
    pub sold_price: Option<MoneyKrw>,
    /// 낙찰일 (`Sold`일 때만 `Some`).
    pub sold_at: Option<NaiveDate>,
    /// 위치 (있으면).
    pub geom: Option<PointSrid>,
    /// `R2` fetch 시각.
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn court_auction_constructs_full_happy_path() {
        let ca = CourtAuction {
            case_number: "2024타경12345".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            kind: CourtAuctionKind::Forced,
            status: CourtAuctionStatus::InProgress,
            appraisal_value: MoneyKrw::try_new(800_000_000).unwrap(),
            minimum_bid: MoneyKrw::try_new(560_000_000).unwrap(),
            bid_count: 2,
            auction_date: NaiveDate::from_ymd_opt(2024, 9, 15),
            sold_price: None,
            sold_at: None,
            geom: Some(PointSrid::try_new_wgs84(126.9784, 37.5666).unwrap()),
            fetched_at: Utc::now(),
        };
        assert_eq!(ca.case_number, "2024타경12345");
        assert_eq!(ca.kind, CourtAuctionKind::Forced);
        assert_eq!(ca.status, CourtAuctionStatus::InProgress);
        assert_eq!(ca.bid_count, 2);
        assert!(ca.geom.is_some());
    }

    #[test]
    fn court_auction_optional_fields_none() {
        let ca = CourtAuction {
            case_number: "2024타경00001".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            kind: CourtAuctionKind::Voluntary,
            status: CourtAuctionStatus::Upcoming,
            appraisal_value: MoneyKrw::try_new(300_000_000).unwrap(),
            minimum_bid: MoneyKrw::try_new(300_000_000).unwrap(),
            bid_count: 0,
            auction_date: None,
            sold_price: None,
            sold_at: None,
            geom: None,
            fetched_at: Utc::now(),
        };
        assert!(ca.auction_date.is_none());
        assert!(ca.sold_price.is_none());
        assert!(ca.sold_at.is_none());
        assert!(ca.geom.is_none());
        assert_eq!(ca.bid_count, 0);
    }

    #[test]
    fn court_auction_sold_with_price_and_date() {
        let ca = CourtAuction {
            case_number: "2024타경99999".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            kind: CourtAuctionKind::Forced,
            status: CourtAuctionStatus::Sold,
            appraisal_value: MoneyKrw::try_new(1_000_000_000).unwrap(),
            minimum_bid: MoneyKrw::try_new(700_000_000).unwrap(),
            bid_count: 3,
            auction_date: NaiveDate::from_ymd_opt(2024, 10, 20),
            sold_price: Some(MoneyKrw::try_new(820_000_000).unwrap()),
            sold_at: NaiveDate::from_ymd_opt(2024, 10, 20),
            geom: None,
            fetched_at: Utc::now(),
        };
        assert_eq!(ca.status, CourtAuctionStatus::Sold);
        assert_eq!(ca.sold_price, Some(MoneyKrw::try_new(820_000_000).unwrap()));
        assert_eq!(ca.sold_at, NaiveDate::from_ymd_opt(2024, 10, 20));
    }

    #[test]
    fn court_auction_serde_roundtrip() {
        let ca = CourtAuction {
            case_number: "2024타경55555".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            kind: CourtAuctionKind::Other,
            status: CourtAuctionStatus::Failed,
            appraisal_value: MoneyKrw::try_new(450_000_000).unwrap(),
            minimum_bid: MoneyKrw::try_new(360_000_000).unwrap(),
            bid_count: 1,
            auction_date: NaiveDate::from_ymd_opt(2024, 11, 5),
            sold_price: None,
            sold_at: None,
            geom: Some(PointSrid::try_new_wgs84(127.0, 37.5).unwrap()),
            fetched_at: Utc::now(),
        };
        let json = serde_json::to_string(&ca).expect("serialize");
        let back: CourtAuction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ca, back);
    }
}
