//! `GET /api/parcels/:pnu` — PNU 19 자리로 필지 정보 조회 (SP10 panel.parcel.summary 의 backing).
//!
//! `parcel-lookup` crate 의 [`ParcelInfoLookup::lookup_by_pnu`] 호출 → Platform Core 또는 `NoOp` 응답.
//! 본 핸들러는 "panel" 단어를 모름 — pure REST resource (spec § 7 F1).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use parcel_lookup::ParcelInfoLookup;
use serde::Serialize;
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// `/api/parcels` 핸들러 공유 상태.
#[derive(Clone)]
pub struct ParcelsState {
    /// PNU → [`parcel_lookup::ParcelInfo`] lookup port.
    pub parcel_lookup: Arc<dyn ParcelInfoLookup>,
}

/// 필지 정보 응답. ADR 0018 PNU-First denormalize 와 동일 surface.
#[derive(Debug, Serialize)]
pub struct ParcelInfoResponse {
    /// 입력 PNU (19 자리).
    pub pnu: String,
    /// 행정구역 시도 코드 (2자리).
    pub sido_code: String,
    /// 행정구역 시군구 코드 (5자리, prefix 포함).
    pub sigungu_code: String,
    /// 행정구역 읍면동 코드 (8자리, prefix 포함).
    pub eupmyeondong_code: String,
    /// 시도 한국어명. `ParcelInfo` 미보유 — 현재 빈 문자열 (frontend 가 코드→명 매핑).
    pub sido_name: String,
    /// 시군구 한국어명. `ParcelInfo` 미보유 — 현재 빈 문자열.
    pub sigungu_name: String,
    /// 읍면동 한국어명. `ParcelInfo` 미보유 — 현재 빈 문자열.
    pub eupmyeondong_name: String,
    /// 지목 (`factory_site` / `warehouse_site` / ...).
    pub land_use_type: String,
    /// 용도지역 (`residential` / `commercial` / ...). Platform Core 미제공 시 `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoning: Option<String>,
    /// 공시지가 (KRW/m²). 미고시 → `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub official_land_price_per_m2: Option<i64>,
    /// 공시지가 고시 연·월 (예: `"202504"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gosi_year_month: Option<String>,
}

/// `GET /api/parcels/:pnu` — 인증 필수.
///
/// # Errors
///
/// - PNU 형식 오류 → `400 invalid-pnu`
/// - lookup 백엔드 실패 → `502 parcel-lookup-failed`
/// - 미발견 → `404 parcel-not-found`
pub async fn get_parcel(
    State(state): State<ParcelsState>,
    _auth: AuthenticatedUser,
    Path(pnu_raw): Path<String>,
) -> Result<Json<ParcelInfoResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(&pnu_raw).map_err(|e| {
        problem(
            "invalid-pnu",
            "잘못된 필지 PNU 에요",
            StatusCode::BAD_REQUEST,
            Some(format!("{e}")),
        )
    })?;

    let info = state
        .parcel_lookup
        .lookup_by_pnu(&pnu)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, pnu = %pnu_raw, "parcel_lookup failed");
            problem(
                "parcel-lookup-failed",
                "필지 정보를 불러오지 못했어요. 잠시 후 다시 시도해 주세요",
                StatusCode::BAD_GATEWAY,
                None,
            )
        })?
        .ok_or_else(|| {
            problem(
                "parcel-not-found",
                "해당 필지를 찾지 못했어요",
                StatusCode::NOT_FOUND,
                Some(format!("pnu={pnu_raw}")),
            )
        })?;

    Ok(Json(ParcelInfoResponse {
        pnu: pnu_raw,
        sido_code: info.admin.sido.as_str().to_owned(),
        sigungu_code: info.admin.sigungu.as_str().to_owned(),
        eupmyeondong_code: info.admin.eupmyeondong.as_str().to_owned(),
        // `AdminDivision` 은 코드만 carry — 한국어명은 frontend 가 코드→명 매핑 (`shared_kernel`
        // 미보유). 여기선 빈 문자열로 응답 shape 만 유지 (FU: 별도 lookup table).
        sido_name: String::new(),
        sigungu_name: String::new(),
        eupmyeondong_name: String::new(),
        land_use_type: info.land_use_type.as_str().to_owned(),
        zoning: info.zoning.map(|z| z.as_str().to_owned()),
        official_land_price_per_m2: info
            .official_land_price_per_m2
            .map(shared_kernel::money::MoneyKrw::as_i64),
        gosi_year_month: info
            .gosi_year_month
            .map(|y| format!("{:04}{:02}", y.year, y.month)),
    }))
}
