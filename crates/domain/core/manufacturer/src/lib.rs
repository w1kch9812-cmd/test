//! `Manufacturer` 도메인 (Core BC, `R2` 정적).
//!
//! 한국 제조업체 — `KOSIS`/data.go.kr/`KICOX`에서 ETL되어 `R2`에 보관.
//! 식별자는 `BusinessNumber` (R2 정적이므로 cross-BC FK 아님).
//! Reader trait만 정의 (구현 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod employee_count_band;
pub mod entity;
pub mod errors;
pub mod reader;
