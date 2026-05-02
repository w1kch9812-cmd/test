//! `RealTransaction` Aggregate (R2 정적, Market BC).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;

use crate::transaction_kind::TransactionKind;

/// `RealTransaction` Aggregate. R2 정적 — *read-only*.
///
/// 한국 실거래가 공개 데이터 (`data.go.kr`)에서 `ETL`되어 `R2`에 보관해요.
/// 한 필지(`Pnu`)에 다수 거래가 가능해요 (시간 순).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealTransaction {
    /// 거래 식별자 (정부 표준).
    pub id: String,
    /// 거래 필지.
    pub pnu: Pnu,
    /// 거래 대상 건물 (있으면).
    pub building_id: Option<String>,
    /// 거래 유형 (매매/전세/월세).
    pub transaction_kind: TransactionKind,
    /// 거래 금액 (`KRW`).
    pub price_krw: MoneyKrw,
    /// 거래 면적 (`m²`).
    pub area_m2: AreaM2,
    /// 층 (음수 = 지하).
    pub floor: Option<i16>,
    /// 거래일.
    pub transaction_date: NaiveDate,
    /// `R2` fetch 시각.
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn real_transaction_constructs_from_r2_data() {
        let rt = RealTransaction {
            id: "RT_2024_001".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            building_id: Some("BLD_001".to_owned()),
            transaction_kind: TransactionKind::Sale,
            price_krw: MoneyKrw::try_new(500_000_000).unwrap(),
            area_m2: AreaM2::try_new(85.5).unwrap(),
            floor: Some(3),
            transaction_date: NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(),
            fetched_at: Utc::now(),
        };
        assert_eq!(rt.transaction_kind, TransactionKind::Sale);
        assert_eq!(rt.floor, Some(3));
    }

    #[test]
    fn real_transaction_underground_floor_negative() {
        let rt = RealTransaction {
            id: "RT_2024_002".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            building_id: None,
            transaction_kind: TransactionKind::MonthlyRent,
            price_krw: MoneyKrw::try_new(500_000).unwrap(),
            area_m2: AreaM2::try_new(50.0).unwrap(),
            floor: Some(-1),
            transaction_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            fetched_at: Utc::now(),
        };
        assert_eq!(rt.floor, Some(-1));
    }

    #[test]
    fn real_transaction_optional_fields_none() {
        let rt = RealTransaction {
            id: "RT_2024_003".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            building_id: None,
            transaction_kind: TransactionKind::Jeonse,
            price_krw: MoneyKrw::try_new(300_000_000).unwrap(),
            area_m2: AreaM2::try_new(60.0).unwrap(),
            floor: None,
            transaction_date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
            fetched_at: Utc::now(),
        };
        assert!(rt.building_id.is_none());
        assert!(rt.floor.is_none());
    }

    #[test]
    fn real_transaction_serde_roundtrip() {
        let rt = RealTransaction {
            id: "RT_2024_004".to_owned(),
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            building_id: None,
            transaction_kind: TransactionKind::Sale,
            price_krw: MoneyKrw::try_new(700_000_000).unwrap(),
            area_m2: AreaM2::try_new(120.0).unwrap(),
            floor: Some(5),
            transaction_date: NaiveDate::from_ymd_opt(2024, 8, 1).unwrap(),
            fetched_at: Utc::now(),
        };
        let json = serde_json::to_string(&rt).expect("serialize");
        let back: RealTransaction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt, back);
    }
}
