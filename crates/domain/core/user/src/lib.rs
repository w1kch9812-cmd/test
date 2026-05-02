//! `User` Aggregate (Core BC, RDS 동적).
//!
//! Plan 2b-i 범위 — spec § 5.1 18 필드 전체 + 도메인 mutation 메서드
//! (`verify_business`, `verify_broker`, `add_role`, `soft_delete` 등).
//! `UserRepository` trait는 4개 query 메서드 (`find_by_id`,
//! `find_by_zitadel_sub`, `find_by_email`, `save`)를 노출해요.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
