## Task 3: Backend REST endpoints (`/api/parcels/:pnu`, `/api/buildings`)

**목표:** Spec § 7 F1-pure REST. backend 는 "panel" 단어 모름 — 그냥 resource server.

**Files:**
- Create: `services/api/src/routes/parcels.rs`
- Create: `services/api/src/routes/buildings.rs`
- Modify: `services/api/src/main.rs` (router 조립)

### Step 3.1: `GET /api/parcels/:pnu`

- [ ] **Step 3.1.1: Implement `parcels.rs`**

```rust
// services/api/src/routes/parcels.rs
//! `GET /api/parcels/:pnu` — PNU 19 자리로 필지 정보 조회 (SP10 panel.parcel.summary 의 backing).
//!
//! parcel-lookup crate 의 `ParcelInfoLookup::lookup` 호출 → V-World 또는 NoOp 응답.
//! 본 핸들러는 "panel" 단어를 모름 — pure REST resource (spec § 7 F1).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::Json;
use parcel_lookup::ParcelInfoLookup;
use serde::Serialize;
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

#[derive(Clone)]
pub struct ParcelsState {
    pub parcel_lookup: Arc<dyn ParcelInfoLookup>,
}

/// 필지 정보 응답. ADR 0018 PNU-First denormalize 와 동일 surface.
#[derive(Debug, Serialize)]
pub struct ParcelInfoResponse {
    pub pnu: String,
    /// 행정구역 시도 코드 (2자리).
    pub sido_code: String,
    /// 행정구역 시군구 코드 (5자리, prefix 포함).
    pub sigungu_code: String,
    /// 행정구역 읍면동 코드 (8자리, prefix 포함).
    pub eupmyeondong_code: String,
    /// 시도 한국어명.
    pub sido_name: String,
    /// 시군구 한국어명.
    pub sigungu_name: String,
    /// 읍면동 한국어명.
    pub eupmyeondong_name: String,
    /// 지목 (factory_site / warehouse_site / ...).
    pub land_use_type: String,
    /// 용도지역 (residential / commercial / ...). V-World 미제공 시 None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoning: Option<String>,
    /// 공시지가 (KRW/m²). 미고시 → None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub official_land_price_per_m2: Option<i64>,
    /// 공시지가 고시 연·월 (예: "202504").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gosi_year_month: Option<String>,
}

/// `GET /api/parcels/:pnu` — 인증 필수.
pub async fn get_parcel(
    State(state): State<ParcelsState>,
    _auth: AuthenticatedUser,
    Path(pnu_raw): Path<String>,
) -> Result<Json<ParcelInfoResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(pnu_raw.clone()).map_err(|e| {
        problem(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_pnu",
            "잘못된 필지 PNU 에요",
            Some(format!("{e}")),
        )
    })?;

    let info = state
        .parcel_lookup
        .lookup(&pnu)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, pnu = %pnu_raw, "parcel_lookup failed");
            problem(
                axum::http::StatusCode::BAD_GATEWAY,
                "parcel_lookup_failed",
                "필지 정보를 불러오지 못했어요. 잠시 후 다시 시도해 주세요",
                None,
            )
        })?
        .ok_or_else(|| {
            problem(
                axum::http::StatusCode::NOT_FOUND,
                "parcel_not_found",
                "해당 필지를 찾지 못했어요",
                Some(format!("pnu={pnu_raw}")),
            )
        })?;

    Ok(Json(ParcelInfoResponse {
        pnu: pnu_raw,
        sido_code: info.admin.sido_code().as_str().to_owned(),
        sigungu_code: info.admin.sigungu_code().as_str().to_owned(),
        eupmyeondong_code: info.admin.eupmyeondong_code().as_str().to_owned(),
        sido_name: info.admin.sido_name().to_owned(),
        sigungu_name: info.admin.sigungu_name().to_owned(),
        eupmyeondong_name: info.admin.eupmyeondong_name().to_owned(),
        land_use_type: info.land_use_type.as_str().to_owned(),
        zoning: info.zoning.as_ref().map(|z| z.as_str().to_owned()),
        official_land_price_per_m2: info.official_land_price_per_m2.map(|m| m.as_i64()),
        gosi_year_month: info.gosi_year_month.as_ref().map(|y| y.to_string()),
    }))
}
```

> **NOTE for engineer:** 위 코드의 `info.admin.sido_code().as_str()`, `info.land_use_type.as_str()` 등은 `shared_kernel` / `parcel_domain` 의 실제 method 이름으로 1:1 매핑 — 만약 method 이름이 다르면 (예: `sido()`, `code()`) 호출부만 조정. `ParcelInfo` struct shape 는 [`crates/parcel-lookup/src/info.rs`](../../../crates/parcel-lookup/src/info.rs) SSOT.

- [ ] **Step 3.1.2: Commit**

```bash
git add services/api/src/routes/parcels.rs
git commit -m "feat(sp10-t3): backend GET /api/parcels/:pnu — parcel-lookup REST shell"
```

### Step 3.2: `GET /api/buildings?parcel_pnu=:pnu`

- [ ] **Step 3.2.1: Pre-check building reader crate exists**

Run: `ls crates/data-clients/data-go-kr/`
Expected: directory contains `building_register` (per spec § 7.1 — SP4-iii-a 기존). If missing, switch this endpoint to a stub returning empty list with TODO comment, escalate to user.

- [ ] **Step 3.2.2: Implement `buildings.rs` (live path if reader exists)**

```rust
// services/api/src/routes/buildings.rs
//! `GET /api/buildings?parcel_pnu=:pnu` — 필지 위 건축물 list.
//! data.go.kr `getBrTitleInfo` 위 thin REST shell (SP4-iii-a building reader 호출).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// SP4-iii-a 의 BuildingRegisterReader trait.
pub trait BuildingRegisterReader: Send + Sync {
    /// PNU 로 건축물 list 조회. 빈 vec 가능.
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<BuildingItem>>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct BuildingItem {
    pub mgm_bldrgst_pk: String, // 관리건축물대장PK
    pub bldg_nm: String,
    pub main_purps_cd_nm: String, // 주용도코드명 (예: "공장")
    pub tot_area: f64,            // m²
    pub use_apr_day: Option<String>, // 사용승인일 YYYYMMDD
}

#[derive(Clone)]
pub struct BuildingsState {
    pub reader: Arc<dyn BuildingRegisterReader>,
}

#[derive(Debug, Deserialize)]
pub struct BuildingsQuery {
    pub parcel_pnu: String,
}

#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub total_area_m2: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BuildingsResponse {
    pub buildings: Vec<BuildingResponse>,
}

pub async fn list_buildings(
    State(state): State<BuildingsState>,
    _auth: AuthenticatedUser,
    Query(q): Query<BuildingsQuery>,
) -> Result<Json<BuildingsResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(q.parcel_pnu.clone()).map_err(|e| {
        problem(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_pnu",
            "잘못된 필지 PNU 에요",
            Some(format!("{e}")),
        )
    })?;

    let items = state.reader.list_by_pnu(&pnu).await.map_err(|e| {
        tracing::warn!(error = %e, pnu = %q.parcel_pnu, "building_register read failed");
        problem(
            axum::http::StatusCode::BAD_GATEWAY,
            "buildings_lookup_failed",
            "건축물 정보를 불러오지 못했어요",
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
```

> **NOTE:** Reader crate 의 실제 trait 이름·method 시그니처가 다르면 (예: `GosiBuildingReader::find`), 본 파일의 `BuildingRegisterReader` trait 정의를 제거하고 그 crate 를 직접 import. 위 stub 정의는 reader 부재 시 fallback shape.

- [ ] **Step 3.2.3: Commit**

```bash
git add services/api/src/routes/buildings.rs
git commit -m "feat(sp10-t3): backend GET /api/buildings — data.go.kr building_register REST shell"
```

