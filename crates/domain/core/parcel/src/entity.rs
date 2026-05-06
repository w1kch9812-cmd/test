//! `Parcel` Aggregate.
//!
//! 필드 도출의 SSOT는 V-World `LP_PA_CBND_BUBUN` (연속지적도) 응답 — 실제
//! 응답에 있는 필드만 비-Optional로 두고, 그 레이어가 제공하지 않는 면적/
//! 용도지역은 `Option`. 타 소스(별도 레이어, 별도 호출, `PostGIS` 계산)로
//! 채워질 수 있음을 타입으로 명시.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::address::{JibunAddress, RoadAddress};
use shared_kernel::admin_division::AdminDivision;
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::MultiPolygonSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::zoning::Zoning;

/// 공시지가 고시 연·월 — 가격(`official_land_price_per_m2`) 데이터 lineage.
///
/// V-World 응답의 `gosi_year` / `gosi_month` 매핑. 가격이 `Some`이면 본 필드도
/// `Some`이어야 함을 호출자(파서)가 보장 — 타입 수준 강제는 V2 후속.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GosiYearMonth {
    /// 고시 연도 (서기 4자리).
    pub year: u16,
    /// 고시 월 (1-12).
    pub month: u8,
}

/// `Parcel` Aggregate — *read-only* aggregate, mutation 메서드 없음.
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
    /// 지목 (대/전/답/임야/...) — V-World `jibun` 마지막 토큰에서 파싱.
    pub land_use_type: LandUseType,
    /// 면적 (`m²`) — V-World `LP_PA_CBND_BUBUN` 미제공. 별도 소스(PostGIS
    /// 계산 / 건축물대장 등)에서 채움. 미충족 시 `None` (정직).
    pub area: Option<AreaM2>,
    /// 공시지가 (`KRW`/`m²`) — V-World `jiga`. 일부 필지는 미고시.
    pub official_land_price_per_m2: Option<MoneyKrw>,
    /// 공시지가 고시 연·월 — `official_land_price_per_m2`의 lineage.
    pub gosi_year_month: Option<GosiYearMonth>,
    /// 용도지역 — V-World `LP_PA_CBND_BUBUN`엔 없음. `LT_C_UQ111` 별도
    /// spatial intersect 호출이 채움. 미호출/실패 시 `None`.
    pub zoning: Option<Zoning>,
    /// 필지 `MultiPolygon` (`WGS84`) — V-World 응답이 항상 `MultiPolygon`.
    pub geom: MultiPolygonSrid,
    /// 외부 소스에서 fetch한 시각 (캐시 만료 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;
