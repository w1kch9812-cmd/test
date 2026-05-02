//! `Parcel` 도메인 (Core BC, `R2` 정적).
//!
//! 한국 필지 (`Parcel`) 데이터 — V-World/data.go.kr에서 ETL되어 `R2`에 보관.
//! Aggregate 자체는 *read-only* — Reader trait만 정의 (구현은 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod reader;
