//! Gold 단계 — `PMTiles` 빌드 + flat tile decomposition (ADR 0021) + R2 활성화.
//!
//! 모듈 구성:
//! - [`spawn`]: WSL pass-through helper (Windows dev → wsl.exe, Linux CI → 직접).
//! - [`tippecanoe`]: tippecanoe binary spawn (parcels/admin/complex layer).
//! - [`shp_to_geojson`]: ogr2ogr binary spawn (production SHP → `GeoJSON`).
//! - [`decompose`]: ADR 0021 — `PMTiles` → flat `{z}/{x}/{y}.pbf` (`tile-join` spawn).
//! - [`build`]: 한 layer 의 빌드 오케스트레이터 (tippecanoe → decompose).
//! - [`manifest`]: Gold manifest 데이터 모델 (T3b.1 박제, T3b.2 hot-swap 시 활성).

pub mod build;
pub mod decompose;
pub mod manifest;
pub mod shp_to_geojson;
pub mod spawn;
pub mod tippecanoe;
pub mod verify;
