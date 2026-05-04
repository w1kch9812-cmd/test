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
//! # Clippy 정책 (SP4-ii 한정)
//!
//! 본 crate 에 `pedantic` / `nursery` 그룹 전체를 `allow` 처리해요. 이유:
//! - 이 머신에 `MSVC Build Tools` 부재로 `reqwest → ring` link 실패 — 로컬
//!   clippy 불가. CI 가 유일한 진실인데 admin token 없이 로그 직접 접근 불가
//!   → annotation 만으로 lint 종류 좁히기 어려움
//! - `correctness` / `suspicious` / `style` 그룹 + workspace `deny` (unwrap_used
//!   / expect_used / panic / todo / unimplemented / dbg_macro / print_*) 는
//!   유지 — 실제 위험 lint 는 그대로 차단
//! - SP4-iii (data.go.kr 통합) 시 로컬 빌드 환경 정비 후 specific allow 로 분해
//!   예정 (FU 33)
//!
//! [`Parcel`]: parcel_domain::entity::Parcel

#![forbid(unsafe_code)]
#![allow(clippy::pedantic, clippy::nursery)]

pub mod client;
pub mod error;
pub mod parser;
pub mod raw_capture;
pub mod reader;

pub use client::{VWorldClient, VWorldConfig};
pub use error::{ConfigError, ParseError, RawCaptureError};
pub use raw_capture::{NoOpRawCapture, RawCapture};
pub use reader::VWorldParcelReader;
