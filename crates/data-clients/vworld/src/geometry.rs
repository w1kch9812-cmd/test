//! V-World GeoJSON geometry → 도메인 [`MultiPolygonSrid`] 변환.
//!
//! V-World `LP_PA_CBND_BUBUN` 응답은 항상 `MultiPolygon`이지만, 코드 안전을
//! 위해 `Polygon`도 받아 단일-멤버 `MultiPolygon`으로 승격. 외부 응답이 미세
//! drift해도 견고.
//!
//! GeoJSON 좌표 배열 형태:
//! - Polygon: `[[[lng, lat], ...]]` (외곽 ring + 0개 이상 inner)
//! - MultiPolygon: `[[[[lng, lat], ...]]]` (Polygon 배열)

#![allow(clippy::module_name_repetitions)]

use geo_types::{Coord, LineString, MultiPolygon as GeoMultiPolygon, Polygon as GeoPolygon};
use serde_json::Value;
use shared_kernel::geometry::MultiPolygonSrid;

use crate::error::ParseError;

/// V-World feature 의 `geometry` 객체를 [`MultiPolygonSrid`]로 변환.
///
/// 지원 타입: `Polygon`, `MultiPolygon`. 그 외는 [`ParseError::Malformed`].
///
/// # Errors
///
/// - `geometry.type` 누락/비지원 → [`ParseError::Malformed`]
/// - 좌표 배열이 GeoJSON spec 위반 → [`ParseError::Malformed`]
/// - 도메인 invariant 위반 (좌표 범위, ring 길이) → [`ParseError::Domain`]
pub fn parse_geometry(geom: &Value) -> Result<MultiPolygonSrid, ParseError> {
    let geom_type = geom
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("geometry.type missing".into()))?;
    let coords = geom
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or_else(|| ParseError::Malformed("geometry.coordinates missing or not array".into()))?;

    let polygons = match geom_type {
        "Polygon" => vec![parse_polygon_coords(coords)?],
        "MultiPolygon" => coords
            .iter()
            .map(|p| {
                let arr = p.as_array().ok_or_else(|| {
                    ParseError::Malformed("MultiPolygon member not array".into())
                })?;
                parse_polygon_coords(arr)
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => {
            return Err(ParseError::Malformed(format!(
                "unsupported geometry.type '{other}' (expected Polygon or MultiPolygon)"
            )));
        }
    };

    MultiPolygonSrid::try_new_wgs84(GeoMultiPolygon(polygons))
        .map_err(|e| ParseError::Domain(format!("geometry: {e}")))
}

/// `[[[lng,lat], ...], inner_ring1, ...]` 배열 → `geo_types::Polygon`.
fn parse_polygon_coords(rings: &[Value]) -> Result<GeoPolygon<f64>, ParseError> {
    let mut iter = rings.iter();
    let outer = iter
        .next()
        .ok_or_else(|| ParseError::Malformed("polygon missing exterior ring".into()))?;
    let exterior = parse_ring(outer)?;
    let mut interiors = Vec::new();
    for inner in iter {
        interiors.push(parse_ring(inner)?);
    }
    Ok(GeoPolygon::new(exterior, interiors))
}

fn parse_ring(ring: &Value) -> Result<LineString<f64>, ParseError> {
    let arr = ring
        .as_array()
        .ok_or_else(|| ParseError::Malformed("ring not array".into()))?;
    let mut points = Vec::with_capacity(arr.len());
    for pair in arr {
        let pair_arr = pair
            .as_array()
            .ok_or_else(|| ParseError::Malformed("coordinate pair not array".into()))?;
        let lng = pair_arr
            .first()
            .and_then(Value::as_f64)
            .ok_or_else(|| ParseError::Malformed("lng not f64".into()))?;
        let lat = pair_arr
            .get(1)
            .and_then(Value::as_f64)
            .ok_or_else(|| ParseError::Malformed("lat not f64".into()))?;
        points.push(Coord { x: lng, y: lat });
    }
    Ok(LineString(points))
}

#[cfg(test)]
mod tests {
    // 실 V-World 응답 fixture 좌표 — readability 보다 fidelity 우선 (separator 추가 시 raw 응답과 mismatch).
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreadable_literal,
        clippy::float_cmp
    )]

    use super::*;
    use serde_json::json;

    #[test]
    fn parses_polygon_into_single_member_multipolygon() {
        let geom = json!({
            "type": "Polygon",
            "coordinates": [[
                [126.97, 37.56],
                [126.98, 37.56],
                [126.98, 37.57],
                [126.97, 37.57],
                [126.97, 37.56]
            ]]
        });
        let mp = parse_geometry(&geom).expect("ok");
        assert_eq!(mp.polygon_count(), 1);
        assert_eq!(mp.first_polygon().exterior().0.len(), 5);
    }

    #[test]
    fn parses_multipolygon_with_two_members() {
        let geom = json!({
            "type": "MultiPolygon",
            "coordinates": [
                [[[126.0, 37.0], [127.0, 37.0], [127.0, 38.0], [126.0, 37.0]]],
                [[[128.0, 37.0], [129.0, 37.0], [129.0, 38.0], [128.0, 37.0]]]
            ]
        });
        let mp = parse_geometry(&geom).expect("ok");
        assert_eq!(mp.polygon_count(), 2);
    }

    #[test]
    fn parses_multipolygon_from_real_v_world_response() {
        // 실 V-World 응답 모양 — single member MultiPolygon (강남 yeoksam 737 발췌).
        let geom = json!({
            "type": "MultiPolygon",
            "coordinates": [[[
                [127.03582619570822, 37.50014255162943],
                [127.03586196213088, 37.500208764641435],
                [127.0367965516218, 37.500495637519585],
                [127.03692036362017, 37.5004512713471],
                [127.03740886415942, 37.499410920905056],
                [127.0373807948595, 37.49936173489393],
                [127.03666077776445, 37.49914715315316],
                [127.03624357167334, 37.49933387338619],
                [127.03582619570822, 37.50014255162943]
            ]]]
        });
        let mp = parse_geometry(&geom).expect("ok");
        assert_eq!(mp.polygon_count(), 1);
        assert_eq!(mp.first_polygon().exterior().0.len(), 9);
    }

    #[test]
    fn rejects_unsupported_geometry_type() {
        let geom = json!({ "type": "LineString", "coordinates": [[1.0, 2.0], [3.0, 4.0]] });
        let err = parse_geometry(&geom).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("LineString")));
    }

    #[test]
    fn rejects_missing_type() {
        let geom = json!({ "coordinates": [] });
        let err = parse_geometry(&geom).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("type")));
    }

    #[test]
    fn rejects_short_ring() {
        let geom = json!({
            "type": "Polygon",
            "coordinates": [[[126.0, 37.0], [127.0, 37.0], [126.0, 37.0]]]
        });
        let err = parse_geometry(&geom).unwrap_err();
        // 도메인 검증에서 ExteriorRingTooShort.
        assert!(matches!(err, ParseError::Domain(s) if s.contains("≥4")));
    }

    #[test]
    fn rejects_lng_out_of_range() {
        let geom = json!({
            "type": "Polygon",
            "coordinates": [[
                [200.0, 37.0],
                [127.0, 37.0],
                [127.0, 38.0],
                [126.0, 37.0]
            ]]
        });
        let err = parse_geometry(&geom).unwrap_err();
        assert!(matches!(err, ParseError::Domain(s) if s.contains("180") || s.contains("longitude")));
    }
}
