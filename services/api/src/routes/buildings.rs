//! `GET /api/buildings?parcel_pnu=:pnu` — 필지 위 건축물 list.
//!
//! data.go.kr `getBrTitleInfo` 위 thin REST shell. 본 모듈은 `panel` 단어를 모름 —
//! pure REST resource (spec § 7 F1).
//!
//! # SSOT (2026-05-08 unification)
//!
//! 이전엔 본 파일이 `BuildingItem` (panel-only, 21 필드) 를 별도 정의해 SSOT 위반.
//! `building_domain::Building` (rich, 11 필드) 와 두 모델 공존 → Codex round 7 verdict
//! 의 "장기 SSOT debt".
//!
//! 본 commit 으로 통합:
//! - Silver = `building_domain::Building` (단일 SSOT, 21+ 필드, geom 옵션)
//! - Gold = 본 파일의 `BuildingResponse` (HTTP wire shape, `snake_case` 도메인 → JSON)
//! - reader trait 은 `Vec<Building>` 반환 (panel 전용 `subset` 제거)

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use building_domain::entity::Building;
use serde::{Deserialize, Serialize};
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// reader 통신 / 파싱 실패 - 라우트가 502 매핑.
pub type BuildingRegisterError = Box<dyn std::error::Error + Send + Sync>;

/// 건축물대장 reader port — `building_domain::Building` 의 list 반환.
///
/// 구현체:
/// - production: `services/api/src/building_reader.rs::DataGoKrBuildingRegisterReader`
/// - dev fallback: `main.rs` 의 `NoOpBuildingRegisterReader` (빈 vec)
pub trait BuildingRegisterReader: Send + Sync {
    /// PNU 한 건의 건축물 list 조회. 빈 vec 가능.
    ///
    /// # Errors
    /// 백엔드 통신 / 파싱 실패는 `BuildingRegisterError` 로 (라우트가 502 매핑).
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<Building>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    >;
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

/// 건축물 응답 단건 — `Building` Silver 의 wire shape (Gold).
///
/// 도메인 필드명 (`snake_case`) → JSON 필드명 (의미 노출). HTTP 계약은 본 struct 가 SSOT.
/// 향후 `utoipa` annotation 추가 → `OpenAPI` → TS 자동 생성 (AGENTS.md § 10.3 Type Safety).
#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    /// 관리건축물대장 PK.
    pub id: String,
    /// 건물명 (없으면 빈 문자열).
    pub name: String,
    /// 대지위치 풀주소.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// 주용도 (도메인 enum 의 사용자 노출 라벨).
    pub purpose: String,
    /// 구조 (도메인 enum 의 사용자 노출 라벨).
    pub structure: String,

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

    /// 지상층수.
    pub above_ground_floors: u8,
    /// 지하층수.
    pub below_ground_floors: u8,
    /// 건물 높이 m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_m: Option<f64>,

    /// 승용 승강기수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passenger_elevators: Option<u32>,
    /// 비상용 승강기수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_elevators: Option<u32>,

    /// 옥내 자주식 주차 대수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indoor_self_parking: Option<u32>,
    /// 옥외 자주식 주차 대수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdoor_self_parking: Option<u32>,

    /// 부속건축물 수.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_count: Option<u32>,
    /// 부속건축물 면적 m².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_area_m2: Option<f64>,

    /// 허가일 (`YYYY-MM-DD`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permitted_at: Option<String>,
    /// 착공일 (`YYYY-MM-DD`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// 사용승인일 (`YYYY-MM-DD`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

impl From<Building> for BuildingResponse {
    fn from(b: Building) -> Self {
        Self {
            id: b.mgm_bldrgst_pk,
            name: b.building_name.unwrap_or_default(),
            address: b.plat_plc,
            purpose: format!("{:?}", b.main_purpose_code),
            structure: format!("{:?}", b.structure_code),
            plot_area_m2: b.plat_area_m2.map(shared_kernel::area::AreaM2::as_f64),
            building_area_m2: b.arch_area_m2.map(shared_kernel::area::AreaM2::as_f64),
            building_coverage_ratio: b.building_coverage_ratio,
            total_area_m2: b.total_floor_area_m2.as_f64(),
            floor_area_ratio: b.floor_area_ratio,
            above_ground_floors: b.ground_floors,
            below_ground_floors: b.underground_floors,
            height_m: b.height_m,
            passenger_elevators: b.passenger_elevators,
            emergency_elevators: b.emergency_elevators,
            indoor_self_parking: b.indoor_self_parking,
            outdoor_self_parking: b.outdoor_self_parking,
            annex_building_count: b.annex_building_count,
            annex_building_area_m2: b.annex_building_area_m2.map(shared_kernel::area::AreaM2::as_f64),
            permitted_at: b.permit_date.map(|d| d.format("%Y-%m-%d").to_string()),
            started_at: b
                .construction_start_date
                .map(|d| d.format("%Y-%m-%d").to_string()),
            approved_at: b
                .use_approval_date
                .map(|d| d.format("%Y-%m-%d").to_string()),
        }
    }
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

    let buildings = state.reader.list_by_pnu(&pnu).await.map_err(|e| {
        tracing::warn!(error = %e, pnu = %q.parcel_pnu, "building_register read failed");
        problem(
            "buildings-lookup-failed",
            "건축물 정보를 불러오지 못했어요",
            StatusCode::BAD_GATEWAY,
            None,
        )
    })?;

    Ok(Json(BuildingsResponse {
        buildings: buildings.into_iter().map(BuildingResponse::from).collect(),
    }))
}
