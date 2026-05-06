//! 공짱 V-World 외부 API 클라이언트.
//!
//! 모듈 분리 (각 책임 단일):
//! - [`client`] — `reqwest` + Circuit Breaker (`Policy` + `Breaker`)
//! - [`envelope`] — V-World response 외피 (status/error/features 추출)
//! - [`geometry`] — V-World JSON geometry → [`MultiPolygonSrid`] 변환
//! - [`layers`] — 레이어별 properties → 도메인 entity 변환 (ACL)
//!   - [`layers::parcel_boundary`] — `LP_PA_CBND_BUBUN` (연속지적도)
//! - [`reader`] — [`VWorldParcelReader`] (`ParcelReader` 구현체, layer 합성)
//!
//! 본 crate 가 SP4-iii data.go.kr / 법제처 / R2 Reader 의 패턴 baseline.
//!
//! [`MultiPolygonSrid`]: shared_kernel::geometry::MultiPolygonSrid

#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
// FU 26 — legitimate HTTP client wrapper (V-World 외부 API 통합, Breaker 경유).
#![allow(clippy::disallowed_types)]

pub mod client;
pub mod envelope;
pub mod error;
pub mod geometry;
pub mod layers;
pub mod reader;

pub use client::{VWorldClient, VWorldConfig};
pub use error::{ConfigError, ParseError};
// raw_capture는 SP4-iii-d 에서 별도 crate 로 추출. 호환성 위해 re-export.
pub use raw_capture_client::{NoOpRawCapture, RawCapture, RawCaptureError};
pub use reader::VWorldParcelReader;
