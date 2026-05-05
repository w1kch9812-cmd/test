//! `SQLx` `Postgres` `Repository` 구현체.
//!
//! 도메인 BC 가 정의한 `*Repository` trait 의 구현. `error_map` 모듈이 공통
//! `sqlx::Error` 매핑을 제공해요.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin_action;
pub mod analysis_report;
pub mod api_health;
pub mod audit_log;
pub mod bookmark;
pub mod bvq;
pub mod error_map;
pub mod listing;
pub mod listing_photo;
pub mod listing_report;
pub mod lrq;
pub mod notification;
pub mod operations_meta;
pub mod outbox;
pub mod pipeline;
pub mod raw_capture;
pub mod search_history;
pub mod user;
