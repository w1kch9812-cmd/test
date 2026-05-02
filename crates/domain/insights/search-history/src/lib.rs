//! `SearchHistory` 도메인 (Insights BC, RDS 동적).
//!
//! 사용자 검색 이력. append-mostly (매 검색마다 1 row).
//! `PIPA` 가명화 — 90일 후 `user_id` → `NULL`. 1년 후 삭제.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
