//! `CourtAuction` 도메인 (Market BC, `R2` 정적).
//!
//! 한국 법원 경매 공개 데이터 `ETL` → `R2` 보관 (활성 + 이력 모두 포함).
//! Reader trait만 정의해요 (구현은 sub-project 4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod auction_kind;
pub mod auction_status;
pub mod entity;
pub mod errors;
pub mod reader;
