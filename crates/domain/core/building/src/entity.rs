//! `Building` Aggregate — 산업 부동산 SSS 단일 SSOT (panel 응답 + 지도 표시 둘 다 공급).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

use crate::purpose_code::BuildingPurposeCode;
use crate::structure_code::BuildingStructureCode;

/// `Building` Aggregate. *read-only*, mutation 메서드 없음.
///
/// SSOT — 한 PNU 당 한 또는 여러 Building. 패널 / 지도 / 검색 카드 / 관리자 view 등
/// **모든 consumer 가 본 entity 의 projection 을 공유**. 추가 view 가 필요하면 wire
/// shape (Gold) 를 별도 작성, 본 entity 는 *최대* 정보 유지.
///
/// # 필드 출처
///
/// - 핵심 식별 + 위치 + 용도 + 구조 + 면적 + 층 + 사용승인일: data.go.kr `getBrTitleInfo`
///   (`docs/data-sources/data-go-kr.md` 의 응답 필드 카탈로그)
/// - `geom` 폴리곤: V-World `LP_PA_CBND_BUBUN` (별도 호출 필요 — data.go.kr 에 없음)
///
/// # `geom: Option`
///
/// data.go.kr 응답에 폴리곤 없음. V-World 합성이 *옵션* 이라 `Option`. panel 응답
/// 처럼 폴리곤 불필요한 use case 는 `None` 으로 둠.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    // === 식별자 ===
    /// 필지 참조 (`PNU` 19자리).
    pub pnu: Pnu,
    /// 관리건축물대장 PK (`mgmBldrgstPk`, 정부 표준 `BigInt` → `String` 보존).
    pub mgm_bldrgst_pk: String,

    // === 위치 ===
    /// 대지위치 풀주소 (`platPlc`, 한글, 빈값 가능).
    pub plat_plc: Option<String>,
    /// 건물명 (`bldNm`, 빈값 가능).
    pub building_name: Option<String>,

    // === 용도 / 구조 ===
    /// 주용도 (`mainPurpsCd` + `mainPurpsCdNm` 하이브리드 매핑).
    pub main_purpose_code: BuildingPurposeCode,
    /// 구조 (`strctCd` + `strctCdNm` 하이브리드 매핑).
    pub structure_code: BuildingStructureCode,

    // === 면적 / 비율 (산업 매물 핵심) ===
    /// 대지면적 m² (`platArea`, 옵션 — 일부 응답 누락 가능).
    pub plat_area_m2: Option<AreaM2>,
    /// 건축면적 m² (`archArea`, 옵션).
    pub arch_area_m2: Option<AreaM2>,
    /// 건폐율 % (`bcRat`, 옵션).
    pub building_coverage_ratio: Option<f64>,
    /// 연면적 m² (`totArea`, 필수 — 모든 응답 보장).
    pub total_floor_area_m2: AreaM2,
    /// 용적률 % (`vlRat`, 옵션).
    pub floor_area_ratio: Option<f64>,

    // === 층수 / 높이 ===
    /// 지상층수 (`grndFlrCnt`).
    pub ground_floors: u8,
    /// 지하층수 (`ugrndFlrCnt`).
    pub underground_floors: u8,
    /// 높이 m (`heit`, 옵션).
    pub height_m: Option<f64>,

    // === 승강기 ===
    /// 승용 승강기수 (`rideUseElvtCnt`, 옵션).
    pub passenger_elevators: Option<u32>,
    /// 비상용 승강기수 (`emgenUseElvtCnt`, 옵션).
    pub emergency_elevators: Option<u32>,

    // === 주차장 ===
    /// 옥내 자주식 주차 대수 (`indrAutoUtcnt`, 옵션).
    pub indoor_self_parking: Option<u32>,
    /// 옥외 자주식 주차 대수 (`oudrAutoUtcnt`, 옵션).
    pub outdoor_self_parking: Option<u32>,

    // === 부속건축물 ===
    /// 부속건축물 수 (`atchBldCnt`, 옵션).
    pub annex_building_count: Option<u32>,
    /// 부속건축물 면적 m² (`atchBldArea`, 옵션).
    pub annex_building_area_m2: Option<AreaM2>,

    // === 날짜 ===
    /// 허가일 (`pmsDay`, 옵션).
    pub permit_date: Option<NaiveDate>,
    /// 착공일 (`stcnsDay`, 옵션).
    pub construction_start_date: Option<NaiveDate>,
    /// 사용승인일 (`useAprDay`, 옵션).
    pub use_approval_date: Option<NaiveDate>,

    // === 폴리곤 ===
    /// 건물 폴리곤 (WGS84) — V-World 합성 결과. 옵션 (panel-only path 는 None).
    pub geom: Option<PolygonSrid>,

    // === Lineage ===
    /// 외부 source 에서 fetch 한 시각 (캐시 만료 / Bronze pointer 일치 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;
