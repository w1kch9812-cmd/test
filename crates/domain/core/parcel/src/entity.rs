//! `Parcel` Aggregate (spec § 8.4, 10 필드).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::address::{JibunAddress, RoadAddress};
use shared_kernel::admin_division::AdminDivision;
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::zoning::Zoning;

/// `Parcel` Aggregate. `R2` 정적 — *read-only*, mutation 메서드 없음.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parcel {
    /// 필지 식별자 (`PNU` 19자리).
    pub pnu: Pnu,
    /// 행정구역 (시도 + 시군구 + 읍면동).
    pub admin: AdminDivision,
    /// 도로명 주소 (선택 — 도로명 미지정 필지 가능).
    pub road_address: Option<RoadAddress>,
    /// 지번 주소 (필수 — 모든 필지는 지번 보유).
    pub jibun_address: JibunAddress,
    /// 지목 (대/전/답/임야/...).
    pub land_use_type: LandUseType,
    /// 면적 (`m²`).
    pub area: AreaM2,
    /// 공시지가 (`KRW`/`m²`, 선택).
    pub official_land_price_per_m2: Option<MoneyKrw>,
    /// 용도지역 (주거/상업/공업/녹지/기타).
    pub zoning: Zoning,
    /// 필지 폴리곤 (`WGS84`).
    pub geom: PolygonSrid,
    /// `R2` 객체에서 fetch한 시각 (캐시 만료 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;
