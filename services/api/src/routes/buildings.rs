//! `GET /api/buildings?parcel_pnu=:pnu` building list route.
//!
//! Gongzzang owns the B2C route contract and user-facing response shape.
//! Canonical catalog building data is read through Platform Core.

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// Reader communication or parsing error mapped to a `502` route response.
pub type BuildingRegisterError = Box<dyn std::error::Error + Send + Sync>;

/// Building reader port returning route-facing records.
///
/// Implementations:
/// - production: `services/api/src/building_reader.rs::PlatformCoreBuildingRegisterReader`
/// - dev fallback: `startup.rs::NoOpBuildingRegisterReader`
pub trait BuildingRegisterReader: Send + Sync {
    /// Lists buildings for a PNU.
    ///
    /// # Errors
    ///
    /// Returns a reader error when the backing Platform Core call or response
    /// translation fails.
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<BuildingRegisterRecord>, BuildingRegisterError>,
                > + Send
                + 'a,
        >,
    >;
}

/// Shared state for `/api/buildings`.
#[derive(Clone)]
pub struct BuildingsState {
    /// Building reader port.
    pub reader: Arc<dyn BuildingRegisterReader>,
}

/// Query parameters for `GET /api/buildings`.
#[derive(Debug, Deserialize)]
pub struct BuildingsQuery {
    /// Parcel PNU, 19 digits.
    pub parcel_pnu: String,
}

/// Route-facing building register record.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingRegisterRecord {
    /// Building identifier.
    pub id: String,
    /// Building name.
    pub name: String,
    /// Address text.
    pub address: Option<String>,
    /// Purpose label or code.
    pub purpose: String,
    /// Structure label or code.
    pub structure: String,
    /// Plot area in square meters.
    pub plot_area_m2: Option<f64>,
    /// Building area in square meters.
    pub building_area_m2: Option<f64>,
    /// Building coverage ratio, percent.
    pub building_coverage_ratio: Option<f64>,
    /// Total floor area in square meters.
    pub total_area_m2: f64,
    /// Floor area ratio, percent.
    pub floor_area_ratio: Option<f64>,
    /// Above-ground floor count.
    pub above_ground_floors: u8,
    /// Underground floor count.
    pub below_ground_floors: u8,
    /// Building height in meters.
    pub height_m: Option<f64>,
    /// Passenger elevator count.
    pub passenger_elevators: Option<u32>,
    /// Emergency elevator count.
    pub emergency_elevators: Option<u32>,
    /// Indoor self-parking count.
    pub indoor_self_parking: Option<u32>,
    /// Outdoor self-parking count.
    pub outdoor_self_parking: Option<u32>,
    /// Annex building count.
    pub annex_building_count: Option<u32>,
    /// Annex building area in square meters.
    pub annex_building_area_m2: Option<f64>,
    /// Permit date, `YYYY-MM-DD`.
    pub permitted_at: Option<String>,
    /// Construction start date, `YYYY-MM-DD`.
    pub started_at: Option<String>,
    /// Use approval date, `YYYY-MM-DD`.
    pub approved_at: Option<String>,
}

/// HTTP response shape for one building.
#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    /// Building identifier from Platform Core.
    pub id: String,
    /// Building name, empty when Platform Core has no route-facing name.
    pub name: String,
    /// Address text when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// Purpose label or code.
    pub purpose: String,
    /// Structure label or code.
    pub structure: String,

    /// Plot area in square meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_area_m2: Option<f64>,
    /// Building area in square meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_area_m2: Option<f64>,
    /// Building coverage ratio, percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_coverage_ratio: Option<f64>,
    /// Total floor area in square meters.
    pub total_area_m2: f64,
    /// Floor area ratio, percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_area_ratio: Option<f64>,

    /// Above-ground floor count.
    pub above_ground_floors: u8,
    /// Underground floor count.
    pub below_ground_floors: u8,
    /// Building height in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_m: Option<f64>,

    /// Passenger elevator count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passenger_elevators: Option<u32>,
    /// Emergency elevator count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_elevators: Option<u32>,

    /// Indoor self-parking count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indoor_self_parking: Option<u32>,
    /// Outdoor self-parking count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdoor_self_parking: Option<u32>,

    /// Annex building count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_count: Option<u32>,
    /// Annex building area in square meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annex_building_area_m2: Option<f64>,

    /// Permit date, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permitted_at: Option<String>,
    /// Construction start date, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// Use approval date, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

impl From<BuildingRegisterRecord> for BuildingResponse {
    fn from(b: BuildingRegisterRecord) -> Self {
        Self {
            id: b.id,
            name: b.name,
            address: b.address,
            purpose: b.purpose,
            structure: b.structure,
            plot_area_m2: b.plot_area_m2,
            building_area_m2: b.building_area_m2,
            building_coverage_ratio: b.building_coverage_ratio,
            total_area_m2: b.total_area_m2,
            floor_area_ratio: b.floor_area_ratio,
            above_ground_floors: b.above_ground_floors,
            below_ground_floors: b.below_ground_floors,
            height_m: b.height_m,
            passenger_elevators: b.passenger_elevators,
            emergency_elevators: b.emergency_elevators,
            indoor_self_parking: b.indoor_self_parking,
            outdoor_self_parking: b.outdoor_self_parking,
            annex_building_count: b.annex_building_count,
            annex_building_area_m2: b.annex_building_area_m2,
            permitted_at: b.permitted_at,
            started_at: b.started_at,
            approved_at: b.approved_at,
        }
    }
}

/// Building list response.
#[derive(Debug, Serialize)]
pub struct BuildingsResponse {
    /// Building list. Empty when none are available.
    pub buildings: Vec<BuildingResponse>,
}

/// Handles `GET /api/buildings?parcel_pnu=...`.
///
/// # Errors
///
/// - `400 invalid-pnu` when the PNU is malformed.
/// - `502 buildings-lookup-failed` when Platform Core lookup fails.
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
