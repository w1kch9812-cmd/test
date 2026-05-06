//! Gold 단계 — `PMTiles` 빌드 + R2 Gold artifact + manifest hot-swap.
//!
//! T3b.1 = manifest 데이터 모델만 (R2 업로드 path 의 type-safe 위치 선점).
//! T3b.2 = ogr2ogr → tippecanoe → upload → activate 의 전체 pipeline.

pub mod manifest;
