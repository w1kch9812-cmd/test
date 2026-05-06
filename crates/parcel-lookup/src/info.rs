//! [`ParcelInfo`] — 매물 denormalize 가 필요로 하는 필지 정보 subset.
//!
//! `Parcel` 전체 (지오메트리 + 면적 + 공시지가 + 주소 등) 가 아닌, listing 검색
//! 인덱싱에 쓰이는 필드만. 좁은 surface area 가 의도 — 호출자가 "왜 이게 필요한지"
//! 명확.

use parcel_domain::entity::GosiYearMonth;
use shared_kernel::admin_division::AdminDivision;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::zoning::Zoning;

/// 매물 denormalize 용 필지 정보.
///
/// V-World `LP_PA_CBND_BUBUN` 응답이 직접 채우는 필드 + `LT_C_UQ111` (선택,
/// 미구현 시 `zoning = None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParcelInfo {
    /// 행정구역 (시도 + 시군구 + 읍면동, cross-field invariant 검증됨).
    pub admin: AdminDivision,
    /// 지목 (대/전/답/공장용지/창고용지 등).
    pub land_use_type: LandUseType,
    /// 용도지역 (주거/상업/공업/녹지/기타). V-World `LP_PA_CBND_BUBUN` 미제공 →
    /// 별도 호출 (`LT_C_UQ111`) 또는 `None`.
    pub zoning: Option<Zoning>,
    /// 공시지가 (`KRW`/`m²`). 일부 필지는 미고시 → `None`.
    pub official_land_price_per_m2: Option<MoneyKrw>,
    /// 공시지가 고시 연·월 — `official_land_price_per_m2` lineage.
    pub gosi_year_month: Option<GosiYearMonth>,
}
