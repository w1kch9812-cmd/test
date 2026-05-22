//! tippecanoe spawn — `GeoJSON` 파일들 → 단일 `PMTiles` 빌드.
//!
//! 본 모듈은 `tippecanoe` binary 를 실행. binary 자체는 dev WSL 에 빌드됨
//! (`/usr/local/bin/tippecanoe`) 또는 CI Ubuntu 에서 felt/tippecanoe make.
//!
//! Layer 별 zoom 스펙 (ADR 0016 §):
//! - **parcels** Z14-17 — 매물 클릭 단위, 가까이서만 visible.
//! - **admin**   Z6-12  — 행정구역 outline, 멀리서 visible.
//! - **complex** Z0-16  — 산업단지 boundary, **모든 zoom 에서 visible** (사용자 SSS 요구).
//!   → low-zoom 에 tippecanoe `--coalesce-smallest-as-needed` 가 sub-pixel polygon merge.
//!
//! flag 셋은 [gongzzang-design-lab build-pmtiles.ts] 검증된 값과 동일:
//! `-P --no-feature-limit --no-tile-size-limit --drop-smallest-as-needed`
//! `--simplification=10 --extend-zooms-if-still-dropping --attribute-type=pnu:string`.

#![allow(clippy::doc_markdown)]

mod availability;
mod error;
mod run;
#[cfg(test)]
mod tests;
mod types;

pub use availability::check_available;
pub use error::TippecanoeError;
#[allow(unused_imports)]
pub use run::{run, TippecanoeArgs, TippecanoeResult};
pub use types::LayerKind;
