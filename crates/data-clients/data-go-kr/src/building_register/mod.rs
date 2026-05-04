//! 건축물대장 표제부 통합 — `getBrTitleInfo`.
//!
//! - [`client::BuildingRegisterClient`] — HTTP 호출 (`getBrTitleInfo`)
//! - [`parser::parse_building_title`] — JSON → `Vec<Building>` ACL
//! - [`reader::DataGoKrBuildingReader`] — `BuildingReader` impl (V-World geom 합성)

pub mod client;
pub mod parser;
pub mod reader;

pub use client::BuildingRegisterClient;
pub use reader::DataGoKrBuildingReader;
