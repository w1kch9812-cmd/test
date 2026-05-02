//! `AnalysisReport` 도메인 (Insights BC, RDS 동적).
//!
//! 사용자가 다수 필지를 묶어 저장한 분석 리포트. `R2` 데이터 시점 캐시(`snapshot`)와
//! optimistic locking(`version`)을 갖는 Aggregate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
