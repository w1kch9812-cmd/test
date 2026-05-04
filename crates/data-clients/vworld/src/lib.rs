//! 공짱 V-World 외부 API 클라이언트.
//!
//! `crates/circuit-breaker` 위에 V-World 단일 API 통합:
//! - [`VWorldClient`] — `reqwest::Client` 래퍼 + `Breaker` + `Policy`
//! - [`VWorldParcelReader`] — `parcel_domain::reader::ParcelReader` 구현체
//! - [`parser::parse_parcel`] — V-World JSON → 도메인 [`Parcel`] 변환 (ACL)
//! - [`RawCapture`] trait — raw_response 보존 hook (`NoOpRawCapture` 가 default)
//!
//! 본 crate 가 SP4-iii (data.go.kr / 법제처 / R2 Reader 6) 의 패턴 baseline.
//!
//! [`Parcel`]: parcel_domain::entity::Parcel

#![forbid(unsafe_code)]
// raw_response, V-World 등 lowercase + 외부 식별자 표기 패턴 false-positive 차단.
#![allow(clippy::doc_markdown)]

pub mod client;
pub mod error;
pub mod parser;
pub mod raw_capture;
pub mod reader;

pub use client::{VWorldClient, VWorldConfig};
pub use error::{ConfigError, ParseError, RawCaptureError};
pub use raw_capture::{NoOpRawCapture, RawCapture};
pub use reader::VWorldParcelReader;
