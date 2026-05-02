//! `SQLx` `Postgres` `Repository` 구현체.
//!
//! 도메인 BC가 정의한 `*Repository` trait의 구현. Walking Skeleton 범위에서는
//! `PgUserRepository`만 — sub-project 5에서 `Listing`/`ListingPhoto` 등 추가해요.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod user;
