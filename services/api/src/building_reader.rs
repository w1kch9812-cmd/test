//! `DataGoKrBuildingRegisterReader` — `routes::buildings::BuildingRegisterReader` 의
//! data.go.kr 라이브 구현체.
//!
//! `getBrTitleInfo` JSON 응답을 `data_go_kr_client::building_register::parser` 에
//! delegate → `building_domain::Building` Silver entity. panel-only 호출이라 V-World
//! 폴리곤 합성 안 함 → `geom = None`.
//!
//! # SSOT (2026-05-08 unification)
//!
//! 이전 panel-only `BuildingItem` (21 필드) 폐기. 이제 `Building` 엔티티 단일 source.
//! enum mapping (purpose/structure Cd primary + label fallback) 도 data-go-kr 의
//! `parse_building_title` SSOT 재사용 — 본 파일은 *delegate + geom 비활성화* 만.

use std::sync::Arc;

use building_domain::entity::Building;
use chrono::Utc;
use data_go_kr_client::building_register::{parser::parse_building_title, BuildingRegisterClient};
use data_go_kr_client::{pnu_split, DataGoKrClient};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use raw_capture_client::RawCapture;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;
use tracing::warn;

use crate::routes::buildings::{BuildingRegisterError, BuildingRegisterReader};

/// `parcel_external_data.source` CHECK 라벨.
const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";

/// `BuildingRegisterReader` 의 data.go.kr 라이브 구현체.
///
/// `getBrTitleInfo` raw JSON 을 `RawCapture` (R2 Bronze) 에 best-effort 보존 → 모든 시점
/// 영구 archive. 보존 실패는 warn 로그 + 응답 정상 진행 (raw 손실은 R2 + 디스크 둘 다
/// 죽어야 발생, ADR 0026).
pub struct DataGoKrBuildingRegisterReader {
    client: Arc<DataGoKrClient>,
    raw_capture: Arc<dyn RawCapture>,
}

impl DataGoKrBuildingRegisterReader {
    /// 새 [`DataGoKrBuildingRegisterReader`].
    #[must_use]
    pub const fn new(client: Arc<DataGoKrClient>, raw_capture: Arc<dyn RawCapture>) -> Self {
        Self {
            client,
            raw_capture,
        }
    }
}

impl BuildingRegisterReader for DataGoKrBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<Building>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let parts = pnu_split::split(pnu);
            let br = BuildingRegisterClient::new(&self.client);
            let raw = br
                .fetch_title_info(parts)
                .await
                .map_err(|e| Box::new(e) as BuildingRegisterError)?;

            // raw_capture best-effort — R2 Bronze 영구 archive (ADR 0026). 실패는 warn 만.
            let now = Utc::now();
            if let Err(capture_err) = self
                .raw_capture
                .capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, now)
                .await
            {
                warn!(
                    pnu = %pnu.as_str(),
                    source = RAW_CAPTURE_SOURCE,
                    error = %capture_err,
                    "raw_capture failed — proceeding with parsed result"
                );
            }

            // SSOT delegate: data-go-kr 의 parser 가 enum mapping (Cd primary + label
            // fallback) + 모든 수치 필드 처리. dummy polygon 은 parse 시그니처 만족용,
            // 결과에서 geom = None 으로 mutate (panel reader 는 V-World 폴리곤 합성 X).
            let mut buildings = parse_building_title(&raw, pnu, &dummy_polygon(), now)
                .map_err(|e| Box::new(e) as BuildingRegisterError)?;
            for b in &mut buildings {
                b.geom = None;
            }
            Ok(buildings)
        })
    }
}

/// parse 시그니처 만족용 dummy polygon — 결과에서 geom = None 으로 mutate 되므로
/// 실제 사용 안 됨. unwrap 안전 (constant WGS84 polygon).
fn dummy_polygon() -> PolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![]))
        .unwrap_or_else(|_| unreachable!("constant polygon always valid WGS84"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use building_domain::purpose_code::BuildingPurposeCode;
    use building_domain::structure_code::BuildingStructureCode;
    use serde_json::Value;
    use shared_kernel::area::AreaM2;

    /// data-go-kr parser 가 Building Silver 로 매핑되는지 (delegate path 검증) — 실 fixture.
    #[test]
    #[allow(clippy::cognitive_complexity)] // fixture 의 다수 필드 검증 — 분해 시 fixture I/O 중복.
    fn parses_live_fixture_to_building_silver() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates/data-clients/data-go-kr/tests/fixtures/live_2026-05-08_gangnam_yeoksam_737.json");
        let raw_str = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
        let raw: Value = serde_json::from_str(&raw_str).expect("valid JSON");
        let pnu = Pnu::try_new("1168010100107370000").expect("valid pnu");

        let mut buildings = parse_building_title(&raw, &pnu, &dummy_polygon(), Utc::now())
            .expect("parse ok");
        for b in &mut buildings {
            b.geom = None;
        }

        assert_eq!(buildings.len(), 1);
        let b = &buildings[0];

        // 식별자
        assert_eq!(b.mgm_bldrgst_pk, "1024112777");
        assert_eq!(b.building_name.as_deref(), Some("강남파이낸스센터"));
        assert!(b.plat_plc.is_some());

        // enum mapping (Cd primary + label fallback)
        assert_eq!(b.main_purpose_code, BuildingPurposeCode::Office); // mainPurpsCd "14000"
        assert_eq!(b.structure_code, BuildingStructureCode::SteelReinforcedConcrete); // strctCd "42"

        // 면적 / 비율 — Building Silver 는 panel 추가 필드 None (rich parser 가 아직 채움 안 함, FU 41+)
        // 단 totArea 는 필수 → AreaM2 invariant 검증.
        assert!(b.total_floor_area_m2.as_f64() > 200_000.0); // 212615.29
        assert!(b.total_floor_area_m2 == AreaM2::try_new(212_615.29).unwrap());

        // 층/높이
        assert_eq!(b.ground_floors, 45);
        assert_eq!(b.underground_floors, 8);
        assert!(b.height_m.is_some_and(|v| v > 200.0));

        // 사용승인일
        assert!(b.use_approval_date.is_some());

        // panel-only path 라 geom None
        assert!(b.geom.is_none());
    }
}
