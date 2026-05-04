//! 공짱 data.go.kr 외부 API 클라이언트.
//!
//! `crates/circuit-breaker` 위에 data.go.kr 의 API 통합:
//! - [`DataGoKrClient`] — `reqwest::Client` 래퍼 + `Breaker` + `Policy::data_go_kr_default`
//! - [`building_register::BuildingRegisterClient`] — 건축물대장 표제부 (`getBrTitleInfo`)
//! - [`building_register::DataGoKrBuildingReader`] —
//!   `building_domain::reader::BuildingReader` 구현체 (V-World 필지 폴리곤 합성)
//!
//! V-World 패턴 답습: ACL parser → 도메인, `RawCapture` 로 raw_response 보존.

#![forbid(unsafe_code)]
// data_go_kr / data.go.kr 등 외부 식별자 표기 패턴 false-positive 차단.
#![allow(clippy::doc_markdown)]

pub mod building_register;
pub mod client;
pub mod error;
pub mod pnu_split;

pub use client::{DataGoKrClient, DataGoKrConfig};
pub use error::{ConfigError, ParseError};
pub use pnu_split::{split, PnuParts};
