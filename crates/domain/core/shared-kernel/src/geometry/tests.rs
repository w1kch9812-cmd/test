#![allow(clippy::expect_used, clippy::unwrap_used)]

use geo_types::{Coord, LineString};

use super::*;

// ── Valid construction ────────────────────────────────────────

#[test]
fn wgs84_seoul_city_hall() {
    // 서울시청: lng=126.9784, lat=37.5666 (대략)
    let p = PointSrid::try_new_wgs84(126.9784, 37.5666).expect("valid WGS84");
    assert!((p.lng - 126.9784).abs() < f64::EPSILON);
    assert!((p.lat - 37.5666).abs() < f64::EPSILON);
    assert_eq!(p.srid, Srid::Wgs84);
}

#[test]
fn wgs84_origin_zero_zero() {
    let p = PointSrid::try_new_wgs84(0.0, 0.0).expect("origin valid");
    assert!(p.lng.abs() < f64::EPSILON);
    assert!(p.lat.abs() < f64::EPSILON);
}

#[test]
fn wgs84_boundary_lng_180() {
    let p = PointSrid::try_new_wgs84(180.0, 0.0).expect("boundary 180 inclusive");
    assert!((p.lng - 180.0).abs() < f64::EPSILON);
}

#[test]
fn wgs84_boundary_lng_neg_180() {
    let p = PointSrid::try_new_wgs84(-180.0, 0.0).expect("boundary -180 inclusive");
    assert!((p.lng + 180.0).abs() < f64::EPSILON);
}

#[test]
fn wgs84_boundary_lat_90() {
    let p = PointSrid::try_new_wgs84(0.0, 90.0).expect("boundary 90 inclusive");
    assert!((p.lat - 90.0).abs() < f64::EPSILON);
}

#[test]
fn wgs84_boundary_lat_neg_90() {
    let p = PointSrid::try_new_wgs84(0.0, -90.0).expect("boundary -90 inclusive");
    assert!((p.lat + 90.0).abs() < f64::EPSILON);
}

// ── Range rejection ─────────────────────────────────────────────

#[test]
fn rejects_lng_above_180() {
    let err = PointSrid::try_new_wgs84(180.5, 0.0).unwrap_err();
    assert!(matches!(err, GeometryError::LngOutOfRange { actual } if actual > 180.0));
}

#[test]
fn rejects_lng_below_neg_180() {
    let err = PointSrid::try_new_wgs84(-181.0, 0.0).unwrap_err();
    assert!(matches!(err, GeometryError::LngOutOfRange { .. }));
}

#[test]
fn rejects_lat_above_90() {
    let err = PointSrid::try_new_wgs84(0.0, 91.0).unwrap_err();
    assert!(matches!(err, GeometryError::LatOutOfRange { .. }));
}

#[test]
fn rejects_lat_below_neg_90() {
    let err = PointSrid::try_new_wgs84(0.0, -91.0).unwrap_err();
    assert!(matches!(err, GeometryError::LatOutOfRange { .. }));
}

// ── Not finite rejection ────────────────────────────────────────

#[test]
fn rejects_lng_nan() {
    let err = PointSrid::try_new_wgs84(f64::NAN, 0.0).unwrap_err();
    assert!(matches!(err, GeometryError::NotFinite { .. }));
}

#[test]
fn rejects_lat_nan() {
    let err = PointSrid::try_new_wgs84(0.0, f64::NAN).unwrap_err();
    assert!(matches!(err, GeometryError::NotFinite { .. }));
}

#[test]
fn rejects_lng_infinity() {
    let err = PointSrid::try_new_wgs84(f64::INFINITY, 0.0).unwrap_err();
    assert!(matches!(err, GeometryError::NotFinite { .. }));
}

#[test]
fn rejects_lng_neg_infinity() {
    let err = PointSrid::try_new_wgs84(f64::NEG_INFINITY, 0.0).unwrap_err();
    assert!(matches!(err, GeometryError::NotFinite { .. }));
}

// ── geo-types interop ──────────────────────────────────────────

#[test]
fn to_geo_point_maps_lng_to_x_lat_to_y() {
    let p = PointSrid::try_new_wgs84(126.9784, 37.5666).expect("valid");
    let geo = p.to_geo_point();
    assert!((geo.x() - 126.9784).abs() < f64::EPSILON);
    assert!((geo.y() - 37.5666).abs() < f64::EPSILON);
}

#[test]
fn copy_semantics_preserves_srid() {
    let p = PointSrid::try_new_wgs84(0.0, 0.0).expect("ok");
    let q = p; // Copy
    assert_eq!(p.srid, q.srid);
    assert!((p.lng - q.lng).abs() < f64::EPSILON);
}

// ── PolygonSrid ────────────────────────────────────────────────

fn unit_square_wgs84() -> GeoPolygon<f64> {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 }, // closing point
    ]);
    GeoPolygon::new(exterior, vec![])
}

#[test]
fn polygon_wgs84_simple_square() {
    let p = PolygonSrid::try_new_wgs84(unit_square_wgs84()).expect("valid");
    assert_eq!(p.srid, Srid::Wgs84);
    assert_eq!(p.polygon.exterior().0.len(), 5);
    assert_eq!(p.polygon.interiors().len(), 0);
}

#[test]
fn polygon_with_hole() {
    let exterior = unit_square_wgs84().exterior().clone();
    let hole = LineString(vec![
        Coord { x: 126.4, y: 37.4 },
        Coord { x: 126.6, y: 37.4 },
        Coord { x: 126.6, y: 37.6 },
        Coord { x: 126.4, y: 37.6 },
        Coord { x: 126.4, y: 37.4 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![hole]);
    let p = PolygonSrid::try_new_wgs84(polygon).expect("valid with hole");
    assert_eq!(p.polygon.interiors().len(), 1);
}

#[test]
fn polygon_rejects_short_exterior_ring() {
    // Only 3 points — too short
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    let err = PolygonSrid::try_new_wgs84(polygon).unwrap_err();
    assert!(matches!(
        err,
        GeometryError::ExteriorRingTooShort { actual: 3 }
    ));
}

#[test]
fn polygon_rejects_lng_out_of_range_exterior() {
    let exterior = LineString(vec![
        Coord { x: 200.0, y: 37.0 }, // lng > 180
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    let err = PolygonSrid::try_new_wgs84(polygon).unwrap_err();
    assert!(matches!(err, GeometryError::LngOutOfRange { .. }));
}

#[test]
fn polygon_rejects_lat_out_of_range_exterior() {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 91.0 }, // lat > 90
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    let err = PolygonSrid::try_new_wgs84(polygon).unwrap_err();
    assert!(matches!(err, GeometryError::LatOutOfRange { .. }));
}

#[test]
fn polygon_rejects_nan_in_exterior() {
    let exterior = LineString(vec![
        Coord {
            x: f64::NAN,
            y: 37.0,
        },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    let err = PolygonSrid::try_new_wgs84(polygon).unwrap_err();
    assert!(matches!(err, GeometryError::NotFinite { .. }));
}

#[test]
fn polygon_rejects_lng_out_of_range_in_hole() {
    let exterior = unit_square_wgs84().exterior().clone();
    let hole = LineString(vec![
        Coord { x: 200.0, y: 37.4 }, // lng > 180 in hole
        Coord { x: 126.6, y: 37.4 },
        Coord { x: 126.6, y: 37.6 },
        Coord { x: 126.4, y: 37.4 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![hole]);
    let err = PolygonSrid::try_new_wgs84(polygon).unwrap_err();
    assert!(matches!(err, GeometryError::LngOutOfRange { .. }));
}

#[test]
fn polygon_boundary_lng_180() {
    let exterior = LineString(vec![
        Coord { x: 180.0, y: 37.0 },
        Coord { x: 179.0, y: 37.0 },
        Coord { x: 179.0, y: 38.0 },
        Coord { x: 180.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    let p = PolygonSrid::try_new_wgs84(polygon).expect("180 inclusive");
    assert_eq!(p.srid, Srid::Wgs84);
}

#[test]
fn polygon_to_geo_polygon_borrows() {
    let p = PolygonSrid::try_new_wgs84(unit_square_wgs84()).expect("valid");
    let geo: &GeoPolygon<f64> = p.as_geo_polygon();
    assert_eq!(geo.exterior().0.len(), 5);
}

#[test]
fn polygon_clone_works() {
    let p = PolygonSrid::try_new_wgs84(unit_square_wgs84()).expect("valid");
    let q = p.clone();
    assert_eq!(p.srid, q.srid);
    assert_eq!(p, q);
}

#[test]
fn polygon_serde_roundtrip() {
    let p = PolygonSrid::try_new_wgs84(unit_square_wgs84()).expect("valid");
    let json = serde_json::to_string(&p).expect("serialize");
    let back: PolygonSrid = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p, back);
}

// ── MultiPolygonSrid ───────────────────────────────────────────

fn unit_multi_wgs84() -> GeoMultiPolygon<f64> {
    GeoMultiPolygon(vec![unit_square_wgs84()])
}

fn two_polygon_multi_wgs84() -> GeoMultiPolygon<f64> {
    let second = GeoPolygon::new(
        LineString(vec![
            Coord { x: 128.0, y: 37.0 },
            Coord { x: 129.0, y: 37.0 },
            Coord { x: 129.0, y: 38.0 },
            Coord { x: 128.0, y: 37.0 },
        ]),
        vec![],
    );
    GeoMultiPolygon(vec![unit_square_wgs84(), second])
}

#[test]
fn multipolygon_single_member_valid() {
    let m = MultiPolygonSrid::try_new_wgs84(unit_multi_wgs84()).expect("valid single");
    assert_eq!(m.srid, Srid::Wgs84);
    assert_eq!(m.polygon_count(), 1);
    assert_eq!(m.first_polygon().exterior().0.len(), 5);
}

#[test]
fn multipolygon_two_members_valid() {
    let m = MultiPolygonSrid::try_new_wgs84(two_polygon_multi_wgs84()).expect("valid two");
    assert_eq!(m.polygon_count(), 2);
}

#[test]
fn multipolygon_rejects_empty() {
    let err = MultiPolygonSrid::try_new_wgs84(GeoMultiPolygon(vec![])).unwrap_err();
    assert!(matches!(err, GeometryError::EmptyMultiPolygon));
}

#[test]
fn multipolygon_rejects_short_exterior_in_member() {
    let bad = GeoPolygon::new(
        LineString(vec![
            Coord { x: 126.0, y: 37.0 },
            Coord { x: 127.0, y: 37.0 },
            Coord { x: 126.0, y: 37.0 },
        ]),
        vec![],
    );
    let err = MultiPolygonSrid::try_new_wgs84(GeoMultiPolygon(vec![bad])).unwrap_err();
    assert!(matches!(err, GeometryError::ExteriorRingTooShort { .. }));
}

#[test]
fn multipolygon_rejects_lng_out_of_range() {
    let bad = GeoPolygon::new(
        LineString(vec![
            Coord { x: 200.0, y: 37.0 },
            Coord { x: 127.0, y: 37.0 },
            Coord { x: 127.0, y: 38.0 },
            Coord { x: 126.0, y: 37.0 },
        ]),
        vec![],
    );
    let err = MultiPolygonSrid::try_new_wgs84(GeoMultiPolygon(vec![bad])).unwrap_err();
    assert!(matches!(err, GeometryError::LngOutOfRange { .. }));
}

#[test]
fn multipolygon_serde_roundtrip() {
    let m = MultiPolygonSrid::try_new_wgs84(two_polygon_multi_wgs84()).expect("valid");
    let json = serde_json::to_string(&m).expect("serialize");
    let back: MultiPolygonSrid = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(m, back);
}

#[test]
fn multipolygon_clone_works() {
    let m = MultiPolygonSrid::try_new_wgs84(unit_multi_wgs84()).expect("valid");
    let cloned = m.clone();
    assert_eq!(m, cloned);
}
