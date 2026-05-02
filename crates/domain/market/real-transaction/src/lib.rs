//! `RealTransaction` 도메인 (Market BC, `R2` 정적).
//!
//! 한국 실거래가 공개 데이터 (`data.go.kr`) `ETL` → `R2` 보관.
//! Reader trait만 정의해요 (구현은 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod reader;
pub mod transaction_kind;
