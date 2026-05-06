//! 공짱 R2 (Cloudflare R2, S3-호환) PMTiles + JSON 인덱스 reader.
//!
//! 6 R2 Reader trait 중 SP4-iii-e 1차 = 3 (Parcel bbox markers / Building
//! footprint / IndustrialComplex). Manufacturer / RealTransaction /
//! CourtAuction 은 SP4-iii-e-2 (FU 61).
//!
//! V-World/data.go.kr 패턴 답습:
//! - [`R2Client`] — `reqwest::Client` 위 GET (R2 public-read 가정)
//! - [`Policy::r2_default`] — 8s timeout / retry 1 / 60s cooldown
//! - [`pmtiles::PmtilesReader`] — v3 spec 직접 구현 (header + directory + tile_at)
//! - 6 R2 Reader 구현체 (Parcel/Building/IC 1차, 나머지 후속)
//!
//! `aws-sdk-s3` SigV4 client 도입은 FU 67 (private bucket / pre-signed URL).
//! 1차는 public-read R2 bucket 가정 — `R2_PUBLIC_URL_BASE` env 가 base URL.

#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
// FU 26 — legitimate HTTP client wrapper (R2 외부 객체 통합, Breaker 경유).
#![allow(clippy::disallowed_types)]

pub mod client;
pub mod error;

pub use client::{R2Client, R2Config};
pub use error::{ConfigError, ParseError};
