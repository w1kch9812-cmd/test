//! `Building` Aggregate (`R2` 정적, 11 필드).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

use crate::purpose_code::BuildingPurposeCode;
use crate::structure_code::BuildingStructureCode;

/// `Building` Aggregate. `R2` 정적 — *read-only*, mutation 메서드 없음.
///
/// 한 필지(`Pnu`)에 여러 건물 가능해요. 식별은 `R2` 객체 키로.
/// `height_m` 의 finiteness/positiveness 같은 invariant는 Reader 구현 시점
/// (sub-project 4) 에서 체크 — Aggregate 자체는 R2 데이터를 그대로 표현해요.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    /// 필지 참조 (`PNU` 19자리).
    pub pnu: Pnu,
    /// 건물명 (≤200자, 선택 — 무명 건물 가능).
    pub building_name: Option<String>,
    /// 주용도.
    pub main_purpose_code: BuildingPurposeCode,
    /// 구조.
    pub structure_code: BuildingStructureCode,
    /// 연면적 (`m²`).
    pub total_floor_area_m2: AreaM2,
    /// 지상층수.
    pub ground_floors: u8,
    /// 지하층수.
    pub underground_floors: u8,
    /// 높이 (`m`, 선택).
    pub height_m: Option<f64>,
    /// 사용승인일 (선택).
    pub use_approval_date: Option<NaiveDate>,
    /// 건물 폴리곤 (`WGS84`).
    pub geom: PolygonSrid,
    /// `R2` 객체에서 fetch한 시각 (캐시 만료 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;
