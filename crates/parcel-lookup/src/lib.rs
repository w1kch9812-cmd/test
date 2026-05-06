//! PNU 기반 필지 정보 lookup port + V-World 구현체.
//!
//! ADR 0018 (PNU-First identity) 의 직접 결과 — 매물 등록/갱신 시 PNU 로
//! 행정구역/지목/용도지역을 조회해서 listing 테이블에 denormalize 함.
//!
//! 모듈:
//! - [`info`] — 좁은 결과 struct ([`ParcelInfo`])
//! - [`lookup`] — port trait ([`ParcelInfoLookup`]) + 에러
//! - [`vworld_lookup`] — V-World `LP_PA_CBND_BUBUN` 어댑터
//!
//! 향후 추가:
//! - Redis 캐시 wrapper (1주 TTL) — V-World quota 부담 시
//! - Bronze SHP R-tree 백엔드 — V-World 의존성 0 옵션

#![forbid(unsafe_code)]

pub mod info;
pub mod lookup;
pub mod noop_lookup;
pub mod vworld_lookup;

pub use info::ParcelInfo;
pub use lookup::{LookupError, ParcelInfoLookup};
pub use noop_lookup::NoOpParcelInfoLookup;
pub use vworld_lookup::VWorldParcelInfoLookup;
