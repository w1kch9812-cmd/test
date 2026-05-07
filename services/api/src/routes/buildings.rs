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

/// 좁은 building 결과 (data.go.kr `getBrTitleInfo` 의 표제부 일부).
#[derive(Debug, Clone)]
pub struct BuildingItem {
    /// 관리건축물대장 PK (`mgmBldrgstPk`).
    pub mgm_bldrgst_pk: String,
    /// 건물명 (`bldNm`).
    pub bldg_nm: String,
    /// 주용도코드명 (예: `"공장"`, `"창고시설"`).
    pub main_purps_cd_nm: String,
    /// 연면적 (`m²`).
    pub tot_area: f64,
    /// 사용승인일 (`YYYYMMDD`).
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

/// 건축물 응답 단건.
#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    /// 관리건축물대장 PK.
    pub id: String,
    /// 건물명.
    pub name: String,
    /// 주용도.
    pub purpose: String,
    /// 연면적 (`m²`).
    pub total_area_m2: f64,
    /// 사용승인일 (`YYYYMMDD`).
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
                purpose: b.main_purps_cd_nm,
                total_area_m2: b.tot_area,
                approved_at: b.use_apr_day,
            })
            .collect(),
    }))
}
