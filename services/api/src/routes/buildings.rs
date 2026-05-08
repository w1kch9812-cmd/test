//! `GET /api/buildings?parcel_pnu=:pnu` — 필지 위 건축물 list.
//!
//! data.go.kr `getBrTitleInfo` 위 thin REST shell. 본 모듈은 `panel` 단어를 모름 —
//! pure REST resource (spec § 7 F1).
//!
//! 주의: 본 파일이 정의한 [`BuildingRegisterReader`] trait 은 **api 로컬** 정의예요.
//! 향후 SP4-iii-a 의 `data-go-kr::DataGoKrBuildingReader` (또는 `building_domain::BuildingReader`)
//! 와 합칠 가능성 — 그땐 본 trait/`BuildingItem` 을 제거하고 그 crate 를 import.
//! 현재는 api 의존성을 늘리지 않기 위해 좁은 surface 로 격리.

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// PNU 로 건축물 list 를 조회하는 좁은 port.
///
/// SP4-iii-a 의 full `BuildingReader` 와 다름 — 본 trait 은 `api` 가 panel 응답으로 노출하는
/// subset 만 표현. live impl 은 `main.rs` 에서 주입.
pub type BuildingRegisterError = Box<dyn std::error::Error + Send + Sync>;

/// 건축물대장 reader port (api 로컬 정의 — 위 NOTE 참조).
pub trait BuildingRegisterReader: Send + Sync {
    /// PNU 한 건의 건축물 list 조회. 빈 vec 가능.
    ///
    /// # Errors
    ///
    /// 백엔드 통신/파싱 실패는 `BuildingRegisterError` 로 (라우트가 502 매핑).
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<BuildingItem>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    >;
}

/// 산업 부동산 매물용 building 결과 (data.go.kr `getBrTitleInfo` 표제부 핵심 필드).
///
/// 전체 80+ 필드 중 *산업 매물 검토에 필요한 21개* 만 추출. 나머지 필드는
/// `parcel_external_data.raw_response JSONB` (Bronze) 에 보존되어 미래 SQL 로
/// 재추출 가능. 카탈로그: [docs/data-sources/data-go-kr.md](../../../../docs/data-sources/data-go-kr.md).
#[derive(Debug, Clone)]
pub struct BuildingItem {
    // === 식별자 / 위치 ===
    /// 관리건축물대장 PK (`mgmBldrgstPk`, 실 응답 number → String).
    pub mgm_bldrgst_pk: String,
    /// 건물명 (`bldNm`).
    pub bldg_nm: String,
    /// 대지위치 풀주소 (`platPlc`).
    pub plat_plc: Option<String>,

    // === 용도 / 구조 ===
    /// 주용도코드명 (`mainPurpsCdNm`, 예: `"공장"` / `"창고시설"`).
    pub main_purps_cd_nm: String,
    /// 구조코드명 (`strctCdNm`, 예: `"철근콘크리트구조"` / `"철골구조"`).
    pub strct_cd_nm: Option<String>,

    // === 면적 / 비율 (산업 매물 핵심) ===
    /// 대지면적 m² (`platArea`).
    pub plat_area: Option<f64>,
    /// 건축면적 m² (`archArea`).
    pub arch_area: Option<f64>,
    /// 건폐율 % (`bcRat`).
    pub bc_rat: Option<f64>,
    /// 연면적 m² (`totArea`).
    pub tot_area: f64,
    /// 용적률 % (`vlRat`).
    pub vl_rat: Option<f64>,

    // === 층수 / 높이 ===
    /// 지상층수 (`grndFlrCnt`).
    pub grnd_flr_cnt: Option<u32>,
    /// 지하층수 (`ugrndFlrCnt`).
    pub ugrnd_flr_cnt: Option<u32>,
    /// 건물 높이 m (`heit`).
    pub heit: Option<f64>,

    // === 승강기 ===
    /// 승용 승강기수 (`rideUseElvtCnt`).
    pub ride_use_elvt_cnt: Option<u32>,
    /// 비상용 승강기수 (`emgenUseElvtCnt`).
    pub emgen_use_elvt_cnt: Option<u32>,

    // === 주차장 ===
    /// 옥내 자주식 주차 대수 (`indrAutoUtcnt`).
    pub indr_auto_utcnt: Option<u32>,
    /// 옥외 자주식 주차 대수 (`oudrAutoUtcnt`).
    pub oudr_auto_utcnt: Option<u32>,

    // === 부속건축물 ===
    /// 부속건축물수 (`atchBldCnt`).
    pub atch_bld_cnt: Option<u32>,
    /// 부속건축물 면적 m² (`atchBldArea`).
    pub atch_bld_area: Option<f64>,

    // === 날짜 (YYYYMMDD) ===
    /// 허가일 (`pmsDay`, 8자리).
    pub pms_day: Option<String>,
    /// 착공일 (`stcnsDay`, 8자리).
    pub stcns_day: Option<String>,
    /// 사용승인일 (`useAprDay`, 8자리).
    pub use_apr_day: Option<String>,
}

/// `/api/buildings` 핸들러 공유 상태.
#[derive(Clone)]
pub struct BuildingsState {
    /// 건축물대장 reader port.
    pub reader: Arc<dyn BuildingRegisterReader>,
}

/// `GET /api/buildings` 쿼리 파라미터.
#[derive(Debug, Deserialize)]
pub struct BuildingsQuery {
    /// 필지 PNU (19 자리).
    pub parcel_pnu: String,
}

/// 건축물 응답 단건. `BuildingItem` 의 wire shape (`snake_case` 도메인 ↔ `camelCase` API).
#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    // === 식별자 / 위치 ===
    /// 관리건축물대장 PK.
    pub id: String,
    /// 건물명.
    pub name: String,
    /// 대지위치 풀주소.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    // === 용도 / 구조 ===
    /// 주용도.
    pub purpose: String,
    /// 구조 (예: `"철근콘크리트구조"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure: Option<String>,

    // === 면적 / 비율 ===
    /// 대지면적 m².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_area_m2: Option<f64>,
    /// 건축면적 m².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_area_m2: Option<f64>,
    /// 건폐율 %.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_coverage_ratio: Option<f64>,
    /// 연면적 m².
    pub total_area_m2: f64,
    /// 용적률 %.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_area_ratio: Option<f64>,

    // === 층수 / 높이 ===
    /// 지상층수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub above_ground_floors: Option<u32>,
    /// 지하층수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub below_ground_floors: Option<u32>,
    /// 건물 높이 m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_m: Option<f64>,

    // === 승강기 ===
    /// 승용 승강기수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passenger_elevators: Option<u32>,
    /// 비상용 승강기수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_elevators: Option<u32>,

    // === 주차장 ===
    /// 옥내 자주식 주차 대수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indoor_self_parking: Option<u32>,
    /// 옥외 자주식 주차 대수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdoor_self_parking: Option<u32>,

    // === 부속건축물 ===
    /// 부속건축물 수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_count: Option<u32>,
    /// 부속건축물 면적 m².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_area_m2: Option<f64>,

    // === 날짜 (YYYYMMDD) ===
    /// 허가일.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permitted_at: Option<String>,
    /// 착공일.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// 사용승인일.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

/// 건축물 list 응답.
#[derive(Debug, Serialize)]
pub struct BuildingsResponse {
    /// 건축물 list. 미발견 시 빈 vec.
    pub buildings: Vec<BuildingResponse>,
}

/// `GET /api/buildings?parcel_pnu=...` — 인증 필수.
///
/// # Errors
///
/// - PNU 형식 오류 → `400 invalid-pnu`
/// - reader 실패 → `502 buildings-lookup-failed`
pub async fn list_buildings(
    State(state): State<BuildingsState>,
    _auth: AuthenticatedUser,
    Query(q): Query<BuildingsQuery>,
) -> Result<Json<BuildingsResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(&q.parcel_pnu).map_err(|e| {
        problem(
            "invalid-pnu",
            "잘못된 필지 PNU 에요",
            StatusCode::BAD_REQUEST,
            Some(format!("{e}")),
        )
    })?;

    let items = state.reader.list_by_pnu(&pnu).await.map_err(|e| {
        tracing::warn!(error = %e, pnu = %q.parcel_pnu, "building_register read failed");
        problem(
            "buildings-lookup-failed",
            "건축물 정보를 불러오지 못했어요",
            StatusCode::BAD_GATEWAY,
            None,
        )
    })?;

    Ok(Json(BuildingsResponse {
        buildings: items
            .into_iter()
            .map(|b| BuildingResponse {
                id: b.mgm_bldrgst_pk,
                name: b.bldg_nm,
                address: b.plat_plc,
                purpose: b.main_purps_cd_nm,
                structure: b.strct_cd_nm,
                plot_area_m2: b.plat_area,
                building_area_m2: b.arch_area,
                building_coverage_ratio: b.bc_rat,
                total_area_m2: b.tot_area,
                floor_area_ratio: b.vl_rat,
                above_ground_floors: b.grnd_flr_cnt,
                below_ground_floors: b.ugrnd_flr_cnt,
                height_m: b.heit,
                passenger_elevators: b.ride_use_elvt_cnt,
                emergency_elevators: b.emgen_use_elvt_cnt,
                indoor_self_parking: b.indr_auto_utcnt,
                outdoor_self_parking: b.oudr_auto_utcnt,
                annex_building_count: b.atch_bld_cnt,
                annex_building_area_m2: b.atch_bld_area,
                permitted_at: b.pms_day,
                started_at: b.stcns_day,
                approved_at: b.use_apr_day,
            })
            .collect(),
    }))
}
