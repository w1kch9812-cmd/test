//! `User` Aggregate (Core BC, RDS 동적).
//!
//! Walking Skeleton 범위 — `try_new` + 기본 필드만. 비즈니스 검증·중개사·roles
//! 등은 Plan 2b-i 본격 구현에서 추가해요.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
