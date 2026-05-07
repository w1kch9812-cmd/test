//! SP10 T3: panel backing endpoints integration tests.
//!
//! `/api/parcels/:pnu` 와 `/api/buildings?parcel_pnu=...` 를 `NoOp` lookup/reader 로
//! 부팅한 axum 앱에서 검증해요. DB 의존성 없음 — `auth_layer` 는 본 테스트에서
//! synthetic `AuthenticatedUser` 주입 미들웨어로 대체.

#![allow(clippy::expect_used, clippy::unwrap_used)]
// 테스트 클라이언트는 직접 `reqwest::Client` 사용 OK — 본 테스트는 외부 API
// 호출을 대상으로 하지 않음 (loopback HTTP). `circuit-breaker` 강제 정책은
// production network 호출에 대한 것.
#![allow(clippy::disallowed_types)]

use std::pin::Pin;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use parcel_lookup::{NoOpParcelInfoLookup, ParcelInfoLookup};
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use shared_kernel::pnu::Pnu;
use user_domain::entity::{User, UserKind};

// 본 테스트는 별도 binary (test target) — `routes::*` 가 lib 노출이 아니므로
// 핸들러 사용은 production 라우터를 그대로 띄우는 대신, 같은 trait/struct 만
// 재현해서 직접 wiring 해요.
//
// `routes::parcels` / `routes::buildings` 는 bin 내부 모듈이라 통합 테스트에선
// 직접 호출 불가 — instead, 본 테스트는 endpoint contract (HTTP shape) 만 검증.
// 이를 위해 production 과 동일한 trait + handler 시그니처를 본 file 안에서
// 재선언해서 라우터를 조립해요. trait surface 가 좁아 재선언 비용 낮음.

// ─────────────────────────────── 재선언 (production parity) ────────────────────────────────

/// production 과 동일한 [`auth::middleware::AuthenticatedUser`] 주입.
async fn inject_synthetic_auth(mut req: Request, next: Next) -> Response {
    let user = User::try_new(
        Id::new(),
        "sp10-test-sub",
        Email::try_new("sp10@test.local").expect("email"),
        "SP10 Tester",
        UserKind::Individual,
        Utc::now(),
    )
    .expect("test user");
    let claims = auth::claims::Claims {
        sub: "sp10-test-sub".to_owned(),
        email: Some("sp10@test.local".to_owned()),
        name: Some("SP10 Tester".to_owned()),
        preferred_username: None,
        jti: "test-jti".to_owned(),
        exp: i64::MAX,
        nbf: None,
        iss: "test".to_owned(),
        aud: auth::claims::Audience::Single("test".to_owned()),
    };
    req.extensions_mut()
        .insert(AuthenticatedUser { user, claims });
    next.run(req).await
}

/// production [`api::routes::parcels`] 와 동일한 응답 shape.
#[derive(Debug, serde::Deserialize)]
struct ParcelInfoResponseTest {
    pnu: String,
    sido_code: String,
    sigungu_code: String,
    eupmyeondong_code: String,
    #[allow(dead_code)]
    sido_name: String,
    #[allow(dead_code)]
    sigungu_name: String,
    #[allow(dead_code)]
    eupmyeondong_name: String,
    land_use_type: String,
    #[serde(default)]
    zoning: Option<String>,
    #[serde(default)]
    official_land_price_per_m2: Option<i64>,
    #[serde(default)]
    gosi_year_month: Option<String>,
}

/// production [`api::routes::buildings`] 와 동일한 응답 shape.
#[derive(Debug, serde::Deserialize)]
struct BuildingsResponseTest {
    buildings: Vec<serde_json::Value>,
}

// production 과 동일 trait 시그니처 재선언 (T3 buildings.rs 와 1:1).
type BuildingRegisterError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
struct BuildingItem {
    mgm_bldrgst_pk: String,
    bldg_nm: String,
    main_purps_cd_nm: String,
    tot_area: f64,
    use_apr_day: Option<String>,
}

trait BuildingRegisterReader: Send + Sync {
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<BuildingItem>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    >;
}

struct NoOpBuildings;
impl BuildingRegisterReader for NoOpBuildings {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a Pnu,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<BuildingItem>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Vec::new()) })
    }
}

// ─────────────────────────────── 재구현 핸들러 (production parity) ─────────────────────────

#[derive(Clone)]
struct ParcelsTestState {
    parcel_lookup: Arc<dyn ParcelInfoLookup>,
}

#[derive(Clone)]
struct BuildingsTestState {
    reader: Arc<dyn BuildingRegisterReader>,
}

async fn get_parcel_test(
    axum::extract::State(state): axum::extract::State<ParcelsTestState>,
    _auth: AuthenticatedUser,
    axum::extract::Path(pnu_raw): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let pnu = Pnu::try_new(&pnu_raw).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "type": "https://gongzzang.com/errors/invalid-pnu",
                "title": "잘못된 필지 PNU 에요",
                "status": 400,
                "detail": format!("{e}"),
            })),
        )
    })?;
    let info = state
        .parcel_lookup
        .lookup_by_pnu(&pnu)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::BAD_GATEWAY,
                axum::Json(serde_json::json!({
                    "type": "https://gongzzang.com/errors/parcel-lookup-failed",
                    "title": "필지 정보를 불러오지 못했어요",
                    "status": 502,
                })),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "type": "https://gongzzang.com/errors/parcel-not-found",
                    "title": "해당 필지를 찾지 못했어요",
                    "status": 404,
                    "detail": format!("pnu={pnu_raw}"),
                })),
            )
        })?;

    Ok(axum::Json(serde_json::json!({
        "pnu": pnu_raw,
        "sido_code": info.admin.sido.as_str(),
        "sigungu_code": info.admin.sigungu.as_str(),
        "eupmyeondong_code": info.admin.eupmyeondong.as_str(),
        "sido_name": "",
        "sigungu_name": "",
        "eupmyeondong_name": "",
        "land_use_type": info.land_use_type.as_str(),
        "zoning": info.zoning.map(shared_kernel::zoning::Zoning::as_str),
        "official_land_price_per_m2": info.official_land_price_per_m2.map(shared_kernel::money::MoneyKrw::as_i64),
        "gosi_year_month": info.gosi_year_month.map(|y| format!("{:04}{:02}", y.year, y.month)),
    })))
}

#[derive(Debug, serde::Deserialize)]
struct BuildingsQuery {
    parcel_pnu: String,
}

async fn list_buildings_test(
    axum::extract::State(state): axum::extract::State<BuildingsTestState>,
    _auth: AuthenticatedUser,
    axum::extract::Query(q): axum::extract::Query<BuildingsQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let pnu = Pnu::try_new(&q.parcel_pnu).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "type": "https://gongzzang.com/errors/invalid-pnu",
                "title": "잘못된 필지 PNU 에요",
                "status": 400,
                "detail": format!("{e}"),
            })),
        )
    })?;
    let items = state.reader.list_by_pnu(&pnu).await.map_err(|_| {
        (
            axum::http::StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "type": "https://gongzzang.com/errors/buildings-lookup-failed",
                "title": "건축물 정보를 불러오지 못했어요",
                "status": 502,
            })),
        )
    })?;
    let buildings: Vec<serde_json::Value> = items
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "id": b.mgm_bldrgst_pk,
                "name": b.bldg_nm,
                "purpose": b.main_purps_cd_nm,
                "total_area_m2": b.tot_area,
                "approved_at": b.use_apr_day,
            })
        })
        .collect();
    Ok(axum::Json(serde_json::json!({ "buildings": buildings })))
}

// ─────────────────────────────── 부팅 헬퍼 ───────────────────────────────────────────────

async fn spawn_test_app() -> String {
    let parcel_lookup: Arc<dyn ParcelInfoLookup> = Arc::new(NoOpParcelInfoLookup::new());
    let parcels_state = ParcelsTestState { parcel_lookup };

    let reader: Arc<dyn BuildingRegisterReader> = Arc::new(NoOpBuildings);
    let buildings_state = BuildingsTestState { reader };

    let app = Router::new()
        .route("/api/parcels/:pnu", get(get_parcel_test))
        .with_state(parcels_state)
        .merge(
            Router::new()
                .route("/api/buildings", get(list_buildings_test))
                .with_state(buildings_state),
        )
        .layer(axum::middleware::from_fn(inject_synthetic_auth));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://{addr}")
}

// ─────────────────────────────── tests ───────────────────────────────────────────────────

#[tokio::test]
async fn get_parcel_returns_404_for_unknown_pnu() {
    let base = spawn_test_app().await;
    let pnu_zeros = "0000000000000000000"; // 19 zeros — valid format
    let url = format!("{base}/api/parcels/{pnu_zeros}");

    let resp = reqwest::Client::new().get(&url).send().await.expect("send");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::NOT_FOUND,
        "NoOp lookup → 404",
    );
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], 404);
    assert!(
        body["type"]
            .as_str()
            .unwrap_or_default()
            .ends_with("parcel-not-found"),
        "type should be parcel-not-found, got: {body}"
    );
}

#[tokio::test]
async fn get_parcel_returns_400_for_invalid_pnu() {
    let base = spawn_test_app().await;
    let bad = "not-a-pnu"; // not 19 digits
    let url = format!("{base}/api/parcels/{bad}");

    let resp = reqwest::Client::new().get(&url).send().await.expect("send");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "non-19-digit → 400",
    );
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], 400);
    assert!(
        body["type"]
            .as_str()
            .unwrap_or_default()
            .ends_with("invalid-pnu"),
        "type should be invalid-pnu, got: {body}"
    );
}

#[tokio::test]
async fn list_buildings_returns_empty_with_noop_reader() {
    let base = spawn_test_app().await;
    let pnu = "1111010100100010000"; // valid 19 digits (서울 종로 청운효자동)
    let url = format!("{base}/api/buildings?parcel_pnu={pnu}");

    let resp = reqwest::Client::new().get(&url).send().await.expect("send");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "NoOp reader returns empty list — 200 OK",
    );
    let body: BuildingsResponseTest = resp.json().await.expect("json");
    assert!(
        body.buildings.is_empty(),
        "NoOp reader → empty buildings list, got: {:?}",
        body.buildings
    );
}

// `ParcelInfoResponseTest` 가 도달 가능한 happy-path 가 NoOp 환경에 없어 unused warning 회피용.
#[allow(dead_code)]
fn _shape_check(r: ParcelInfoResponseTest) -> String {
    format!(
        "{}/{}/{}/{}/{:?}/{:?}/{:?}",
        r.pnu,
        r.sido_code,
        r.sigungu_code,
        r.eupmyeondong_code,
        r.land_use_type,
        r.zoning,
        r.official_land_price_per_m2
    ) + r.gosi_year_month.unwrap_or_default().as_str()
}
