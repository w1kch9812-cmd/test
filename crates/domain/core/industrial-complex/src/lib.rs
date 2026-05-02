//! `IndustrialComplex` 도메인 (Core BC, `R2` 정적).
//!
//! 한국 산업단지 데이터 — `KICOX`/data.go.kr에서 ETL되어 `R2`에 보관.
//! 4종 (국가/일반/도시첨단/농공). Reader trait만 정의 (구현 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod kind;
pub mod reader;
