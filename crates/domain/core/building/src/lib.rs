//! `Building` 도메인 (Core BC, `R2` 정적).
//!
//! 한국 건축물대장 데이터 — V-World/data.go.kr에서 ETL되어 `R2`에 보관.
//! 한 필지(`Pnu`)에 여러 건물 가능. Reader trait만 정의 (구현 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod purpose_code;
pub mod reader;
pub mod structure_code;
